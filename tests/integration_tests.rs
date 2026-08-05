use tiwut_ai_v2::config::{AppConfig, ModelConfig};
use tiwut_ai_v2::memory::MemoryBank;
use tiwut_ai_v2::model::TiwutModel;
use tiwut_ai_v2::package::ModelPackage;
use tiwut_ai_v2::tensor::Tensor2D;
use tiwut_ai_v2::tokenizer::Tokenizer;

#[test]
fn test_tokenizer_encoding_decoding() {
    let mut tok = Tokenizer::default();
    let text = "Hello world! Tiwut-AI is fast on Apple Silicon.";
    let tokens = tok.encode(text, true);
    assert!(!tokens.is_empty());
    let decoded = tok.decode(&tokens, true);
    assert!(decoded.contains("Hello world"));
    assert!(decoded.contains("Tiwut-AI"));

    let added = tok.train_on_text("supercalifragilisticexpialidocious supercalifragilisticexpialidocious", 10);
    assert!(added >= 1);
}

#[test]
fn test_tensor_matmul_and_rope() {
    let a = Tensor2D::new(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let b = Tensor2D::new(3, 2, vec![7.0, 8.0, 9.0, 1.0, 2.0, 3.0]);
    let c = a.matmul(&b);
    assert_eq!(c.rows, 2);
    assert_eq!(c.cols, 2);
    assert_eq!(c.get(0, 0), 1.0 * 7.0 + 2.0 * 9.0 + 3.0 * 2.0);

    let mut q = Tensor2D::randn(4, 16, 0.1);
    q.apply_rope(4, 10000.0);
    assert_eq!(q.rows, 4);
    assert_eq!(q.cols, 16);
}

#[test]
fn test_model_forward_and_semantic_vectors() {
    let mut config = ModelConfig::default();
    config.vocab_size = 300;
    config.embed_dim = 64;
    config.num_layers = 2;
    config.num_heads = 4;
    config.feedforward_dim = 128;

    let model = TiwutModel::new(config);
    let token_ids = vec![1, 10, 25, 45, 2];
    let (logits, hidden) = model.forward(&token_ids);

    assert_eq!(logits.rows, 5);
    assert_eq!(logits.cols, 300);
    assert_eq!(hidden.rows, 5);
    assert_eq!(hidden.cols, 64);

    let vector = model.encode_semantic_vector(&token_ids);
    assert_eq!(vector.len(), 64);
}

#[test]
fn test_package_roundtrip() {
    let temp_dir = std::env::temp_dir();
    let model_file = temp_dir.join("test_tiwut_package.model");

    let config = AppConfig::default();
    let tok = Tokenizer::default();
    let model = TiwutModel::new(config.model.clone());
    let mut memory = MemoryBank::new();

    memory.add_chunks(
        "test_source",
        "Test Title",
        vec![("Test content about Rust".to_string(), vec![1, 2, 3], vec![0.1; config.model.embed_dim])],
    );

    let save_res = ModelPackage::save_to_file(&model_file, &config, &model, &tok, &memory);
    assert!(save_res.is_ok());

    let load_res = ModelPackage::load_from_file(&model_file);
    assert!(load_res.is_ok());

    let (loaded_cfg, loaded_model, loaded_tok, loaded_mem) = load_res.unwrap();
    assert_eq!(loaded_cfg.model.embed_dim, config.model.embed_dim);
    assert_eq!(loaded_tok.vocab_size(), tok.vocab_size());
    assert_eq!(loaded_model.total_parameters(), model.total_parameters());
    assert_eq!(loaded_mem.chunks.len(), 1);

    let _ = std::fs::remove_file(model_file);
}

