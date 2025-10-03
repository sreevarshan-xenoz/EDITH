# 🚀 Universal LLM Wrapper - Usage Guide

## What You Built

A smart CLI wrapper that automatically adapts to different local LLM capabilities:
- **Thinking models** (like your qwen3:8b) show reasoning process
- **Vision models** (like LLaVA) handle images  
- **Regular models** work normally

## Quick Start

### Simple CLI (Recommended)
```bash
# Single question
python llm_simple.py "What's the capital of Japan?"

# Interactive chat
python llm_simple.py
```

### Full-Featured CLI
```bash
# Interactive with model switching
python llm_cli.py --chat

# List available models
python llm_cli.py --list

# Specific model
python llm_cli.py -m qwen3:8b "Explain quantum computing"
```

### Library Usage
```python
from llm_wrapper import LLMWrapper

llm = LLMWrapper(model="qwen3:8b")
response = llm.chat("Hello!")
print(f"Supports thinking: {llm.capabilities.supports_thinking}")
```

## Your Current Setup

- **Model**: qwen3:8b (thinking/reasoning model)
- **Server**: Ollama on localhost:11434
- **Features**: Shows reasoning process like OpenAI o1

## Example Output

```
🤖 qwen3:8b: 🤔 <think>
The user is asking about the capital of Japan. 
I know this is Tokyo, which is both the capital 
and largest city...
</think>
💭 The capital of Japan is Tokyo.
```

## Next Steps

1. **Add vision model**: `ollama pull llava` for image support
2. **Try different models**: `ollama pull llama3.2` for faster responses  
3. **Build Rust version**: Fix Windows toolchain for single binary

Your wrapper automatically detects what each model can do! 🎯