#!/usr/bin/env python3
"""
Example usage of the LLM Wrapper
"""

from llm_wrapper import LLMWrapper

def main():
    # Initialize with your local LLM server
    llm = LLMWrapper(
        base_url="http://localhost:11434",  # Ollama default
        model="llama3.2"
    )
    
    print("=== Basic Chat ===")
    response = llm.chat("Explain quantum computing in simple terms")
    print()
    
    print("=== With System Prompt ===")
    response = llm.chat(
        "What's the weather like?",
        system_prompt="You are a helpful assistant that always responds in a pirate accent."
    )
    print()
    
    # Example with images (if model supports it)
    print("=== Vision Example ===")
    if llm.capabilities.supports_vision:
        # Uncomment if you have an image file
        # response = llm.chat(
        #     "What do you see in this image?",
        #     images=["path/to/your/image.jpg"]
        # )
        print("Vision supported! Add image paths to test.")
    else:
        print("Current model doesn't support vision")
    
    print()
    print("=== Model Info ===")
    caps = llm.get_capabilities()
    print(f"Model: {caps.model_name}")
    print(f"Vision: {caps.supports_vision}")
    print(f"Thinking: {caps.supports_thinking}")
    
    print()
    print("=== Available Models ===")
    models = llm.list_models()
    for model in models:
        print(f"  - {model}")

if __name__ == "__main__":
    main()