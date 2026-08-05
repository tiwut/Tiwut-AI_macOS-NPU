use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const PAD_TOKEN_ID: usize = 0;
pub const BOS_TOKEN_ID: usize = 1;
pub const EOS_TOKEN_ID: usize = 2;
pub const UNK_TOKEN_ID: usize = 3;
pub const MASK_TOKEN_ID: usize = 4;
pub const SEP_TOKEN_ID: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokenizer {
    pub max_vocab_size: usize,
    pub token_to_id: HashMap<String, usize>,
    pub id_to_token: Vec<String>,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new(4096)
    }
}

impl Tokenizer {
    pub fn new(max_vocab_size: usize) -> Self {
        let mut tok = Self {
            max_vocab_size,
            token_to_id: HashMap::new(),
            id_to_token: Vec::new(),
        };
        tok.init_base_vocab();
        tok
    }

    fn init_base_vocab(&mut self) {
        self.token_to_id.clear();
        self.id_to_token.clear();

        let specials = ["<pad>", "<s>", "</s>", "<unk>", "<mask>", "<sep>"];
        for s in specials {
            let id = self.id_to_token.len();
            self.token_to_id.insert(s.to_string(), id);
            self.id_to_token.push(s.to_string());
        }

        for b in 0..=255u8 {
            let s = format!("<byte_{:02x}>", b);
            let id = self.id_to_token.len();
            self.token_to_id.insert(s.clone(), id);
            self.id_to_token.push(s);
        }

        let english_base = [
            "the", "be", "to", "of", "and", "a", "in", "that", "have", "i",
            "it", "for", "not", "on", "with", "he", "as", "you", "do", "at",
            "this", "but", "his", "by", "from", "they", "we", "say", "her", "she",
            "or", "an", "will", "my", "one", "all", "would", "there", "their", "what",
            "so", "up", "out", "if", "about", "who", "get", "which", "go", "me",
            "when", "make", "can", "like", "time", "no", "just", "him", "know", "take",
            "people", "into", "year", "your", "good", "some", "could", "them", "see", "other",
            "than", "then", "now", "look", "only", "come", "its", "over", "think", "also",
            "back", "after", "use", "two", "how", "our", "work", "first", "well", "way",
            "even", "new", "want", "because", "any", "these", "give", "day", "most", "us",
            "is", "are", "was", "were", "been", "has", "had", "does", "did", "am",
            "AI", "Tiwut", "Tiwut-AI", "Apple", "Silicon", "Rust", "Neural", "Network",
            "model", "memory", "RAM", "CPU", "GPU", "NPU", "data", "system", "file",
            "learning", "training", "train", "chat", "ask", "answer", "query", "server",
            "API", "GUI", "Transformer", "attention", "embedding", "vector", "layer",
            "hello", "hi", "hey", "help", "status", "version", "engine", "code", "text",
            "User", "Assistant", "Question", "Answer:", "Yes", "No", "Please", "Thank"
        ];

        for word in english_base {
            if !self.token_to_id.contains_key(word) {
                let id = self.id_to_token.len();
                self.token_to_id.insert(word.to_string(), id);
                self.id_to_token.push(word.to_string());
            }
        }
    }

    pub fn vocab_size(&self) -> usize {
        self.id_to_token.len()
    }

    pub fn train_on_text(&mut self, text: &str, max_new_tokens: usize) -> usize {
        if text.trim().is_empty() {
            return 0;
        }

        let mut word_freq: HashMap<String, usize> = HashMap::new();
        for word in text.split_whitespace() {
            let clean: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if clean.len() >= 2 && clean.len() <= 64 {
                *word_freq.entry(clean).or_insert(0) += 1;
            }
        }

        let mut sorted_words: Vec<(String, usize)> = word_freq.into_iter().collect();
        sorted_words.sort_by(|a, b| b.1.cmp(&a.1));

        let mut added = 0;
        for (w, _freq) in sorted_words {
            if !self.token_to_id.contains_key(&w) {
                if self.id_to_token.len() >= self.max_vocab_size {
                    break;
                }
                let id = self.id_to_token.len();
                self.token_to_id.insert(w.clone(), id);
                self.id_to_token.push(w);
                added += 1;
                if added >= max_new_tokens {
                    break;
                }
            }
        }

        added
    }

    pub fn encode(&self, text: &str, add_special_tokens: bool) -> Vec<usize> {
        let mut tokens = Vec::new();
        if add_special_tokens {
            tokens.push(BOS_TOKEN_ID);
        }

        let words: Vec<&str> = text.split(' ').collect();
        let total_words = words.len();

        for (i, word) in words.into_iter().enumerate() {
            if word.is_empty() {
                if i + 1 < total_words {
                    tokens.push(self.byte_token_id(b' '));
                }
                continue;
            }

            if let Some(&id) = self.token_to_id.get(word) {
                tokens.push(id);
            } else {

                for &b in word.as_bytes() {
                    tokens.push(self.byte_token_id(b));
                }
            }

            if i + 1 < total_words {
                tokens.push(self.byte_token_id(b' '));
            }
        }

        if add_special_tokens {
            tokens.push(EOS_TOKEN_ID);
        }

        tokens
    }

    fn byte_token_id(&self, b: u8) -> usize {
        let key = format!("<byte_{:02x}>", b);
        *self.token_to_id.get(&key).unwrap_or(&UNK_TOKEN_ID)
    }

    pub fn decode(&self, token_ids: &[usize], skip_special_tokens: bool) -> String {
        let mut byte_buffer: Vec<u8> = Vec::new();
        let mut result = String::new();

        let special_ids = [
            PAD_TOKEN_ID,
            BOS_TOKEN_ID,
            EOS_TOKEN_ID,
            UNK_TOKEN_ID,
            MASK_TOKEN_ID,
            SEP_TOKEN_ID,
        ];

        for &id in token_ids {
            if skip_special_tokens && special_ids.contains(&id) {
                continue;
            }

            if id >= self.id_to_token.len() {
                continue;
            }

            let token = &self.id_to_token[id];

            if token.starts_with("<byte_") && token.ends_with('>') && token.len() == 9 {
                if let Ok(b) = u8::from_str_radix(&token[6..8], 16) {
                    byte_buffer.push(b);
                    continue;
                }
            }

            if !byte_buffer.is_empty() {
                result.push_str(&String::from_utf8_lossy(&byte_buffer));
                byte_buffer.clear();
            }

            result.push_str(token);
        }

        if !byte_buffer.is_empty() {
            result.push_str(&String::from_utf8_lossy(&byte_buffer));
        }

        result
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

