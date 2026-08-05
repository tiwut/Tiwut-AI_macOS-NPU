# Tiwut-AI v2 Native Qt6 Desktop GUI Application

A high-performance, standalone native **Qt6 C++ Desktop Application** designed with a modern dark glassmorphic UI, real-time SSE chat streaming, and dynamic training dashboards for **Tiwut-AI v2**.

---

## 🌟 Architecture: Pure Decoupled Client

The GUI application is **100% decoupled** from the AI core engine:
- The AI Core runs as a headless Rust engine with a high-throughput async **REST & Server-Sent Events (SSE)** API.
- The GUI is a standalone C++/Qt6 desktop app communicating strictly over network protocols via `QNetworkAccessManager`.
- You can run the AI backend on the same machine or a remote high-performance server.

```
┌─────────────────────────────────────────────────┐
│     Tiwut-AI v2 Native Qt6 GUI (Desktop App)     │
│  [Chat] [Training] [Memory] [Telemetry] [Config]│
└───────────────────────┬─────────────────────────┘
                        │ HTTP / REST & SSE (Port 8080)
┌───────────────────────▼─────────────────────────┐
│   Tiwut-AI v2 Rust Headless Engine & REST API   │
│ (Transformer Core • RAG • Apple Silicon Accel)  │
└─────────────────────────────────────────────────┘
```

---

## 🚀 Quick Start

### 1. Start the Tiwut-AI v2 Engine (API Server)
In the `version_2/` directory:
```bash
# Build and run the headless AI engine & API server
cargo run --release -- serve --port 8080
```

### 2. Launch the Qt6 Desktop Application
In the `version_2/gui/` directory:
```bash
# Run the pre-built native binary
./build/tiwut-ai-gui
```

---

## 🛠️ Building from Source

### Prerequisites
- **CMake** 3.16+
- **Qt6** (Core, Gui, Widgets, Network)
  ```bash
  # macOS (Homebrew)
  brew install qt@6 cmake

  # Ubuntu / Debian
  sudo apt install qt6-base-dev cmake build-essential
  ```

### Build Steps
```bash
cd version_2/gui
mkdir -p build && cd build
cmake ..
make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)
```

The compiled binary will be in `version_2/gui/build/tiwut-ai-gui`.

---

## ✨ Features & Tabs

1. **💬 Neural Chat Studio**: Interactive chat interface with real-time SSE token streaming, prompt suggestions, and token counter.
2. **🧠 Training Studio**: Ingest URLs, local documents, directories, or custom text. Configure epochs & learning rate, and watch training logs with live progress bar.
3. **📚 Memory Bank**: In-RAM semantic vector search explorer, document source browser, and direct RAG Q&A query engine.
4. **⚡ Hardware Telemetry**: Live telemetry of Apple Silicon SoC (M4), CPU core multi-threading, RAM memory usage, and Transformer model dimensions.
5. **⚙️ Hyperparameters & Configuration**: Fine-tune temperature, top-k, top-p, repetition penalty, memory threshold, and customize API endpoint connections.
