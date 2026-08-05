use crate::config::AppConfig;
use crate::dataset::{DocumentReader, RawDocument, TextChunker, WebScraper};
use crate::memory::MemoryBank;
use crate::model::TiwutModel;
use crate::package::ModelPackage;
use crate::tensor::Tensor2D;
use crate::tokenizer::Tokenizer;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainProgressEvent {
    pub stage: String,
    pub epoch: usize,
    pub total_epochs: usize,
    pub loss: f32,
    pub message: String,
    pub done: bool,
}

pub struct NeuralTrainer;

impl NeuralTrainer {
    pub fn train_sources<F>(
        config: &mut AppConfig,
        model: &mut TiwutModel,
        tokenizer: &mut Tokenizer,
        memory: &mut MemoryBank,
        urls: Option<&[String]>,
        files: Option<&[PathBuf]>,
        directories: Option<&[PathBuf]>,
        raw_texts: Option<&[String]>,
        include_default_knowledge: bool,
        epochs_override: Option<usize>,
        lr_override: Option<f32>,
        model_file_path: &str,
        mut progress_cb: F,
    ) -> Result<TrainProgressEvent, Box<dyn std::error::Error>>
    where
        F: FnMut(TrainProgressEvent),
    {
        let t0 = Instant::now();
        let epochs = epochs_override.unwrap_or(config.training.default_epochs);
        let lr = lr_override.unwrap_or(config.training.learning_rate);

        progress_cb(TrainProgressEvent {
            stage: "ingestion".to_string(),
            epoch: 0,
            total_epochs: epochs,
            loss: 0.0,
            message: "📥 Ingesting and sanitizing data sources...".to_string(),
            done: false,
        });

        let mut raw_docs: Vec<RawDocument> = Vec::new();

        if include_default_knowledge {
            let default_text = include_str!("../default_knowledge.txt");
            raw_docs.push(RawDocument {
                source: "builtin://default_english_knowledge".to_string(),
                title: "Builtin English Knowledge & Dialogue Base".to_string(),
                content: default_text.to_string(),
            });
        }

        if let Some(url_list) = urls {
            for url in url_list {
                if let Ok(doc) = WebScraper::scrape_url(url) {
                    if !doc.content.is_empty() {
                        raw_docs.push(doc);
                    }
                }
            }
        }

        if let Some(file_list) = files {
            for f in file_list {
                if let Ok(doc) = DocumentReader::read_file(f) {
                    if !doc.content.is_empty() {
                        raw_docs.push(doc);
                    }
                }
            }
        }

        if let Some(dir_list) = directories {
            for d in dir_list {
                let docs = DocumentReader::read_directory(d, true);
                raw_docs.extend(docs);
            }
        }

        if let Some(texts) = raw_texts {
            for (i, t) in texts.iter().enumerate() {
                if !t.trim().is_empty() {
                    raw_docs.push(RawDocument {
                        source: format!("input://custom_text_{}", i + 1),
                        title: format!("Custom Input {}", i + 1),
                        content: t.clone(),
                    });
                }
            }
        }

        if raw_docs.is_empty() {
            let err = TrainProgressEvent {
                stage: "error".to_string(),
                epoch: 0,
                total_epochs: epochs,
                loss: 0.0,
                message: "No valid documents or data sources provided to train on.".to_string(),
                done: true,
            };
            progress_cb(err.clone());
            return Ok(err);
        }

        let combined_text: String = raw_docs.iter().map(|d| d.content.as_str()).collect::<Vec<_>>().join("\n\n");

        progress_cb(TrainProgressEvent {
            stage: "vocab_expansion".to_string(),
            epoch: 0,
            total_epochs: epochs,
            loss: 0.0,
            message: "🔡 Expanding dynamic vocabulary...".to_string(),
            done: false,
        });

        let old_vocab = tokenizer.vocab_size();
        let _added_tokens = tokenizer.train_on_text(&combined_text, 512);
        let new_vocab = tokenizer.vocab_size();

        if new_vocab > old_vocab {
            model.resize_vocab(new_vocab);
            config.model.vocab_size = new_vocab;
        }

        progress_cb(TrainProgressEvent {
            stage: "chunking".to_string(),
            epoch: 0,
            total_epochs: epochs,
            loss: 0.0,
            message: format!("✂️ Segmenting text into semantic neural chunks (vocab: {})...", new_vocab),
            done: false,
        });

        let chunk_size = config.training.chunk_size;
        let overlap = config.training.chunk_overlap;
        let mut all_chunk_tuples: Vec<(String, String, String, Vec<usize>)> = Vec::new();

        for doc in &raw_docs {
            let chunks = TextChunker::chunk_text(&doc.content, tokenizer, chunk_size, overlap);
            for (c_text, c_tokens) in chunks {
                if c_tokens.len() >= 4 {
                    all_chunk_tuples.push((doc.source.clone(), doc.title.clone(), c_text, c_tokens));
                }
            }
        }

        if all_chunk_tuples.is_empty() {
            let err = TrainProgressEvent {
                stage: "error".to_string(),
                epoch: 0,
                total_epochs: epochs,
                loss: 0.0,
                message: "Generated chunk set is empty.".to_string(),
                done: true,
            };
            progress_cb(err.clone());
            return Ok(err);
        }

        let mut m_tok = Tensor2D::zeros(model.tok_embeddings.rows, model.tok_embeddings.cols);
        let mut v_tok = Tensor2D::zeros(model.tok_embeddings.rows, model.tok_embeddings.cols);
        let mut step = 0;

        let mut final_loss = 0.0;

        for epoch in 1..=epochs {
            let mut epoch_loss = 0.0;
            let mut batch_count = 0;

            let progress = (epoch as f32) / (epochs as f32);
            let current_lr = lr * 0.5 * (1.0 + (progress * std::f32::consts::PI).cos()).max(0.1);

            for (_, _, _, tokens) in &all_chunk_tuples {
                if tokens.len() < 2 {
                    continue;
                }

                step += 1;
                let (logits, _) = model.forward(tokens);

                let seq_len = logits.rows;
                let vocab = logits.cols;
                let mut loss_sum = 0.0;
                let mut loss_count = 0;

                let mut grad_embed = Tensor2D::zeros(model.tok_embeddings.rows, model.tok_embeddings.cols);

                for i in 0..(seq_len - 1) {
                    let target_id = tokens[i + 1];
                    let logit_row = logits.row(i);

                    let mut max_val = f32::NEG_INFINITY;
                    for &l in logit_row {
                        if l > max_val {
                            max_val = l;
                        }
                    }
                    let mut sum_exp = 0.0;
                    for &l in logit_row {
                        sum_exp += (l - max_val).exp();
                    }

                    let target_logit = if target_id < vocab { logit_row[target_id] } else { 0.0 };
                    let prob = ((target_logit - max_val).exp() / sum_exp.max(1e-8)).max(1e-8);
                    loss_sum += -prob.ln();
                    loss_count += 1;

                    let cur_tok = tokens[i];
                    if cur_tok < grad_embed.rows {
                        let d = grad_embed.cols;
                        for j in 0..d {
                            let g = (prob - 1.0) * 0.01;
                            grad_embed.set(cur_tok, j, grad_embed.get(cur_tok, j) + g);
                        }
                    }
                }

                if loss_count > 0 {
                    epoch_loss += loss_sum / (loss_count as f32);
                    batch_count += 1;
                }

                Tensor2D::adamw_update(
                    &mut model.tok_embeddings,
                    &grad_embed,
                    &mut m_tok,
                    &mut v_tok,
                    step,
                    current_lr,
                    0.9,
                    0.999,
                    1e-8,
                    config.training.weight_decay,
                );
            }

            final_loss = if batch_count > 0 { epoch_loss / (batch_count as f32) } else { 0.0 };

            progress_cb(TrainProgressEvent {
                stage: "training".to_string(),
                epoch,
                total_epochs: epochs,
                loss: final_loss,
                message: format!("Epoch [{}/{}] • Loss: {:.4} • LR: {:.6}", epoch, epochs, final_loss, current_lr),
                done: false,
            });
        }

        progress_cb(TrainProgressEvent {
            stage: "indexing".to_string(),
            epoch: epochs,
            total_epochs: epochs,
            loss: final_loss,
            message: "🧠 Projecting semantic latent vectors and updating in-RAM memory bank...".to_string(),
            done: false,
        });

        let mut grouped: std::collections::HashMap<(String, String), Vec<(String, Vec<usize>, Vec<f32>)>> =
            std::collections::HashMap::new();

        for (src, title, text, tokens) in all_chunk_tuples {
            let embedding = model.encode_semantic_vector(&tokens);
            grouped
                .entry((src, title))
                .or_default()
                .push((text, tokens, embedding));
        }

        for ((src, title), chunks) in grouped {
            memory.add_chunks(&src, &title, chunks);
        }

        progress_cb(TrainProgressEvent {
            stage: "saving".to_string(),
            epoch: epochs,
            total_epochs: epochs,
            loss: final_loss,
            message: format!("💾 Bundling all weights, config, and memory into '{}'...", model_file_path),
            done: false,
        });

        ModelPackage::save_to_file(model_file_path, config, model, tokenizer, memory)?;

        let elapsed = t0.elapsed().as_secs_f32();
        let finish_event = TrainProgressEvent {
            stage: "complete".to_string(),
            epoch: epochs,
            total_epochs: epochs,
            loss: final_loss,
            message: format!(
                "✅ Training complete in {:.2}s! Added {} chunks across {} sources into '{}'.",
                elapsed,
                memory.chunks.len(),
                memory.metadata.sources.len(),
                model_file_path
            ),
            done: true,
        };

        progress_cb(finish_event.clone());
        Ok(finish_event)
    }
}

