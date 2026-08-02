# Tiwut-AI Training Guide

Complete documentation on training, fine-tuning, data ingestion, and optimization for **Tiwut-AI** on **macOS Apple Silicon (M4 / MPS)**.

---

## Table of Contents

1. [Training Pipeline Overview](#training-pipeline-overview)
2. [Supported Data Sources](#supported-data-sources)
   - [Websites & URLs](#1-websites--urls)
   - [Local Files](#2-local-files)
   - [Directories](#3-directories)
3. [CLI Training Commands & Flags](#cli-training-commands--flags)
4. [Training Pipeline Stages](#training-pipeline-stages)
   - [Stage 1: Ingestion & Sanitization](#stage-1-ingestion--sanitization)
   - [Stage 2: Dynamic Vocabulary Expansion](#stage-2-dynamic-vocabulary-expansion)
   - [Stage 3: Semantic Chunking & Dataset Creation](#stage-3-semantic-chunking--dataset-creation)
   - [Stage 4: MPS Matrix Engine Training](#stage-4-mps-matrix-engine-training)
   - [Stage 5: High-Speed Database & RAM Persistence](#stage-5-high-speed-database--ram-persistence)
5. [Hyperparameter Tuning](#hyperparameter-tuning)
6. [Training Metrics Explained](#training-metrics-explained)
7. [Failsafes & Recovery](#failsafes--recovery)
8. [Best Practices for Apple Silicon M4](#best-practices-for-apple-silicon-m4)

---

## Training Pipeline Overview

Tiwut-AI features an end-to-end training pipeline tailored for Apple Silicon Unified Memory and Metal Performance Shaders (MPS):

```
Raw Sources (URLs / Files / Dirs)
       │
       ▼
[1] Ingestion & Sanitization (HTML stripping / multi-encoding reader)
       │
       ▼
[2] Tokenizer Vocabulary Expansion (subword frequency analysis)
       │
       ▼
[3] Semantic Chunking (window overlap & boundary detection)
       │
       ▼
[4] Neural Training on MPS (AdamW + Cosine Annealing + RoPE + SwiGLU)
       │
       ▼
[5] Persistence (SQLite WAL Storage + Unified RAM Pinning)
```

---

## Supported Data Sources

### 1. Websites & URLs

Tiwut-AI automatically scrapes web pages, strips boilerplate (scripts, navigation, headers, footers, ads), and extracts the core textual content.

```bash
./tiwut-ai -train -url https://en.wikipedia.org/wiki/Apple_silicon -epochs 10
```

Raw text files or GitHub raw URLs can also be ingested directly:

```bash
./tiwut-ai -train -url https://raw.githubusercontent.com/tiwut/tiwut/refs/heads/main/README.md -epochs 10
```

### 2. Local Files

Train on individual plain text, markdown, code, or data files (`.txt`, `.md`, `.csv`, `.tsv`, `.json`, `.py`, `.log`, `.rst`):

```bash
./tiwut-ai -train -file ./notes.txt -epochs 15
```

### 3. Directories

Recursively scan and train on an entire folder containing multiple documents:

```bash
./tiwut-ai -train -dir ./documents -epochs 8
```

---

## CLI Training Commands & Flags

| Flag | Argument | Description | Default |
|---|---|---|---|
| `-train`, `--train` | None | Triggers the neural training pipeline | Required |
| `-url`, `--url` | `<URL>` | Web address to scrape and ingest | `None` |
| `-file`, `--file` | `<PATH>` | Path to a single text document | `None` |
| `-dir`, `--dir` | `<PATH>` | Path to a directory for recursive ingestion | `None` |
| `-epochs`, `--epochs` | `<INT>` | Number of full passes over the dataset | From `config.json` (`10`) |
| `-lr`, `--lr` | `<FLOAT>` | Initial learning rate for AdamW | From `config.json` (`0.0003`) |
| `-batch`, `--batch` | `<INT>` | Batch size for gradient updates | From `config.json` (`16`) |

### Example Combinations:

```bash
# Train on a file with custom epochs and learning rate:
./tiwut-ai -train -file manual.txt -epochs 20 -lr 0.0002

# Train on a URL with a smaller batch size:
./tiwut-ai -train -url https://example.com/docs -epochs 12 -batch 8

# Train on a whole directory with default settings:
./tiwut-ai -train -dir ./knowledge_base
```

---

## Training Pipeline Stages

### Stage 1: Ingestion & Sanitization

- **Websites**: Ingested via an HTTP client with browser user-agent emulation, redirect tracking, and HTML parsing via BeautifulSoup. Boilerplate tags (`<script>`, `<style>`, `<nav>`, `<footer>`, `<aside>`, etc.) are decomposed.
- **Local Documents**: Read with automatic fallback across multiple character encodings (`utf-8`, `utf-8-sig`, `latin-1`, `cp1252`). Binary files are safely detected and skipped.

### Stage 2: Dynamic Vocabulary Expansion

- The **Byte-Level Tokenizer** analyzes the frequency of new word forms in the ingested documents.
- If new recurring subwords are found, they are assigned dedicated token IDs.
- The **TiwutNeuralAI** model dynamically expands its token embedding table (`tok_embeddings`) and language modeling head (`lm_head`) on the GPU without corrupting or resetting existing weights.

### Stage 3: Semantic Chunking & Dataset Creation

- Text is split at natural paragraph and sentence boundaries.
- Chunks are created using sliding windows governed by:
  - `chunk_size`: Maximum sequence length per chunk (default: `256` tokens).
  - `chunk_overlap`: Overlap between adjacent chunks (default: `64` tokens) to preserve contextual flow.

### Stage 4: MPS Matrix Engine Training

- Samples are fed into the PyTorch DataLoader.
- Model weights are optimized using **AdamW** with:
  - **Cosine Annealing Scheduler**: Decays the learning rate smoothly from `learning_rate` down to `min_learning_rate`.
  - **Gradient Clipping**: Norm capped at `grad_clip = 1.0` to eliminate exploding gradients.
  - **Weight Decay**: Applied at `0.01` to prevent overfitting on smaller corpora.

### Stage 5: High-Speed Database & RAM Persistence

- Upon completion:
  1. Updated model weights are saved to `storage/neural_weights.pt`.
  2. The expanded tokenizer vocabulary is saved to `storage/tokenizer.json`.
  3. Chunks, token sequences, and dense semantic latent vectors are saved into the SQLite WAL database (`storage/neural_brain.db`).
  4. The active memory bank is pinned into Unified RAM for zero-latency retrieval during chat and query modes.

---

## Hyperparameter Tuning

All default hyperparameters are configured in `config.json` under the `"training"` key:

```json
{
  "training": {
    "batch_size": 16,
    "learning_rate": 0.0003,
    "min_learning_rate": 0.00003,
    "weight_decay": 0.01,
    "grad_clip": 1.0,
    "default_epochs": 10,
    "chunk_size": 256,
    "chunk_overlap": 64
  }
}
```

### Guidance for Selecting Parameters:

| Objective | Recommended Settings | Rationale |
|---|---|---|
| **Small Document (< 10 KB)** | `epochs: 15-20`, `lr: 0.0003`, `batch: 8` | Higher epochs allow the model to fully memorize key factual sequences. |
| **Medium Corpus (10 KB - 1 MB)** | `epochs: 10`, `lr: 0.0003`, `batch: 16` | Standard balance between training speed and convergence. |
| **Large Knowledge Base (> 1 MB)** | `epochs: 5-8`, `lr: 0.0002`, `batch: 32` | Lower learning rate and higher batch size maximize MPS throughput. |

---

## Training Metrics Explained

During training, Tiwut-AI displays real-time statistics for each epoch:

```text
Epoch  5/10 | Loss: 6.4359 | Perplexity: 623.85 | Speed: 14,015 tok/s | LR: 1.65e-04
```

- **Loss**: Cross-entropy loss measuring the model's next-token prediction error. Lower is better.
- **Perplexity**: `exp(loss)`. Represents the effective number of choices the model is uncertain between at each token. Lower is better.
- **Speed**: Processing throughput in tokens per second processed on the M4 MPS engine.
- **LR**: Current learning rate calculated by the cosine schedule.

---

## Failsafes & Recovery

- **Safe Interruptions (`Ctrl+C`)**: If you press `Ctrl+C` during training, the trainer safely saves the current model weights, tokenizer state, and database chunks before exiting. No progress is corrupted.
- **NaN Protection**: If a numerical overflow occurs in gradients, the step is safely skipped to preserve model integrity.
- **OOM Prevention**: The batch size and sequence lengths are capped within Apple Silicon Unified Memory limits.

---

## Best Practices for Apple Silicon M4

1. **Verify MPS Device Acceleration**:
   Check that Tiwut-AI detects your M4 chip and MPS compute device:
   ```bash
   ./tiwut-ai -status
   ```
2. **Review Learned Knowledge in RAM**:
   After training, check the active chunks and sources in memory:
   ```bash
   ./tiwut-ai -status
   ```
3. **Resetting Training State**:
   If you ever want to wipe all trained weights, tokenizer expansions, and database chunks to start fresh:
   ```bash
   ./tiwut-ai -reset
   ```
