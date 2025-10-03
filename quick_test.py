#!/usr/bin/env python3
import requests
import json

# Test basic connectivity
print("Testing Ollama connection...")

try:
    # Check if server is up
    response = requests.get("http://localhost:11434/api/tags", timeout=5)
    print(f"✅ Server is up: {response.status_code}")
    
    models = response.json().get('models', [])
    print(f"📦 Available models: {[m['name'] for m in models]}")
    
    # Try a simple chat
    print("\n🤖 Testing chat...")
    payload = {
        "model": "qwen3:8b",
        "messages": [{"role": "user", "content": "Hi! Just say 'Hello' back."}],
        "stream": False
    }
    
    print("Sending request...")
    response = requests.post("http://localhost:11434/api/chat", json=payload, timeout=120)
    
    if response.status_code == 200:
        result = response.json()
        content = result.get('message', {}).get('content', 'No content')
        print(f"✅ Response: {content}")
    else:
        print(f"❌ Error: {response.status_code} - {response.text}")
        
except Exception as e:
    print(f"❌ Error: {e}")