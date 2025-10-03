#!/usr/bin/env python3
"""
Universal Local LLM Wrapper
Dynamically detects and adapts to model capabilities like vision and thinking
"""

import json
import base64
import requests
from typing import Dict, List, Optional, Any, Union
from dataclasses import dataclass
from pathlib import Path
import mimetypes
import time
import os

@dataclass
class ModelCapabilities:
    """Model capability detection"""
    supports_vision: bool = False
    supports_thinking: bool = False
    supports_streaming: bool = True
    max_tokens: int = 4096
    model_name: str = ""

class LLMWrapper:
    def __init__(self, base_url: str = "http://localhost:11434", model: str = "llama3.2", config_file: str = "config.json"):
        self.base_url = base_url.rstrip('/')
        self.model = model
        self.capabilities = ModelCapabilities()
        self.config = self._load_config(config_file)
        self._detect_capabilities()
    
    def _load_config(self, config_file: str) -> Dict:
        """Load configuration from JSON file"""
        try:
            if os.path.exists(config_file):
                with open(config_file, 'r') as f:
                    return json.load(f)
        except Exception as e:
            print(f"Warning: Could not load config: {e}")
        
        # Default config
        return {
            "vision_models": ["llava", "bakllava", "moondream", "vision"],
            "thinking_models": ["o1", "reasoning", "thinking"],
            "model_aliases": {}
        }
    
    def _detect_capabilities(self):
        """Auto-detect what the model can do"""
        try:
            # Check if server is reachable
            response = requests.get(f"{self.base_url}/api/tags", timeout=5)
            if response.status_code == 200:
                models = response.json().get('models', [])
                current_model = next((m for m in models if self.model in m['name']), None)
                
                if current_model:
                    self.capabilities.model_name = current_model['name']
                    model_name_lower = current_model['name'].lower()
                    
                    # Check for vision capabilities using config
                    vision_indicators = self.config.get('vision_models', [])
                    if any(indicator in model_name_lower for indicator in vision_indicators):
                        self.capabilities.supports_vision = True
                    
                    # Check for thinking capabilities using config
                    thinking_indicators = self.config.get('thinking_models', [])
                    if any(indicator in model_name_lower for indicator in thinking_indicators):
                        self.capabilities.supports_thinking = True
                        
                    # Try to get more detailed model info
                    try:
                        model_info = requests.post(f"{self.base_url}/api/show", 
                                                 json={"name": self.model}, timeout=5)
                        if model_info.status_code == 200:
                            info = model_info.json()
                            # Extract additional capabilities from model info if available
                            pass
                    except:
                        pass
                        
        except requests.exceptions.RequestException as e:
            print(f"Warning: Could not connect to LLM server at {self.base_url}: {e}")
        except Exception as e:
            print(f"Warning: Could not detect capabilities: {e}")
    
    def _encode_image(self, image_path: str) -> str:
        """Convert image to base64"""
        with open(image_path, "rb") as image_file:
            return base64.b64encode(image_file.read()).decode('utf-8')
    
    def _is_image_file(self, file_path: str) -> bool:
        """Check if file is an image"""
        mime_type, _ = mimetypes.guess_type(file_path)
        return mime_type and mime_type.startswith('image/')  
  
    def chat(self, 
             message: str, 
             images: Optional[List[str]] = None,
             system_prompt: Optional[str] = None,
             stream: bool = True) -> Union[str, Any]:
        """
        Universal chat method that adapts to model capabilities
        """
        
        # Build the request payload
        payload = {
            "model": self.model,
            "stream": stream
        }
        
        # Handle messages
        messages = []
        
        # Add system message if provided
        if system_prompt:
            messages.append({"role": "system", "content": system_prompt})
        
        # Build user message
        user_message = {"role": "user", "content": message}
        
        # Handle images if model supports vision
        if images and self.capabilities.supports_vision:
            # For models that support vision, add images to the message
            image_data = []
            for img_path in images:
                if Path(img_path).exists() and self._is_image_file(img_path):
                    image_data.append(self._encode_image(img_path))
            
            if image_data:
                user_message["images"] = image_data
        elif images and not self.capabilities.supports_vision:
            print("⚠️  Model doesn't support vision - ignoring images")
        
        messages.append(user_message)
        payload["messages"] = messages
        
        # Handle thinking models
        if self.capabilities.supports_thinking:
            # For thinking models, we might want to add special parameters
            payload["options"] = payload.get("options", {})
            payload["options"]["thinking"] = True
        
        return self._make_request(payload, stream)
    
    def _make_request(self, payload: Dict, stream: bool) -> Union[str, Any]:
        """Make the actual API request"""
        try:
            response = requests.post(
                f"{self.base_url}/api/chat",
                json=payload,
                stream=stream
            )
            
            if stream:
                return self._handle_stream(response)
            else:
                return response.json()
                
        except Exception as e:
            return f"Error: {e}"
    
    def _handle_stream(self, response):
        """Handle streaming responses"""
        full_response = ""
        thinking_content = ""
        
        for line in response.iter_lines():
            if line:
                try:
                    data = json.loads(line.decode('utf-8'))
                    
                    if 'message' in data:
                        content = data['message'].get('content', '')
                        
                        # Handle thinking models
                        if self.capabilities.supports_thinking and 'thinking' in data['message']:
                            thinking_content += data['message']['thinking']
                            print(f"🤔 Thinking: {data['message']['thinking']}", end='', flush=True)
                        else:
                            full_response += content
                            print(content, end='', flush=True)
                    
                    if data.get('done', False):
                        break
                        
                except json.JSONDecodeError:
                    continue
        
        print()  # New line after streaming
        return {
            "response": full_response,
            "thinking": thinking_content if self.capabilities.supports_thinking else None
        }
    
    def get_capabilities(self) -> ModelCapabilities:
        """Get current model capabilities"""
        return self.capabilities
    
    def list_models(self) -> List[str]:
        """List available models"""
        try:
            response = requests.get(f"{self.base_url}/api/tags")
            if response.status_code == 200:
                models = response.json().get('models', [])
                return [model['name'] for model in models]
        except Exception as e:
            print(f"Error listing models: {e}")
        return []
    
    def switch_model(self, model_name: str):
        """Switch to a different model"""
        # Check if it's an alias
        if model_name in self.config.get('model_aliases', {}):
            model_name = self.config['model_aliases'][model_name]
        
        self.model = model_name
        self._detect_capabilities()
        print(f"Switched to {model_name}")
        print(f"Capabilities: Vision={self.capabilities.supports_vision}, Thinking={self.capabilities.supports_thinking}")
    
    def pull_model(self, model_name: str) -> bool:
        """Pull/download a model"""
        try:
            print(f"Pulling model {model_name}...")
            response = requests.post(
                f"{self.base_url}/api/pull",
                json={"name": model_name},
                stream=True,
                timeout=300
            )
            
            for line in response.iter_lines():
                if line:
                    try:
                        data = json.loads(line.decode('utf-8'))
                        if 'status' in data:
                            print(f"\r{data['status']}", end='', flush=True)
                        if data.get('completed'):
                            print(f"\n✅ Model {model_name} pulled successfully")
                            return True
                    except json.JSONDecodeError:
                        continue
            
        except Exception as e:
            print(f"❌ Error pulling model: {e}")
            return False
    
    def delete_model(self, model_name: str) -> bool:
        """Delete a model"""
        try:
            response = requests.delete(f"{self.base_url}/api/delete", json={"name": model_name})
            if response.status_code == 200:
                print(f"✅ Model {model_name} deleted")
                return True
            else:
                print(f"❌ Failed to delete model: {response.text}")
                return False
        except Exception as e:
            print(f"❌ Error deleting model: {e}")
            return False
    
    def health_check(self) -> bool:
        """Check if the LLM server is healthy"""
        try:
            response = requests.get(f"{self.base_url}/api/tags", timeout=5)
            return response.status_code == 200
        except:
            return False


if __name__ == "__main__":
    # Example usage
    llm = LLMWrapper(model="llama3.2")
    
    print("🤖 LLM Wrapper initialized")
    print(f"Model: {llm.capabilities.model_name}")
    print(f"Vision support: {llm.capabilities.supports_vision}")
    print(f"Thinking support: {llm.capabilities.supports_thinking}")
    print()
    
    # Basic chat
    response = llm.chat("Hello! How are you?")
    print(f"Response: {response}")