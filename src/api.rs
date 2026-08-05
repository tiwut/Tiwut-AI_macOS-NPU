use crate::chat::ChatEngine;
use crate::config::AppConfig;
use crate::hardware::{HardwareInfo, HardwareManager};
use crate::memory::MemoryBank;
use crate::model::TiwutModel;
use crate::package::ModelPackage;
use crate::tokenizer::Tokenizer;
use crate::trainer::NeuralTrainer;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::cors::{Any, CorsLayer};

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub model: Mutex<TiwutModel>,
    pub tokenizer: Mutex<Tokenizer>,
    pub memory: Mutex<MemoryBank>,
    pub model_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub duration_ms: u128,
}

#[derive(Debug, Deserialize)]
pub struct AskRequest {
    pub question: String,
}

#[derive(Debug, Serialize)]
pub struct AskResponse {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Deserialize)]
pub struct TrainRequest {
    pub urls: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    pub text: Option<String>,
    pub epochs: Option<usize>,
    pub learning_rate: Option<f32>,
    pub include_default_knowledge: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MemoryResponse {
    pub total_chunks: usize,
    pub total_tokens: usize,
    pub total_documents: usize,
    pub sources: Vec<String>,
    pub ram_usage_mb: f32,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub hardware: HardwareInfo,
    pub total_parameters: usize,
    pub vocab_size: usize,
    pub memory_chunks: usize,
    pub memory_ram_mb: f32,
    pub model_path: String,
    pub engine: String,
}

pub struct ApiServer;

impl ApiServer {
    pub async fn start(state: Arc<AppState>, host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/", get(|| async {
                Json(serde_json::json!({
                    "engine": "Tiwut-AI v2",
                    "version": "2.0.0",
                    "status": "online",
                    "endpoints": [
                        "/api/health",
                        "/api/status",
                        "/api/chat",
                        "/api/chat/stream",
                        "/api/ask",
                        "/api/train",
                        "/api/memory",
                        "/api/config",
                        "/api/reset"
                    ]
                }))
            }))
            .route("/api/health", get(|| async { "OK" }))
            .route("/api/status", get(get_status))
            .route("/api/chat", post(chat_handler))
            .route("/api/chat/stream", post(chat_stream_handler))
            .route("/api/ask", post(ask_handler))
            .route("/api/train", post(train_handler))
            .route("/api/memory", get(get_memory))
            .route("/api/config", get(get_config).post(update_config))
            .route("/api/reset", post(reset_handler))
            .with_state(state.clone())
            .layer(cors);

        let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
        println!("🚀 Tiwut-AI v2 Backend API Server running at http://{}", addr);
        println!("📡 REST API & SSE Streaming ready for client connections");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let hw = HardwareManager::get_info();
    let model = state.model.lock().unwrap();
    let tok = state.tokenizer.lock().unwrap();
    let mem = state.memory.lock().unwrap();

    Json(StatusResponse {
        hardware: hw,
        total_parameters: model.total_parameters(),
        vocab_size: tok.vocab_size(),
        memory_chunks: mem.chunks.len(),
        memory_ram_mb: mem.memory_usage_mb(),
        model_path: state.model_path.clone(),
        engine: "Tiwut-AI v2 (Rust Core)".to_string(),
    })
}

async fn ask_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AskRequest>,
) -> Json<AskResponse> {
    let config = state.config.lock().unwrap().clone();
    let model = state.model.lock().unwrap().clone();
    let tok = state.tokenizer.lock().unwrap().clone();
    let mem = state.memory.lock().unwrap().clone();

    let answer = ChatEngine::answer_query(&payload.question, &config, &model, &tok, &mem, false, None);

    Json(AskResponse {
        question: payload.question,
        answer,
    })
}

async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatRequest>,
) -> Json<ChatResponse> {
    let t0 = std::time::Instant::now();
    let config = state.config.lock().unwrap().clone();
    let model = state.model.lock().unwrap().clone();
    let tok = state.tokenizer.lock().unwrap().clone();
    let mem = state.memory.lock().unwrap().clone();

    let response = ChatEngine::answer_query(&payload.message, &config, &model, &tok, &mem, false, None);

    Json(ChatResponse {
        response,
        duration_ms: t0.elapsed().as_millis(),
    })
}

async fn chat_stream_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::unbounded_channel();

    let config = state.config.lock().unwrap().clone();
    let model = state.model.lock().unwrap().clone();
    let tok = state.tokenizer.lock().unwrap().clone();
    let mem = state.memory.lock().unwrap().clone();

    tokio::task::spawn_blocking(move || {
        let mut callback = |token: &str| {
            let event = Event::default().data(token.to_string());
            let _ = tx.send(Ok(event));
        };

        ChatEngine::answer_query(
            &payload.message,
            &config,
            &model,
            &tok,
            &mem,
            true,
            Some(&mut callback),
        );

        let end_event = Event::default().event("done").data("[DONE]");
        let _ = tx.send(Ok(end_event));
    });

    let stream = UnboundedReceiverStream::new(rx);
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn train_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TrainRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::unbounded_channel();

    let state_clone = state.clone();

    tokio::task::spawn_blocking(move || {
        let mut config = state_clone.config.lock().unwrap();
        let mut model = state_clone.model.lock().unwrap();
        let mut tok = state_clone.tokenizer.lock().unwrap();
        let mut mem = state_clone.memory.lock().unwrap();

        let files: Option<Vec<PathBuf>> = payload.files.map(|f| f.into_iter().map(PathBuf::from).collect());
        let raw_texts: Option<Vec<String>> = payload.text.map(|t| vec![t]);
        let inc_default = payload.include_default_knowledge.unwrap_or(false);

        let tx_sender = tx.clone();
        let callback = move |evt: crate::trainer::TrainProgressEvent| {
            if let Ok(json_str) = serde_json::to_string(&evt) {
                let event = Event::default().event("progress").data(json_str);
                let _ = tx_sender.send(Ok(event));
            }
        };

        let result = NeuralTrainer::train_sources(
            &mut config,
            &mut model,
            &mut tok,
            &mut mem,
            payload.urls.as_deref(),
            files.as_deref(),
            None,
            raw_texts.as_deref(),
            inc_default,
            payload.epochs,
            payload.learning_rate,
            &state_clone.model_path,
            callback,
        );

        if let Ok(finish_evt) = result {
            if let Ok(json) = serde_json::to_string(&finish_evt) {
                let _ = tx.send(Ok(Event::default().event("complete").data(json)));
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx);
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn get_memory(State(state): State<Arc<AppState>>) -> Json<MemoryResponse> {
    let mem = state.memory.lock().unwrap();
    Json(MemoryResponse {
        total_chunks: mem.chunks.len(),
        total_tokens: mem.metadata.total_tokens,
        total_documents: mem.metadata.total_documents,
        sources: mem.metadata.sources.clone(),
        ram_usage_mb: mem.memory_usage_mb(),
        last_updated: mem.metadata.last_updated,
    })
}

async fn get_config(State(state): State<Arc<AppState>>) -> Json<AppConfig> {
    let config = state.config.lock().unwrap().clone();
    Json(config)
}

async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(new_config): Json<AppConfig>,
) -> Json<AppConfig> {
    let mut config = state.config.lock().unwrap();
    *config = new_config.clone();

    let model = state.model.lock().unwrap();
    let tok = state.tokenizer.lock().unwrap();
    let mem = state.memory.lock().unwrap();
    let _ = ModelPackage::save_to_file(&state.model_path, &config, &model, &tok, &mem);

    Json(new_config)
}

async fn reset_handler(State(state): State<Arc<AppState>>) -> StatusCode {
    let mut config = state.config.lock().unwrap();
    let mut model = state.model.lock().unwrap();
    let mut tok = state.tokenizer.lock().unwrap();
    let mut mem = state.memory.lock().unwrap();

    *config = AppConfig::default();
    *tok = Tokenizer::default();
    *model = TiwutModel::new(config.model.clone());
    mem.reset();

    let _ = ModelPackage::save_to_file(&state.model_path, &config, &model, &tok, &mem);
    StatusCode::OK
}

