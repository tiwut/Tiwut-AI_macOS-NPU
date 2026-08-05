# Tiwut-AI v2 Neural Architecture & Technical Design

This document details the mathematical models, neural transformer architecture, memory bank retrieval system, and hardware acceleration mechanisms implemented in **Tiwut-AI v2**.

---

## 🧠 Transformer Core Architecture

Tiwut-AI v2 implements an optimized, multi-layer decoder-style Transformer neural network written in native Rust with **Rayon** multi-core data parallelism and SIMD instructions.

```
Input Tokens (x)
      │
      ▼
Token Embedding + Positional Projection
      │
┌─────┴──────────────────────────────────────┐
│            Transformer Layer (x N)         │
│                                            │
│   ┌────────────────────────────────────┐   │
│   │ RMSNorm (Pre-Attention)            │   │
│   └─────────────────┬──────────────────┘   │
│                     │                      │
│   ┌─────────────────▼──────────────────┐   │
│   │ Multi-Head RoPE Attention          │   │
│   │ - Rotary Position Embedding (RoPE) │   │
│   │ - Causal Masked Softmax            │   │
│   │ - Matrix Projection (W_q, W_k, W_v)│   │
│   └─────────────────┬──────────────────┘   │
│                     │                      │
│            Residual Addition (x + Attn)    │
│                     │                      │
│   ┌─────────────────▼──────────────────┐   │
│   │ RMSNorm (Pre-FFN)                  │   │
│   └─────────────────┬──────────────────┘   │
│                     │                      │
│   ┌─────────────────▼──────────────────┐   │
│   │ SwiGLU Gated Feed-Forward Network  │   │
│   │ FFN(x) = (Swish(xW_1) * xW_3) W_2  │   │
│   └─────────────────┬──────────────────┘   │
│                     │                      │
│            Residual Addition (h + FFN)     │
└─────────────────────┬──────────────────────┘
                      │
              Final RMSNorm Layer
                      │
               Output Projection
                      │
                      ▼
               Next Token Logits
```

### 1. Model Hyperparameters

| Dimension | Default Value | Description |
|---|---|---|
| `embed_dim` | `256` | Latent embedding space dimension |
| `feedforward_dim` | `1024` | Intermediate SwiGLU projection dimension |
| `num_layers` | `6` | Number of stacked transformer blocks |
| `num_heads` | `8` | Number of multi-head attention mechanisms |
| `head_dim` | `32` | Dimensionality of each individual attention head |
| `max_seq_len` | `512` | Maximum context sequence length |
| `total_parameters`| `~6.95 Million` | Parameter count for ultra-low latency inference |

---

### 2. Rotary Position Embeddings (RoPE)
Instead of static learned or sinusoidal additive positional embeddings, Tiwut-AI applies **Rotary Position Embeddings (RoPE)** to the query ($Q$) and key ($K$) projections:

$$\mathbf{R}_{\Theta, m}^d = \text{diag}\left( \mathbf{R}_{\theta_1, m}, \mathbf{R}_{\theta_2, m}, \dots, \mathbf{R}_{\theta_{d/2}, m} \right)$$

This allows relative distance encoding with decay over longer distances, preserving grammatical syntax and long-range dependencies.

---

### 3. SwiGLU Gated Feed-Forward Layers
We replace traditional ReLU/GELU multilayer perceptrons with **SwiGLU** (Swish Gated Linear Unit):

$$\text{SwiGLU}(x) = \left( \text{Swish}(x W_1) \odot x W_3 \right) W_2$$

Where:
$$\text{Swish}(z) = z \cdot \sigma(z) = \frac{z}{1 + e^{-z}}$$

---

### 4. Root Mean Square Normalization (RMSNorm)
Pre-layer normalization is performed using **RMSNorm**, which improves training stability and avoids mean-centering computations:

$$\bar{a}_i = \frac{a_i}{\text{RMS}(a)} \cdot g_i, \quad \text{where } \text{RMS}(a) = \sqrt{\frac{1}{d} \sum_{i=1}^d a_i^2 + \epsilon}$$

---

## 📚 In-RAM Associative Vector Memory Bank (RAG)

Tiwut-AI incorporates an in-RAM semantic associative vector bank:

1. **Semantic Latent Projection**:
   During text ingestion, documents are segmented into semantic chunks (256 tokens with 64 token overlap). The neural core computes a 256-dimensional semantic latent vector for each chunk.

2. **Cosine Similarity Associative Search**:
   $$\text{Similarity}(u, v) = \frac{u \cdot v}{\|u\|_2 \|v\|_2}$$

3. **Intelligent Answer Extraction**:
   When a user query is received, top-$K$ chunks with cosine similarity above `memory_threshold` are retrieved. The neural engine extracts sentences with high keyword and concept alignment and cites the exact source.

---

## 📦 Single-File Model Container (`ai.model`)

All components of the trained AI are saved into a single unified archive `ai.model`:
- **`weights.bin`**: Neural network weights (bincode binary).
- **`config.json`**: Model dimensions and training hyperparameters.
- **`tokenizer.json`**: Dynamic vocabulary mappings and subword merges.
- **`memory.bin`**: In-RAM memory bank chunks, latent vectors, and source metadata.

Training on any new source automatically updates and re-bundles `ai.model` atomically.
