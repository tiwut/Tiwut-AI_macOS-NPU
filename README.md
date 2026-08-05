# Tiwut-AI Version 2 (Rust Neural Engine & Native Qt6 GUI)

A high-performance, cross-platform neural network AI engine built from the ground up in **Rust**, with a completely decoupled **Native Qt6 C++ Desktop Application**.

---

## 🌟 Key Features & Improvements over v1

1. **Native Rust Engine & Rayon SIMD Multi-Threading**:
   - Sub-millisecond tensor operations, matrix multiplication, RoPE attention, SwiGLU activations, and RMSNorm parallelized across all CPU cores.
   - Cross-platform native binary compilation with zero Python runtime dependencies.

2. **Single-File Unified Container (`ai.model`)**:
   - All trained neural weights, token embeddings, dynamic vocabulary, configuration, and in-RAM knowledge chunks are bundled into a single compressed `ai.model` package.
   - Atomically updated whenever new data is trained or ingested.

3. **Built-in Default English Knowledge Base**:
   - Pre-loaded with extensive English grammar structures, technical computing definitions, dialogues, and system context so the AI is immediately capable upon startup.

4. **100% Decoupled Architecture**:
   - **Headless AI Core & API**: Pure REST and SSE streaming API server (`http://127.0.0.1:8080`).
   - **Native Qt6 Desktop GUI**: High-performance C++ desktop client residing in `version_2/gui/` communicating exclusively via network API.

---

## 🚀 Quick Start Guide

### 1. Build the Rust AI Engine
```bash
cd version_2
cargo build --release
cargo test
```

### 2. Interactive CLI Neural Chat
```bash
./target/release/tiwut-ai chat
```
*Special commands inside chat*: `/status`, `/memory`, `/clear`, `/help`, `/exit`.

### 3. Ask a Single Question (Instant CLI Output)
```bash
./target/release/tiwut-ai ask "What is Apple Silicon?"
```

### 4. Neural Training & Knowledge Ingestion
You can train the AI on any URL, file, directory, or raw text:

- **Train on a website / article**:
  ```bash
  ./target/release/tiwut-ai train --url "https://en.wikipedia.org/wiki/Rust_(programming_language)" --epochs 10
  ```

- **Train on a local file or notes**:
  ```bash
  ./target/release/tiwut-ai train --file ./my_notes.txt --epochs 8
  ```

- **Train on an entire directory of documents**:
  ```bash
  ./target/release/tiwut-ai train --dir ./documentation/ --epochs 12
  ```

- **Re-initialize with default English knowledge base**:
  ```bash
  ./target/release/tiwut-ai init-default
  ```

### 5. Launch the Headless API Server
```bash
./target/release/tiwut-ai serve --port 8080 --host 127.0.0.1
```

### 6. Launch the Native Qt6 Desktop Application
```bash
cd gui
mkdir -p build && cd build
cmake ..
make -j$(sysctl -n hw.ncpu)
./tiwut-ai-gui
```

---

## 📡 REST & Streaming API Reference

| Endpoint | Method | Description |
|---|---|---|
| `/api/health` | `GET` | Health check probe (returns `OK`) |
| `/api/status` | `GET` | Hardware specs, CPU cores, active parameters, memory bank stats |
| `/api/chat` | `POST` | Standard JSON chat completion |
| `/api/chat/stream` | `POST` | Real-time Server-Sent Events (SSE) token-by-token streaming |
| `/api/ask` | `POST` | Single question/answer retrieval |
| `/api/train` | `POST` | Trigger training on URLs, files, or text |
| `/api/memory` | `GET` | List all indexed memory chunks and learned sources |
| `/api/config` | `GET`/`POST` | Retrieve or update runtime hyperparameters |
| `/api/reset` | `POST` | Reset model weights and knowledge bank |

---

## 🏛️ System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│              Tiwut-AI v2 Native Qt6 GUI (Desktop App)           │
│   (Chat Studio • Training Studio • Memory Bank • Telemetry)     │
└────────────────────────────────┬────────────────────────────────┘
                                 │ HTTP / REST & SSE (Port 8080)
┌────────────────────────────────▼────────────────────────────────┐
│             Tiwut-AI v2 Async Axum / Tokio API Server           │
│     (/api/chat/stream, /api/train, /api/memory, /api/status)    │
└────────────────────────────────┬────────────────────────────────┘
                                 │
┌────────────────────────────────▼────────────────────────────────┐
│                Tiwut-AI Neural Core Engine (Rust)               │
│   - RoPE Multi-Head Attention (6 Layers, 8 Heads)               │
│   - SwiGLU Gated FeedForward (1024 Dim)                         │
│   - RMSNorm Pre-Layer Normalization                             │
│   - In-RAM Semantic Vector Memory Bank (RAG)                    │
│   - Rayon Multi-Core SIMD Engine (Apple Silicon / x86_64)       │
└────────────────────────────────┬────────────────────────────────┘
                                 │
┌────────────────────────────────▼────────────────────────────────┐
│                       ai.model Package                          │
│   (Weights + Config + Tokenizer + Knowledge Memory Chunks)      │
└─────────────────────────────────────────────────────────────────┘
```
