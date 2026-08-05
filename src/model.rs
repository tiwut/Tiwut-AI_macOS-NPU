use crate::config::ModelConfig;
use crate::tensor::Tensor2D;
use crate::tokenizer::EOS_TOKEN_ID;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformerBlock {
    pub attn_norm: Tensor2D,
    pub q_proj: Tensor2D,
    pub k_proj: Tensor2D,
    pub v_proj: Tensor2D,
    pub out_proj: Tensor2D,
    pub ffn_norm: Tensor2D,
    pub w1: Tensor2D,
    pub w2: Tensor2D,
    pub w3: Tensor2D,
}

impl TransformerBlock {
    pub fn new(config: &ModelConfig) -> Self {
        let d = config.embed_dim;
        let ffn = config.feedforward_dim;
        let std_dev = (2.0 / (d as f32)).sqrt() * 0.1;

        Self {
            attn_norm: Tensor2D::ones(1, d),
            q_proj: Tensor2D::randn(d, d, std_dev),
            k_proj: Tensor2D::randn(d, d, std_dev),
            v_proj: Tensor2D::randn(d, d, std_dev),
            out_proj: Tensor2D::randn(d, d, std_dev),
            ffn_norm: Tensor2D::ones(1, d),
            w1: Tensor2D::randn(d, ffn, std_dev),
            w2: Tensor2D::randn(ffn, d, std_dev),
            w3: Tensor2D::randn(d, ffn, std_dev),
        }
    }

    pub fn forward(&self, x: &Tensor2D, config: &ModelConfig) -> Tensor2D {
        let seq_len = x.rows;
        let d = config.embed_dim;
        let num_heads = config.num_heads;
        let head_dim = d / num_heads;

        let normed_attn = x.rms_norm(&self.attn_norm, 1e-6);

        let mut q = normed_attn.matmul(&self.q_proj);
        let mut k = normed_attn.matmul(&self.k_proj);
        let v = normed_attn.matmul(&self.v_proj);

        q.apply_rope(head_dim, 10000.0);
        k.apply_rope(head_dim, 10000.0);

        let scale = 1.0 / (head_dim as f32).sqrt();
        let mut attn_out = vec![0.0; seq_len * d];

        for h in 0..num_heads {
            let h_offset = h * head_dim;

            let mut q_head = Tensor2D::zeros(seq_len, head_dim);
            let mut k_head = Tensor2D::zeros(seq_len, head_dim);
            let mut v_head = Tensor2D::zeros(seq_len, head_dim);

            for i in 0..seq_len {
                for j in 0..head_dim {
                    q_head.set(i, j, q.get(i, h_offset + j));
                    k_head.set(i, j, k.get(i, h_offset + j));
                    v_head.set(i, j, v.get(i, h_offset + j));
                }
            }

            let mut scores = q_head.matmul_transposed_b(&k_head);
            scores.scale(scale);

            scores.softmax_rowwise(true);

            let head_out = scores.matmul(&v_head);

            for i in 0..seq_len {
                for j in 0..head_dim {
                    attn_out[i * d + h_offset + j] = head_out.get(i, j);
                }
            }
        }

        let attn_tensor = Tensor2D::new(seq_len, d, attn_out);
        let attn_proj = attn_tensor.matmul(&self.out_proj);

        let h = x.add(&attn_proj);

        let normed_ffn = h.rms_norm(&self.ffn_norm, 1e-6);
        let ffn_out = Tensor2D::swiglu_forward(&normed_ffn, &self.w1, &self.w2, &self.w3);

        h.add(&ffn_out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiwutModel {
    pub config: ModelConfig,
    pub tok_embeddings: Tensor2D,
    pub layers: Vec<TransformerBlock>,
    pub final_norm: Tensor2D,
    pub lm_head: Tensor2D,
    pub semantic_proj1: Tensor2D,
    pub semantic_proj2: Tensor2D,
}

impl TiwutModel {
    pub fn new(config: ModelConfig) -> Self {
        let vocab_size = config.vocab_size;
        let d = config.embed_dim;
        let std_dev = (2.0 / (d as f32)).sqrt() * 0.1;

        let tok_embeddings = Tensor2D::randn(vocab_size, d, std_dev);
        let mut layers = Vec::with_capacity(config.num_layers);
        for _ in 0..config.num_layers {
            layers.push(TransformerBlock::new(&config));
        }

        let final_norm = Tensor2D::ones(1, d);
        let lm_head = if config.tie_weights {
            Tensor2D::zeros(d, vocab_size)
        } else {
            Tensor2D::randn(d, vocab_size, std_dev)
        };

        let semantic_proj1 = Tensor2D::randn(d, d, std_dev);
        let semantic_proj2 = Tensor2D::randn(d, d, std_dev);

        Self {
            config,
            tok_embeddings,
            layers,
            final_norm,
            lm_head,
            semantic_proj1,
            semantic_proj2,
        }
    }

    pub fn total_parameters(&self) -> usize {
        let mut total = self.tok_embeddings.data.len();
        for l in &self.layers {
            total += l.attn_norm.data.len();
            total += l.q_proj.data.len();
            total += l.k_proj.data.len();
            total += l.v_proj.data.len();
            total += l.out_proj.data.len();
            total += l.ffn_norm.data.len();
            total += l.w1.data.len();
            total += l.w2.data.len();
            total += l.w3.data.len();
        }
        total += self.final_norm.data.len();
        if !self.config.tie_weights {
            total += self.lm_head.data.len();
        }
        total += self.semantic_proj1.data.len();
        total += self.semantic_proj2.data.len();
        total
    }

    pub fn resize_vocab(&mut self, new_vocab_size: usize) {
        if new_vocab_size <= self.config.vocab_size {
            return;
        }

        let old_vocab = self.config.vocab_size;
        let d = self.config.embed_dim;
        let std_dev = (2.0 / (d as f32)).sqrt() * 0.1;

        let mut new_embed_data = vec![0.0; new_vocab_size * d];
        new_embed_data[..old_vocab * d].copy_from_slice(&self.tok_embeddings.data);

        let new_part = Tensor2D::randn(new_vocab_size - old_vocab, d, std_dev);
        new_embed_data[old_vocab * d..].copy_from_slice(&new_part.data);

        self.tok_embeddings = Tensor2D::new(new_vocab_size, d, new_embed_data);

        if !self.config.tie_weights {
            let mut new_lm_data = vec![0.0; d * new_vocab_size];
            for r in 0..d {
                for c in 0..old_vocab {
                    new_lm_data[r * new_vocab_size + c] = self.lm_head.get(r, c);
                }
                for c in old_vocab..new_vocab_size {
                    new_lm_data[r * new_vocab_size + c] = rand::thread_rng().gen_range(-0.02..0.02);
                }
            }
            self.lm_head = Tensor2D::new(d, new_vocab_size, new_lm_data);
        }

        self.config.vocab_size = new_vocab_size;
    }

    pub fn forward(&self, token_ids: &[usize]) -> (Tensor2D, Tensor2D) {
        let seq_len = token_ids.len().min(self.config.max_seq_len).max(1);
        let d = self.config.embed_dim;

        let mut x_data = vec![0.0; seq_len * d];
        for (i, &tid) in token_ids.iter().take(seq_len).enumerate() {
            let safe_id = if tid < self.tok_embeddings.rows { tid } else { 0 };
            let embed_row = self.tok_embeddings.row(safe_id);
            x_data[i * d..(i + 1) * d].copy_from_slice(embed_row);
        }

        let mut x = Tensor2D::new(seq_len, d, x_data);

        for layer in &self.layers {
            x = layer.forward(&x, &self.config);
        }

        let hidden_states = x.rms_norm(&self.final_norm, 1e-6);

        let logits = if self.config.tie_weights {

            hidden_states.matmul_transposed_b(&self.tok_embeddings)
        } else {
            hidden_states.matmul(&self.lm_head)
        };

        (logits, hidden_states)
    }

    pub fn encode_semantic_vector(&self, token_ids: &[usize]) -> Vec<f32> {
        if token_ids.is_empty() {
            return vec![0.0; self.config.embed_dim];
        }

        let (_, hidden_states) = self.forward(token_ids);
        let seq_len = hidden_states.rows;
        let d = hidden_states.cols;

        let mut mean_pooled = vec![0.0; d];
        for i in 0..seq_len {
            let row = hidden_states.row(i);
            for j in 0..d {
                mean_pooled[j] += row[j];
            }
        }
        let inv_len = 1.0 / (seq_len as f32);
        for v in &mut mean_pooled {
            *v *= inv_len;
        }

        let mean_tensor = Tensor2D::new(1, d, mean_pooled);
        let mut p1 = mean_tensor.matmul(&self.semantic_proj1);

        for v in &mut p1.data {
            *v = *v / (1.0 + (-*v).exp());
        }
        let mut latent = p1.matmul(&self.semantic_proj2);
        latent.normalize_rows();

        latent.data
    }

    pub fn generate_next_token(
        &self,
        token_ids: &[usize],
        temperature: f32,
        top_k: usize,
        top_p: f32,
        repetition_penalty: f32,
    ) -> usize {
        let (logits, _) = self.forward(token_ids);
        let last_row_idx = logits.rows - 1;
        let mut next_logits = logits.row(last_row_idx).to_vec();
        let vocab_size = next_logits.len();

        if (repetition_penalty - 1.0).abs() > 1e-4 {
            let seen_tokens: std::collections::HashSet<usize> = token_ids.iter().copied().collect();
            for &tok in &seen_tokens {
                if tok < vocab_size {
                    if next_logits[tok] > 0.0 {
                        next_logits[tok] /= repetition_penalty;
                    } else {
                        next_logits[tok] *= repetition_penalty;
                    }
                }
            }
        }

        if temperature <= 0.01 {
            let mut best_id = 0;
            let mut best_val = f32::NEG_INFINITY;
            for (id, &val) in next_logits.iter().enumerate() {
                if val > best_val {
                    best_val = val;
                    best_id = id;
                }
            }
            return best_id;
        }

        for v in &mut next_logits {
            *v /= temperature;
        }

        let mut indexed_logits: Vec<(usize, f32)> = next_logits.into_iter().enumerate().collect();
        indexed_logits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if top_k > 0 && top_k < indexed_logits.len() {
            indexed_logits.truncate(top_k);
        }

        let max_val = indexed_logits.first().map(|x| x.1).unwrap_or(0.0);
        let mut sum_exp = 0.0;
        for (_, val) in &mut indexed_logits {
            *val = (*val - max_val).exp();
            sum_exp += *val;
        }
        for (_, val) in &mut indexed_logits {
            *val /= sum_exp;
        }

        if top_p < 1.0 {
            let mut cumulative = 0.0;
            let mut cutoff = indexed_logits.len();
            for (idx, (_, prob)) in indexed_logits.iter().enumerate() {
                cumulative += prob;
                if cumulative >= top_p {
                    cutoff = idx + 1;
                    break;
                }
            }
            indexed_logits.truncate(cutoff);

            let new_sum: f32 = indexed_logits.iter().map(|x| x.1).sum();
            if new_sum > 0.0 {
                for (_, prob) in &mut indexed_logits {
                    *prob /= new_sum;
                }
            }
        }

        let mut rng = rand::thread_rng();
        let r: f32 = rng.gen_range(0.0..1.0);
        let mut acc = 0.0;
        for (token_id, prob) in indexed_logits {
            acc += prob;
            if r <= acc {
                return token_id;
            }
        }

        EOS_TOKEN_ID
    }
}

