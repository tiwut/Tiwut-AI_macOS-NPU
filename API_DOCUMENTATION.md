# Tiwut-AI v2 REST & SSE Streaming API Reference

The **Tiwut-AI v2** engine provides a high-throughput, async REST and Server-Sent Events (SSE) streaming API powered by **Axum** and **Tokio**. It allows any desktop app, web app, terminal client, or external service to communicate with the neural model and semantic memory bank.

---

## 🌐 Base URL
```
http://127.0.0.1:8080
```
*(Configurable via `--host` and `--port` CLI flags or `config.json`)*

---

## 📋 Endpoints Overview

| Endpoint | Method | Description | Content-Type |
|---|---|---|---|
| `/api/health` | `GET` | Server health check probe | `text/plain` |
| `/api/status` | `GET` | Hardware specs, parameters, memory metrics | `application/json` |
| `/api/chat` | `POST` | Non-streaming chat completion | `application/json` |
| `/api/chat/stream` | `POST` | Real-time SSE token-by-token streaming | `text/event-stream` |
| `/api/ask` | `POST` | Instant semantic Q&A extraction | `application/json` |
| `/api/train` | `POST` | Dispatch neural training on URLs, files, or text | `text/event-stream` |
| `/api/memory` | `GET` | List RAM memory chunks and learned sources | `application/json` |
| `/api/config` | `GET` | Retrieve active inference & model hyperparameters | `application/json` |
| `/api/config` | `POST` | Update runtime inference hyperparameters | `application/json` |
| `/api/reset` | `POST` | Reset model weights and knowledge memory bank | `application/json` |

---

## 🔍 Detailed Endpoint Documentation

### 1. Health Check
`GET /api/health`

#### Response:
```
OK
```

---

### 2. System & Model Telemetry
`GET /api/status`

#### Response:
```json
{
  "hardware": {
    "os": "Darwin",
    "arch": "aarch64",
    "chip_name": "Apple M4",
    "cpu_cores": 10,
    "total_ram_mb": 16384,
    "available_ram_mb": 4120,
    "is_apple_silicon": true,
    "acceleration_engine": "Apple Silicon NEON / Accelerate SIMD + Rayon Multi-Core"
  },
  "total_parameters": 6950144,
  "vocab_size": 1415,
  "memory_chunks": 259,
  "memory_ram_mb": 0.81,
  "model_path": "ai.model",
  "engine": "Tiwut-AI v2 (Rust Core)"
}
```

---

### 3. Real-Time Streaming Chat (SSE)
`POST /api/chat/stream`

#### Request Body:
```json
{
  "message": "What is Apple Silicon?",
  "temperature": 0.6,
  "max_tokens": 250
}
```

#### Response Stream (`text/event-stream`):
```
data: Apple
data:  Silicon
data:  is
data:  Apple's
...
data: [Source: builtin://default_english_knowledge]

event: done
data: [DONE]
```

---

### 4. Single Question / Answer Query
`POST /api/ask`

#### Request Body:
```json
{
  "question": "What is an NPU?"
}
```

#### Response:
```json
{
  "question": "What is an NPU?",
  "answer": "An NPU (Neural Processing Unit), such as the Apple Neural Engine, is a specialized hardware accelerator engineered for fast matrix multiplication and deep learning operations with high energy efficiency.\n\n[Source: builtin://default_english_knowledge]"
}
```

---

### 5. Memory Bank Inspection
`GET /api/memory`

#### Response:
```json
{
  "total_chunks": 259,
  "total_documents": 2,
  "total_tokens": 61760,
  "ram_usage_mb": 0.81,
  "sources": [
    "builtin://default_english_knowledge",
    "https://en.wikipedia.org/wiki/Rust_(programming_language)"
  ]
}
```

---

### 6. Neural Training Job
`POST /api/train`

#### Request Body:
```json
{
  "urls": ["https://en.wikipedia.org/wiki/Rust_(programming_language)"],
  "files": ["/path/to/notes.txt"],
  "raw_text": "Custom knowledge text here...",
  "epochs": 10,
  "learning_rate": 0.0004,
  "include_default_knowledge": true
}
```

---

## 💻 Client Integration Examples

### Python (Streaming SSE)
```python
import requests

response = requests.post(
    "http://127.0.0.1:8080/api/chat/stream",
    json={"message": "Explain how RoPE attention works.", "temperature": 0.6},
    stream=True
)

for line in response.iter_lines():
    if line:
        decoded = line.decode('utf-8')
        if decoded.startswith("data:"):
            payload = decoded[5:]
            if payload.startswith(" "):
                payload = payload[1:]
            if payload == "[DONE]":
                break
            print(payload, end="", flush=True)
print()
```

### JavaScript / Node.js
```javascript
const response = await fetch("http://127.0.0.1:8080/api/ask", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ question: "What is Tiwut-AI?" })
});
const data = await response.json();
console.log("Answer:", data.answer);
```

### cURL
```bash
curl -X POST http://127.0.0.1:8080/api/ask \
  -H "Content-Type: application/json" \
  -d '{"question": "What is Apple Silicon?"}'
```
