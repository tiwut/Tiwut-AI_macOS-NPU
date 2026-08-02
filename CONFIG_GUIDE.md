# ⚙️ Tiwut-AI: Configuration, Training & Database Guide

Comprehensive guide for configuring **Tiwut-AI**, managing the **High-Speed Database Engine**, switching database backends, and fine-tuning neural network training parameters.

---

## 📑 Table of Contents

1. [Configuration Overview (`config.json`)](#-configuration-overview-configjson)
2. [Database Switching & Configuration](#-database-switching--configuration)
   - [File-Backed SQLite with WAL & MMAP (Default)](#1-file-backed-sqlite-with-wal--mmap-default)
   - [Pure In-Memory Mode (`:memory:`)](#2-pure-in-memory-mode-memory)
   - [Custom Storage Directory](#3-custom-storage-directory)
3. [Training Configuration & Best Practices](#-training-configuration--best-practices)
   - [Training Sources (URLs, Files, Directories)](#training-sources)
   - [Hyperparameter Tuning](#hyperparameter-tuning)
   - [Dynamic Tokenizer & Embedding Expansion](#dynamic-tokenizer--embedding-expansion)
4. [Inference & Reasoning Settings](#-inference--reasoning-settings)
5. [macOS M4 Hardware Optimization](#-macos-m4-hardware-optimization)

---

## 🛠️ Configuration Overview (`config.json`)

The entire system is controlled via `config.json` in the root directory. You can inspect active settings anytime with:

```bash
./tiwut-ai -config
```

### Complete Schema:

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
    "min_learning_rate": 0.00003,
    "weight_decay": 0.01,
    "grad_clip": 1.0,
    "default_epochs": 10,
    "chunk_size": 256,
    "chunk_overlap": 64
  },
  "inference": {
    "temperature": 0.6,
    "top_k": 40,
    "top_p": 0.9,
    "repetition_penalty": 1.15,
    "max_tokens": 200,
    "stream": true,
    "memory_threshold": 0.1
  }
}
```

---

## 🗄️ Database Switching & Configuration

Tiwut-AI uses a high-performance database layer designed for zero-latency retrieval into Apple Silicon Unified RAM.

### 1. File-Backed SQLite with WAL & MMAP (Default)

Persists learned knowledge across sessions with sub-millisecond retrieval speeds.

```json
"database": {
  "type": "sqlite_wal",
  "path": "storage/neural_brain.db",
  "in_memory_cache": true,
  "mmap_size_mb": 256,
  "wal_autocheckpoint": 1000
}
```

- **`type`**: `"sqlite_wal"` — Enables SQLite Write-Ahead Logging for concurrent non-blocking reads and writes.
- **`path`**: File location for the SQLite database.
- **`mmap_size_mb`**: Amount of memory-mapped I/O (default: `256` MB). Allows the OS kernel to map the database file directly into RAM, bypassing disk buffers.
- **`wal_autocheckpoint`**: Automatically commits the write-ahead log to disk every `1000` pages.

### 2. Pure In-Memory Mode (`:memory:`)

For ephemeral sessions or maximum speed without writing database files to disk:

```json
"database": {
  "type": "sqlite_memory",
  "path": ":memory:",
  "in_memory_cache": true,
  "mmap_size_mb": 0,
  "wal_autocheckpoint": 1000
}
```

- In this mode, no `.db` files are created on disk.
- All documents, chunks, and embeddings exist exclusively in RAM.

### 3. Custom Storage Directory

To organize checkpoints or databases in another folder (e.g., external drive or project folder):

```json
"storage": {
  "base_dir": "my_custom_storage",
  "checkpoint_file": "my_custom_storage/model.pt",
  "tokenizer_file": "my_custom_storage/tokenizer.json",
  "meta_file": "my_custom_storage/metadata.json"
},
"database": {
  "path": "my_custom_storage/brain.db"
}
```

---

## 🏋️ Training Configuration & Best Practices

### Training Sources

You can train Tiwut-AI on web pages, local documents, or entire directories:

```bash
# 1. Scrape & train on a website URL
./tiwut-ai -train -url https://example.com/article -epochs 10

# 2. Train on a local document
./tiwut-ai -train -file ./notes.txt -epochs 15 -lr 0.0002

# 3. Recursively train on a directory of documents (.txt, .md, .csv, .py, etc.)
./tiwut-ai -train -dir ./knowledge_folder -epochs 8
```

### Hyperparameter Tuning

| Setting | Type | Recommended | Description |
|---|---|---|---|
| `batch_size` | `int` | `8` – `32` | Number of token sequences processed concurrently on M4 MPS. |
| `learning_rate` | `float` | `0.0001` – `0.0005` | Base AdamW learning rate. |
| `min_learning_rate` | `float` | `0.00001` – `0.00005` | Minimum learning rate for Cosine Annealing scheduler. |
| `weight_decay` | `float` | `0.01` | L2 regularization to prevent overfitting on small texts. |
| `grad_clip` | `float` | `1.0` | Gradient clipping threshold to prevent exploding gradients. |
| `chunk_size` | `int` | `128` – `512` | Sequence length for semantic chunking. |
| `chunk_overlap` | `int` | `32` – `128` | Token overlap between adjacent chunks to maintain context. |

### Dynamic Tokenizer & Embedding Expansion

When you train on new documents or websites:
1. The **Byte-Level Tokenizer** analyzes word frequencies and introduces new subword tokens.
2. The **TiwutNeuralAI** model dynamically resizes its embedding and projection matrices on the M4 GPU without forgetting prior weights.
3. Chunks are chunked, tokenized, and embedded into the **High-Speed Database** and pinned in **RAM**.

---

## 🧠 Inference & Reasoning Settings

Configure the generation behavior in `config.json`:

```json
"inference": {
  "temperature": 0.6,
  "top_k": 40,
  "top_p": 0.9,
  "repetition_penalty": 1.15,
  "max_tokens": 200,
  "stream": true,
  "memory_threshold": 0.1
}
```

- **`temperature`**: Controls randomness. `0.2` for precise factual answers, `0.7` for creative responses.
- **`top_k`**: Limits sampling to top `k` probable tokens (default: `40`).
- **`top_p`**: Nucleus sampling cutoff cumulative probability (default: `0.9`).
- **`repetition_penalty`**: Penalizes recently generated tokens (default: `1.15`) to eliminate repetition loops.
- **`memory_threshold`**: Minimum hybrid cosine/lexical similarity score to trigger memory retrieval.

---

## ⚡ macOS M4 Hardware Optimization

Tiwut-AI is engineered for Apple Silicon (M4 / M3 / M2 / M1):
- **MPS Matrix Acceleration**: Neural forward passes and backward gradients run on Metal Performance Shaders.
- **Unified RAM Residency**: The entire embedding matrix and chunk index stay resident in Unified Memory for sub-millisecond retrieval.
- **SQLite Memory-Mapped I/O**: `mmap_size_mb` enables OS-level zero-copy reads directly into memory.

Check your hardware acceleration status anytime:
```bash
./tiwut-ai -status
```
