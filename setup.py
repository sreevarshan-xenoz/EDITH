#!/usr/bin/env python3
"""
Setup script for LLM Wrapper
"""

import subprocess
import sys
import os

def install_requirements():
    """Install required packages"""
    try:
        subprocess.check_call([sys.executable, "-m", "pip", "install", "-r", "requirements.txt"])
        print("✅ Requirements installed successfully")
        return True
    except subprocess.CalledProcessError as e:
        print(f"❌ Failed to install requirements: {e}")
        return False

def check_ollama():
    """Check if Ollama is running"""
    try:
        import requests
        response = requests.get("http://localhost:11434/api/tags", timeout=5)
        if response.status_code == 200:
            print("✅ Ollama is running")
            models = response.json().get('models', [])
            if models:
                print(f"📦 Available models: {', '.join([m['name'] for m in models])}")
            else:
                print("⚠️  No models found. You might want to pull a model first:")
                print("   ollama pull llama3.2")
            return True
        else:
            print("❌ Ollama is not responding properly")
            return False
    except Exception as e:
        print("❌ Ollama is not running or not accessible")
        print("💡 Make sure Ollama is installed and running:")
        print("   - Install: https://ollama.ai/")
        print("   - Run: ollama serve")
        return False

def main():
    print("🚀 Setting up LLM Wrapper...")
    
    # Install requirements
    if not install_requirements():
        return
    
    # Check Ollama
    ollama_ok = check_ollama()
    
    print("\n" + "="*50)
    print("🎉 Setup complete!")
    print("\nUsage options:")
    print("1. CLI: python chat_cli.py")
    print("2. Web UI: python web_ui.py")
    print("3. Programmatic: from llm_wrapper import LLMWrapper")
    
    if not ollama_ok:
        print("\n⚠️  Note: Install and start Ollama first for full functionality")

if __name__ == "__main__":
    main()