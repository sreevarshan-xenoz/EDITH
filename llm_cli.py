#!/usr/bin/env python3
"""
Simple LLM CLI - Works immediately without build issues
"""

import argparse
import json
import requests
import sys
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description="Simple LLM CLI")
    parser.add_argument("message", nargs="?", help="Message to send")
    parser.add_argument("-m", "--model", default="llama3.2", help="Model to use")
    parser.add_argument("-u", "--url", default="http://localhost:11434", help="Base URL")
    parser.add_argument("--chat", action="store_true", help="Interactive mode")
    parser.add_argument("--list", action="store_true", help="List models")
    
    args = parser.parse_args()
    
    if args.list:
        list_models(args.url)
    elif args.chat or not args.message:
        interactive_mode(args.model, args.url)
    else:
        response = send_message(args.model, args.url, args.message)
        print(response)

def list_models(base_url):
    try:
        response = requests.get(f"{base_url}/api/tags")
        if response.status_code == 200:
            models = response.json().get('models', [])
            print("Available models:")
            for model in models:
                print(f"  - {model['name']}")
        else:
            print("❌ Failed to fetch models")
    except Exception as e:
        print(f"❌ Error: {e}")

def interactive_mode(model, base_url):
    print(f"🤖 Connected to: {model}")
    print("Commands: /quit to exit")
    print("-" * 50)
    
    while True:
        try:
            user_input = input("💬 You: ").strip()
            
            if user_input.lower() in ['/quit', '/q', 'quit', 'exit']:
                break
                
            if not user_input:
                continue
                
            print("🤖 Assistant: ", end="", flush=True)
            response = send_message(model, base_url, user_input)
            print(response)
            
        except KeyboardInterrupt:
            print("\n👋 Goodbye!")
            break
        except Exception as e:
            print(f"❌ Error: {e}")

def send_message(model, base_url, message):
    try:
        payload = {
            "model": model,
            "messages": [
                {"role": "user", "content": message}
            ],
            "stream": False
        }
        
        response = requests.post(
            f"{base_url}/api/chat",
            json=payload,
            timeout=30
        )
        
        if response.status_code == 200:
            result = response.json()
            return result.get('message', {}).get('content', 'No response')
        else:
            return f"Error: {response.status_code}"
            
    except Exception as e:
        return f"Error: {e}"

if __name__ == "__main__":
    main()