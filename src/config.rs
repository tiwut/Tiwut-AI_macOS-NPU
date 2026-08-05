use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub model: ModelConfig,
    pub training: TrainConfig,
    pub inference: InferenceConfig,
    pub api: ApiConfig,
    pub hardware: HardwareConfig,
    pub storage: PathConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub vocab_size: usize,
    pub embed_dim: usize,
    pub num_layers: usize,
    pub num_heads: usize,
    pub feedforward_dim: usize,
    pub max_seq_len: usize,
    pub dropout: f32,
    pub tie_weights: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainConfig {
    pub batch_size: usize,
    pub learning_rate: f32,
    pub default_epochs: usize,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub weight_decay: f32,
    pub warmup_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub temperature: f32,
    pub top_k: usize,
    pub top_p: f32,
    pub repetition_penalty: f32,
    pub max_tokens: usize,
    pub memory_threshold: f32,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    pub num_threads: usize,
    pub use_accelerate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub model_file: String,
    pub default_knowledge_file: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            vocab_size: 2048,
            embed_dim: 256,
            num_layers: 6,
            num_heads: 8,
            feedforward_dim: 1024,
            max_seq_len: 512,
            dropout: 0.1,
            tie_weights: true,
        }
    }
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            batch_size: 8,
            learning_rate: 0.0004,
            default_epochs: 10,
            chunk_size: 256,
            chunk_overlap: 64,
            weight_decay: 0.01,
            warmup_ratio: 0.1,
        }
    }
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            temperature: 0.6,
            top_k: 40,
            top_p: 0.9,
            repetition_penalty: 1.15,
            max_tokens: 250,
            memory_threshold: 0.25,
            stream: true,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            enable_cors: true,
        }
    }
}

impl Default for HardwareConfig {
    fn default() -> Self {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            num_threads: threads,
            use_accelerate: cfg!(target_os = "macos"),
        }
    }
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            model_file: "ai.model".to_string(),
            default_knowledge_file: "default_knowledge.txt".to_string(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig::default(),
            training: TrainConfig::default(),
            inference: InferenceConfig::default(),
            api: ApiConfig::default(),
            hardware: HardwareConfig::default(),
            storage: PathConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        if path.as_ref().exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, json)
    }
}

