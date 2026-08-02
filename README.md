# Tiwut-AI: macOS M4 Neural Network AI (CLI)

A lightweight, high-performance, CLI-first Neural Network AI engine written in Python, tailored specifically for **Apple Silicon (M4)** devices with **Metal Performance Shaders (MPS / NPU)** hardware acceleration, **High-Speed SQLite WAL Database (Memory-Mapped I/O)**, and **RAM-resident Unified Memory neural state**.

---

## Key Features

- **Apple Silicon M4 Native Acceleration**: Leverages PyTorch Metal Performance Shaders (`mps`) and Unified Memory for ultra-fast matrix calculations.
- **High-Speed Database Engine**: SQLite in WAL mode with 256MB memory-mapped I/O (`mmap`), 64MB RAM page cache, and zero-copy binary vector serialization.
- **In-RAM Pre-Loaded Neural Memory**: Pre-loads neural network weights and semantic embeddings directly into active RAM for instant, sub-millisecond query responses.
- **Centralized `config.json`**: Configure database location, model hyperparameters, training settings, and hardware options in one central JSON file.
- **Neural Network Architecture**:
  - RoPE (Rotary Positional Embeddings)
  - Pre-RMSNorm Multi-Head Attention
  - SwiGLU Feed-Forward Neural Memory Layers
  - Dense Neural Semantic Latent Encoder for associative retrieval
- **Ingestion & Dynamic Training**:
  - **Websites**: Scrapes and cleans live URLs with boilerplate filtering.
  - **Text Documents**: Reads `.txt`, `.md`, `.csv`, `.log` files or entire directories.
  - **Dynamic Vocabulary Expansion**: Auto-expands Byte-level tokenizer and resizes neural embeddings without forgetting.
- **Interactive Streaming Chat**: Real-time token-by-token generation with colored CLI interface.
- **Single-Dash & Double-Dash Arguments**: Supports `-help`, `-config`, `-train`, `-chat`, `-ask`, `-status`, `-reset`, etc.

---

## Configuration (`config.json`)

All database paths, storage locations, and hyperparameters are configurable via `config.json`:

```json
{
  "database": {
    "type": "sqlite_wal",
    "path": "storage/neural_brain.db",
    "in_memory_cache": true,
    "mmap_size_mb": 256,
    "wal_autocheckpoint": 1000
  },
  "storage": {
    "base_dir": "storage",
    "checkpoint_file": "storage/neural_weights.pt",
    "tokenizer_file": "storage/tokenizer.json",
    "meta_file": "storage/metadata.json"
  },
  "hardware": {
    "device": "auto",
    "enable_mps_fallback": true
  },
  "model": {
    "vocab_size": 4096,
    "embed_dim": 256,
    "num_layers": 6,
    "num_heads": 8,
    "feedforward_dim": 1024,
    "max_seq_len": 512,
    "dropout": 0.1,
    "tie_weights": true
  },
  "training": {
    "batch_size": 16,
    "learning_rate": 0.0003,
    "default_epochs": 10,
    "chunk_size": 256,
    "chunk_overlap": 64
  },
  "inference": {
    "temperature": 0.6,
    "top_k": 40,
    "top_p": 0.9,
    "max_tokens": 200,
    "stream": true
  }
}
```

View active settings at any time with:
```bash
./tiwut-ai -config
```

---

## Installation

```bash
# 1. Clone or navigate to the repository
cd Tiwut-AI

# 2. Setup environment (automatic with ./tiwut-ai or manually)
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

---

## CLI Usage & Options

You can run commands using `./tiwut-ai` or `python3 main.py`:

| Option / Flag | Description |
|---|---|
| `-help`, `--help` | Display formatted help manual with examples |
| `-status`, `--status` | Show Apple Silicon chip info, database metrics, and RAM usage |
| `-config`, `--config` | View active JSON configuration (`config.json`) |
| `-train`, `--train` | Train/fine-tune neural network on documents or websites |
| `-url <URL>` | Train on content scraped from a website URL |
| `-file <path>` | Train on a local `.txt` or `.md` file |
| `-dir <path>` | Recursively train on a directory of text documents |
| `-epochs <n>` | Number of training epochs (default from `config.json`) |
| `-lr <float>` | Learning rate (default from `config.json`) |
| `-chat`, `--chat` | Start interactive real-time streaming chat CLI |
| `-ask "<question>"` | Ask a single question and get an instant neural response |
| `-reset`, `--reset` | Clear neural weights, database, and RAM knowledge base |

---

## Quick Examples

### 1. View Configuration & Status
```bash
./tiwut-ai -config
./tiwut-ai -status
```

### 2. Train on a Website URL
```bash
./tiwut-ai -train -url https://en.wikipedia.org/wiki/Apple_silicon -epochs 10
```

### 3. Train on a Text Document
```bash
./tiwut-ai -train -file sample_knowledge.txt -epochs 10
```

### 4. Train on an Entire Folder of Text Files
```bash
./tiwut-ai -train -dir ./documents -epochs 8
```

### 5. Interactive Chat Mode
```bash
./tiwut-ai -chat
```
Inside chat mode:
- `/help` — view chat commands
- `/status` — view Apple M4 hardware & memory status
- `/memory` — list learned sources in RAM
- `/clear` — reset conversation history
- `/exit` — quit chat

### 6. One-Shot Question Query
```bash
./tiwut-ai -ask "What features does Apple Silicon M4 have?"
```

---

## Documentation

- [Training Guide](file:///Users/tiwut/Documents/dev/Tiwut-AI/TRAINING.md) — Comprehensive training, data ingestion, pipeline stages, and hyperparameter tuning.
- [Configuration & Database Guide](file:///Users/tiwut/Documents/dev/Tiwut-AI/CONFIG_GUIDE.md) — Detailed reference for `config.json`, database switching, and memory options.
