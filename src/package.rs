use crate::config::AppConfig;
use crate::memory::MemoryBank;
use crate::model::TiwutModel;
use crate::tokenizer::Tokenizer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackageMetadata {
    pub package_version: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub total_parameters: usize,
    pub total_chunks: usize,
    pub total_tokens: usize,
    pub vocab_size: usize,
    pub engine: String,
}

pub struct ModelPackage;

impl ModelPackage {
    pub const CURRENT_VERSION: &'static str = "2.0.0";

    pub fn exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }

    pub fn save_to_file<P: AsRef<Path>>(
        path: P,
        config: &AppConfig,
        model: &TiwutModel,
        tokenizer: &Tokenizer,
        memory: &MemoryBank,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();
        let temp_path = path.with_extension("model.tmp");

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let file = File::create(&temp_path)?;
        let mut zip = ZipWriter::new(BufWriter::new(file));
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        zip.start_file("config.json", options)?;
        let config_json = serde_json::to_string_pretty(config)?;
        zip.write_all(config_json.as_bytes())?;

        zip.start_file("tokenizer.json", options)?;
        let tok_json = serde_json::to_string(tokenizer)?;
        zip.write_all(tok_json.as_bytes())?;

        zip.start_file("weights.json", options)?;
        let weights_json = serde_json::to_string(model)?;
        zip.write_all(weights_json.as_bytes())?;

        zip.start_file("knowledge.json", options)?;
        let memory_json = serde_json::to_string(memory)?;
        zip.write_all(memory_json.as_bytes())?;

        let meta = ModelPackageMetadata {
            package_version: Self::CURRENT_VERSION.to_string(),
            created_at: memory.metadata.last_updated,
            updated_at: chrono::Utc::now().timestamp(),
            total_parameters: model.total_parameters(),
            total_chunks: memory.chunks.len(),
            total_tokens: memory.metadata.total_tokens,
            vocab_size: tokenizer.vocab_size(),
            engine: "Tiwut-AI v2 (Rust Core)".to_string(),
        };
        zip.start_file("metadata.json", options)?;
        let meta_json = serde_json::to_string_pretty(&meta)?;
        zip.write_all(meta_json.as_bytes())?;

        zip.finish()?;

        std::fs::rename(&temp_path, path)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<(AppConfig, TiwutModel, Tokenizer, MemoryBank), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mut zip = ZipArchive::new(BufReader::new(file))?;

        let config_str = {
            let mut config_file = zip.by_name("config.json")?;
            let mut s = String::new();
            config_file.read_to_string(&mut s)?;
            s
        };
        let config: AppConfig = serde_json::from_str(&config_str)?;

        let tok_str = {
            let mut tok_file = zip.by_name("tokenizer.json")?;
            let mut s = String::new();
            tok_file.read_to_string(&mut s)?;
            s
        };
        let tokenizer: Tokenizer = serde_json::from_str(&tok_str)?;

        let weights_str = {
            let mut weights_file = zip.by_name("weights.json")?;
            let mut s = String::new();
            weights_file.read_to_string(&mut s)?;
            s
        };
        let model: TiwutModel = serde_json::from_str(&weights_str)?;

        let memory_str = {
            let mut memory_file = zip.by_name("knowledge.json")?;
            let mut s = String::new();
            memory_file.read_to_string(&mut s)?;
            s
        };
        let memory: MemoryBank = serde_json::from_str(&memory_str)?;

        Ok((config, model, tokenizer, memory))
    }
}

