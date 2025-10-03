#!/usr/bin/env python3
"""
Ultra-simple LLM CLI that just works
"""

import requests
import json
import sys

def chat(model="qwen3:8b", message="Hello!"):
    url = "http://localhost:11434/api/chat"
    
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": message}],
        "stream": False  # Use non-streaming for simplicity
    }
    
    try:
        print(f"🤖 {model}: ", end="", flush=True)
        
        response = requests.post(url, json=payload, timeout=120)
        
        if response.status_code != 200:
            print(f"Error {response.status_code}: {response.text}")
            return
        
        result = response.json()
        content = result.get('message', {}).get('content', 'No response')
        
        # Handle thinking models - extract actual response after <think> tags
        if '<think>' in content and '</think>' in content:
            # Extract thinking part
            thinking_start = content.find('<think>')
            thinking_end = content.find('</think>') + 8
            thinking = content[thinking_start:thinking_end]
            actual_response = content[thinking_end:].strip()
            
            print(f"🤔 {thinking}")
            print(f"💭 {actual_response}")
        else:
            print(content)
        
        print()  # New line
        return content
        
    except requests.exceptions.RequestException as e:
        print(f"Connection error: {e}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        message = " ".join(sys.argv[1:])
        chat(message=message)
    else:
        # Interactive mode
        print("🤖 Simple LLM Chat (type 'quit' to exit)")
        while True:
            try:
                user_input = input("\n💬 You: ").strip()
                if user_input.lower() in ['quit', 'exit', 'q']:
                    break
                if user_input:
                    chat(message=user_input)
            except KeyboardInterrupt:
                print("\n👋 Goodbye!")
                break