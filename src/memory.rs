use crate::tensor::Tensor2D;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeChunk {
    pub id: usize,
    pub source: String,
    pub title: String,
    pub text: String,
    pub tokens: Vec<usize>,
    pub embedding: Vec<f32>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMetadata {
    pub total_documents: usize,
    pub total_chunks: usize,
    pub total_tokens: usize,
    pub sources: Vec<String>,
    pub last_updated: i64,
}

impl Default for KnowledgeMetadata {
    fn default() -> Self {
        Self {
            total_documents: 0,
            total_chunks: 0,
            total_tokens: 0,
            sources: Vec::new(),
            last_updated: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: KnowledgeChunk,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBank {
    pub chunks: Vec<KnowledgeChunk>,
    pub metadata: KnowledgeMetadata,
}

impl Default for MemoryBank {
    fn default() -> Self {
        Self {
            chunks: Vec::new(),
            metadata: KnowledgeMetadata::default(),
        }
    }
}

impl MemoryBank {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_chunks(&mut self, source: &str, title: &str, new_chunks: Vec<(String, Vec<usize>, Vec<f32>)>) {
        if new_chunks.is_empty() {
            return;
        }

        let now = chrono::Utc::now().timestamp();
        let mut source_tokens = 0;

        for (text, tokens, embedding) in new_chunks {
            let id = self.chunks.len();
            source_tokens += tokens.len();
            self.chunks.push(KnowledgeChunk {
                id,
                source: source.to_string(),
                title: title.to_string(),
                text,
                tokens,
                embedding,
                created_at: now,
            });
        }

        if !self.metadata.sources.iter().any(|s| s == source) {
            self.metadata.sources.push(source.to_string());
            self.metadata.total_documents += 1;
        }

        self.metadata.total_chunks = self.chunks.len();
        self.metadata.total_tokens += source_tokens;
        self.metadata.last_updated = now;
    }

    pub fn search(&self, query_vec: &[f32], query_text: &str, top_k: usize, threshold: f32) -> Vec<SearchResult> {
        if self.chunks.is_empty() {
            return Vec::new();
        }

        let stopwords = ["what", "is", "a", "an", "the", "how", "do", "does", "are", "and", "in", "to", "for", "of", "with", "about", "tell", "me", "can", "you", "explain"];
        let query_lower = query_text.to_lowercase();
        let query_words: Vec<String> = query_lower
            .split_whitespace()
            .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
            .filter(|w| w.len() >= 2 && !stopwords.contains(&w.as_str()))
            .collect();

        let mut scored: Vec<SearchResult> = self
            .chunks
            .iter()
            .map(|chunk| {
                let vec_score = Tensor2D::cosine_similarity(query_vec, &chunk.embedding);

                let text_lower = chunk.text.to_lowercase();
                let chunk_words: HashSet<String> = text_lower
                    .split_whitespace()
                    .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
                    .filter(|w| !w.is_empty())
                    .collect();

                let mut keyword_hits = 0;
                for word in &query_words {
                    if chunk_words.contains(word) {
                        keyword_hits += 1;
                    }
                }

                let exact_q_hit = text_lower.lines().any(|l| {
                    let l_trim = l.trim();
                    l_trim.starts_with("question:") && !query_words.is_empty() && query_words.iter().all(|w| l_trim.to_lowercase().contains(w.as_str()))
                });

                let keyword_boost = if exact_q_hit {
                    0.85
                } else if !query_words.is_empty() {
                    (keyword_hits as f32) / (query_words.len() as f32) * 0.70
                } else {
                    0.0
                };

                let combined_score = vec_score * 0.35 + keyword_boost;
                SearchResult {
                    chunk: chunk.clone(),
                    score: combined_score,
                }
            })
            .filter(|res| res.score >= threshold)
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    pub fn extract_intelligent_answer(&self, query: &str, results: &[SearchResult]) -> Option<String> {
        if results.is_empty() {
            return None;
        }

        let stopwords = ["what", "is", "a", "an", "the", "how", "do", "does", "are", "and", "in", "to", "for", "of", "with", "about", "tell", "me", "can", "you", "explain"];
        let query_lower = query.to_lowercase();
        let query_words: HashSet<String> = query_lower
            .split_whitespace()
            .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
            .filter(|s: &String| s.len() >= 2 && !stopwords.contains(&s.as_str()))
            .collect();

        let query_clean = query_lower.trim_end_matches('?').trim();
        for res in results {
            let clean_text = &res.chunk.text;
            for line in clean_text.lines() {
                let line_trim = line.trim();
                if line_trim.to_lowercase().starts_with("question:") {
                    let q_part = line_trim[9..].to_lowercase();
                    let q_part_clean = q_part.trim_end_matches('?').trim();
                    let matching_words = query_words.iter().filter(|w| q_part.contains(w.as_str())).count();
                    let is_match = q_part_clean == query_clean
                        || (!query_words.is_empty()
                            && matching_words >= query_words.len());

                    if is_match {
                        if let Some(pos) = clean_text.find(line_trim) {
                            let after = &clean_text[pos + line_trim.len()..];
                            for next_line in after.lines() {
                                let next_trim = next_line.trim();
                                if next_trim.to_lowercase().starts_with("answer:") {
                                    return Some(next_trim[7..].trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut candidate_sentences: Vec<(&str, usize)> = Vec::new();
        for res in results {
            for line in res.chunk.text.lines() {
                let mut line_trim = line.trim();
                if line_trim.starts_with('#') || line_trim.starts_with("Question:") {
                    continue;
                }
                if line_trim.to_lowercase().starts_with("answer:") {
                    line_trim = line_trim[7..].trim();
                }
                for raw_s in line_trim.split(". ") {
                    let s_clean = raw_s.trim().trim_end_matches('.');
                    if s_clean.len() >= 20 {
                        let first_char = s_clean.chars().next().unwrap_or(' ');
                        if !first_char.is_uppercase() {
                            continue;
                        }
                        let s_lower = s_clean.to_lowercase();
                        let mut hits = query_words.iter().filter(|w| s_lower.contains(w.as_str())).count();
                        for w in &query_words {
                            if s_lower.starts_with(w.as_str())
                                || s_lower.starts_with(&format!("a {}", w))
                                || s_lower.starts_with(&format!("an {}", w))
                                || s_lower.starts_with(&format!("the {}", w))
                            {
                                hits += 5;
                            }
                        }
                        if hits >= 1 {
                            candidate_sentences.push((s_clean, hits));
                        }
                    }
                }
            }
        }

        candidate_sentences.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((best_sentence, hits)) = candidate_sentences.first() {
            if *hits >= 1 {
                return Some(format!("{}.", best_sentence));
            }
        }

        if let Some(top_result) = results.first() {
            if top_result.score >= 0.35 {
                let mut preview = top_result.chunk.text.clone();
                if preview.len() > 400 {
                    preview.truncate(400);
                    preview.push_str("...");
                }
                return Some(preview);
            }
        }

        None
    }

    pub fn memory_usage_mb(&self) -> f32 {
        let mut bytes = 0;
        for c in &self.chunks {
            bytes += c.text.len();
            bytes += c.tokens.len() * std::mem::size_of::<usize>();
            bytes += c.embedding.len() * std::mem::size_of::<f32>();
        }
        (bytes as f32) / (1024.0 * 1024.0)
    }

    pub fn reset(&mut self) {
        self.chunks.clear();
        self.metadata = KnowledgeMetadata::default();
    }
}

