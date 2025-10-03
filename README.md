# Universal Local LLM Wrapper

A flexible Python wrapper for local LLMs that automatically detects and adapts to model capabilities like vision input and thinking/reasoning features.

## Features

- 🔍 **Auto-detection** of model capabilities (vision, thinking, streaming)
- 📷 **Image support** for vision-enabled models (LLaVA, Moondream, etc.)
- 🧠 **Thinking models** support (O1-style reasoning)
- 💬 **Interactive CLI** with real-time streaming
- 🔄 **Model switching** on the fly
- ⚙️ **Configurable** endpoints and settings

## Quick Start

```bash
# Install dependencies
pip install -r requirements.txt

# Basic usage
python chat_cli.py

# With specific model
python chat_cli.py --model llava

# Single message with image
python chat_cli.py --message "What's in this image?" --image photo.jpg

# List available models
python chat_cli.py --list-models
```

## Usage Examples

### Interactive Chat
```bash
python chat_cli.py --model llama3.2
```

### With Images (Vision Models)
```bash
python chat_cli.py --model llava --image screenshot.png
```

### Programmatic Usage
```python
from llm_wrapper import LLMWrapper

llm = LLMWrapper(model="llama3.2")

# Basic chat
response = llm.chat("Hello!")

# With images
response = llm.chat("Describe this image", images=["photo.jpg"])

# Check capabilities
caps = llm.get_capabilities()
print(f"Vision: {caps.supports_vision}")
```

## CLI Commands

In interactive mode:
- `/image <path>` - Add image to next message
- `/model <name>` - Switch model
- `/clear` - Clear loaded images
- `quit` or `q` - Exit

## Supported Model Types

### Vision Models
- LLaVA variants
- Moondream
- BakLLaVA
- Any model with "vision" in the name

### Thinking Models
- O1-style models
- Models with "reasoning" or "thinking" in name

## Configuration

Edit `config.json` to customize:
- Default models
- Server endpoints
- Model aliases
- Capability detection patterns