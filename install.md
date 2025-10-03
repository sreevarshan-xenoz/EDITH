# Installation Guide

## 1. Install Rust

### Windows
```powershell
# Using winget
winget install Rustlang.Rustup

# Or download from https://rustup.rs/
```

### Linux/macOS
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## 2. Build the Project

### Windows
```powershell
# Run the build script
.\build.ps1

# Or manually
cargo build --release
```

### Linux/macOS
```bash
# Build release binary
cargo build --release

# The binary will be at target/release/llm
```

## 3. Install (Optional)

```bash
# Install to cargo bin directory
cargo install --path .

# Now you can run from anywhere
llm "Hello world!"
```

## 4. Setup Ollama

Make sure you have Ollama running:

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Start the service
ollama serve

# Pull a model
ollama pull llama3.2
```

## 5. Test

```bash
# Quick test
./target/release/llm "Hello!"

# Interactive mode
./target/release/llm chat

# List models
./target/release/llm list
```

You're all set! 🚀