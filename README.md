# 🦀 Universal LLM Wrapper (Rust)

Fast, zero-dependency CLI for local LLMs with automatic capability detection.

## Features

- 🔍 **Auto-detection** of model capabilities (vision, thinking, streaming)
- 📷 **Image support** for vision models (LLaVA, Moondream, etc.)
- 🧠 **Thinking models** support (O1-style reasoning)
- 💬 **Interactive TUI** with real-time chat
- ⚡ **Zero runtime dependencies** - single binary
- 🔧 **Configurable** via TOML

## Installation

```bash
# Build from source
cargo build --release

# The binary will be at target/release/llm
```

## Usage

### Quick Chat
```bash
# Basic chat
./llm "Hello, how are you?"

# With specific model
./llm -m llava "What's in this image?" -i photo.jpg

# With system prompt
./llm -s "You are a helpful coding assistant" "Explain Rust ownership"
```

### Interactive Mode
```bash
# Start interactive chat
./llm chat

# Or just run without arguments
./llm
```

### Model Management
```bash
# List available models
./llm list

# Pull a new model
./llm pull llava

# Delete a model
./llm delete old-model

# Show model info
./llm info llama3.2
```

## Interactive Commands

In chat mode:
- `/image <path>` - Add image to next message
- `/model <name>` - Switch model
- `/clear` - Clear loaded images
- `/quit` or `/q` - Exit

## Configuration

Edit `config.toml`:

```toml
default_model = "llama3.2"
base_url = "http://localhost:11434"

vision_models = ["llava", "moondream", "qwen-vl"]
thinking_models = ["o1", "reasoning"]

[model_aliases]
smart = "llama3.2:70b"
vision = "llava"
```

## Supported Backends

- **Ollama** (default)
- **LM Studio** (OpenAI-compatible API)
- Any OpenAI-compatible endpoint

## Examples

```bash
# Vision model with image
./llm -m llava "Describe this screenshot" -i desktop.png

# Thinking model for complex reasoning  
./llm -m o1-preview "Solve this math problem step by step: ..."

# Quick model switching
./llm -m coder "Write a Rust function to parse JSON"
```

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

The release binary is completely self-contained - no Python, no dependencies, just works.