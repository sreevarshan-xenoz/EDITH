#!/usr/bin/env python3
"""
Interactive CLI for the LLM Wrapper
"""

import argparse
import sys
from pathlib import Path
from llm_wrapper import LLMWrapper

def main():
    parser = argparse.ArgumentParser(description="Universal Local LLM Chat")
    parser.add_argument("--model", "-m", default="llama3.2", help="Model to use")
    parser.add_argument("--url", "-u", default="http://localhost:11434", help="Base URL for LLM server")
    parser.add_argument("--image", "-i", action="append", help="Image file to include (can be used multiple times)")
    parser.add_argument("--system", "-s", help="System prompt")
    parser.add_argument("--message", help="Single message mode")
    parser.add_argument("--list-models", action="store_true", help="List available models")
    
    args = parser.parse_args()
    
    # Initialize wrapper
    llm = LLMWrapper(base_url=args.url, model=args.model)
    
    if args.list_models:
        models = llm.list_models()
        print("Available models:")
        for model in models:
            print(f"  - {model}")
        return
    
    # Show capabilities
    caps = llm.get_capabilities()
    print(f"🤖 Using: {caps.model_name or args.model}")
    print(f"📷 Vision: {'✅' if caps.supports_vision else '❌'}")
    print(f"🧠 Thinking: {'✅' if caps.supports_thinking else '❌'}")
    print("-" * 50)
    
    if args.message:
        # Single message mode
        response = llm.chat(
            message=args.message,
            images=args.image,
            system_prompt=args.system
        )
        if isinstance(response, dict):
            print(f"\n💭 Response: {response['response']}")
            if response.get('thinking'):
                print(f"🤔 Thinking: {response['thinking']}")
        else:
            print(f"\n💭 Response: {response}")
    else:
        # Interactive mode
        print("Interactive mode - type 'quit' to exit, '/image <path>' to add image, '/model <name>' to switch")
        
        current_images = args.image or []
        
        while True:
            try:
                user_input = input("\n👤 You: ").strip()
                
                if user_input.lower() in ['quit', 'exit', 'q']:
                    break
                
                if user_input.startswith('/image '):
                    img_path = user_input[7:].strip()
                    if Path(img_path).exists():
                        current_images.append(img_path)
                        print(f"📷 Added image: {img_path}")
                    else:
                        print(f"❌ Image not found: {img_path}")
                    continue
                
                if user_input.startswith('/model '):
                    new_model = user_input[7:].strip()
                    llm.switch_model(new_model)
                    continue
                
                if user_input.startswith('/clear'):
                    current_images = []
                    print("🗑️ Cleared images")
                    continue
                
                if not user_input:
                    continue
                
                print("🤖 Assistant: ", end="")
                response = llm.chat(
                    message=user_input,
                    images=current_images if current_images else None,
                    system_prompt=args.system
                )
                
                # Clear images after use (unless you want to keep them)
                current_images = []
                
            except KeyboardInterrupt:
                print("\n👋 Goodbye!")
                break
            except Exception as e:
                print(f"❌ Error: {e}")

if __name__ == "__main__":
    main()