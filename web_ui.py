#!/usr/bin/env python3
"""
Simple web UI for the LLM Wrapper
"""

from flask import Flask, render_template, request, jsonify, session
import os
import uuid
from llm_wrapper import LLMWrapper

app = Flask(__name__)
app.secret_key = 'your-secret-key-change-this'

# Global LLM instance
llm = None

@app.route('/')
def index():
    return render_template('index.html')

@app.route('/api/init', methods=['POST'])
def init_llm():
    global llm
    data = request.json
    
    try:
        llm = LLMWrapper(
            base_url=data.get('base_url', 'http://localhost:11434'),
            model=data.get('model', 'llama3.2')
        )
        
        if not llm.health_check():
            return jsonify({'error': 'Cannot connect to LLM server'}), 500
        
        caps = llm.get_capabilities()
        return jsonify({
            'success': True,
            'model': caps.model_name or llm.model,
            'capabilities': {
                'vision': caps.supports_vision,
                'thinking': caps.supports_thinking,
                'streaming': caps.supports_streaming
            }
        })
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/models')
def get_models():
    global llm
    if not llm:
        return jsonify({'error': 'LLM not initialized'}), 400
    
    models = llm.list_models()
    return jsonify({'models': models})

@app.route('/api/chat', methods=['POST'])
def chat():
    global llm
    if not llm:
        return jsonify({'error': 'LLM not initialized'}), 400
    
    data = request.json
    message = data.get('message', '')
    system_prompt = data.get('system_prompt')
    
    try:
        response = llm.chat(
            message=message,
            system_prompt=system_prompt,
            stream=False  # For web UI, we'll use non-streaming for simplicity
        )
        
        if isinstance(response, dict):
            return jsonify({
                'response': response.get('response', ''),
                'thinking': response.get('thinking')
            })
        else:
            return jsonify({'response': str(response)})
            
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/switch_model', methods=['POST'])
def switch_model():
    global llm
    if not llm:
        return jsonify({'error': 'LLM not initialized'}), 400
    
    data = request.json
    model_name = data.get('model')
    
    try:
        llm.switch_model(model_name)
        caps = llm.get_capabilities()
        
        return jsonify({
            'success': True,
            'model': caps.model_name or llm.model,
            'capabilities': {
                'vision': caps.supports_vision,
                'thinking': caps.supports_thinking,
                'streaming': caps.supports_streaming
            }
        })
    except Exception as e:
        return jsonify({'error': str(e)}), 500

if __name__ == '__main__':
    # Create templates directory if it doesn't exist
    os.makedirs('templates', exist_ok=True)
    
    # Create a simple HTML template
    html_template = '''
<!DOCTYPE html>
<html>
<head>
    <title>LLM Wrapper UI</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .chat-container { border: 1px solid #ddd; height: 400px; overflow-y: auto; padding: 10px; margin: 10px 0; }
        .message { margin: 10px 0; padding: 10px; border-radius: 5px; }
        .user { background-color: #e3f2fd; text-align: right; }
        .assistant { background-color: #f5f5f5; }
        .thinking { background-color: #fff3e0; font-style: italic; }
        input, textarea, select { width: 100%; padding: 8px; margin: 5px 0; }
        button { padding: 10px 20px; margin: 5px; cursor: pointer; }
        .status { padding: 10px; margin: 10px 0; border-radius: 5px; }
        .success { background-color: #d4edda; color: #155724; }
        .error { background-color: #f8d7da; color: #721c24; }
    </style>
</head>
<body>
    <h1>🤖 Universal LLM Wrapper</h1>
    
    <div id="status"></div>
    
    <div>
        <h3>Configuration</h3>
        <input type="text" id="baseUrl" placeholder="Base URL (http://localhost:11434)" value="http://localhost:11434">
        <input type="text" id="model" placeholder="Model name (llama3.2)" value="llama3.2">
        <button onclick="initLLM()">Connect</button>
        <button onclick="loadModels()">Load Models</button>
        <select id="modelSelect" onchange="switchModel()" style="display:none;"></select>
    </div>
    
    <div id="capabilities" style="display:none;">
        <h3>Model Capabilities</h3>
        <div id="capsList"></div>
    </div>
    
    <div>
        <h3>Chat</h3>
        <textarea id="systemPrompt" placeholder="System prompt (optional)" rows="2"></textarea>
        <div class="chat-container" id="chatContainer"></div>
        <textarea id="messageInput" placeholder="Type your message..." rows="3"></textarea>
        <button onclick="sendMessage()">Send</button>
        <button onclick="clearChat()">Clear</button>
    </div>

    <script>
        let currentModel = null;
        
        function showStatus(message, isError = false) {
            const status = document.getElementById('status');
            status.innerHTML = message;
            status.className = 'status ' + (isError ? 'error' : 'success');
        }
        
        function initLLM() {
            const baseUrl = document.getElementById('baseUrl').value;
            const model = document.getElementById('model').value;
            
            fetch('/api/init', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({base_url: baseUrl, model: model})
            })
            .then(response => response.json())
            .then(data => {
                if (data.error) {
                    showStatus('Error: ' + data.error, true);
                } else {
                    currentModel = data.model;
                    showStatus('Connected to ' + data.model);
                    updateCapabilities(data.capabilities);
                    loadModels();
                }
            })
            .catch(error => showStatus('Error: ' + error, true));
        }
        
        function updateCapabilities(caps) {
            const capsDiv = document.getElementById('capabilities');
            const capsList = document.getElementById('capsList');
            
            capsList.innerHTML = `
                📷 Vision: ${caps.vision ? '✅' : '❌'}<br>
                🧠 Thinking: ${caps.thinking ? '✅' : '❌'}<br>
                💬 Streaming: ${caps.streaming ? '✅' : '❌'}
            `;
            capsDiv.style.display = 'block';
        }
        
        function loadModels() {
            fetch('/api/models')
            .then(response => response.json())
            .then(data => {
                if (data.models) {
                    const select = document.getElementById('modelSelect');
                    select.innerHTML = '';
                    data.models.forEach(model => {
                        const option = document.createElement('option');
                        option.value = model;
                        option.textContent = model;
                        if (model === currentModel) option.selected = true;
                        select.appendChild(option);
                    });
                    select.style.display = 'block';
                }
            });
        }
        
        function switchModel() {
            const select = document.getElementById('modelSelect');
            const model = select.value;
            
            fetch('/api/switch_model', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({model: model})
            })
            .then(response => response.json())
            .then(data => {
                if (data.error) {
                    showStatus('Error: ' + data.error, true);
                } else {
                    currentModel = data.model;
                    showStatus('Switched to ' + data.model);
                    updateCapabilities(data.capabilities);
                }
            });
        }
        
        function sendMessage() {
            const message = document.getElementById('messageInput').value.trim();
            const systemPrompt = document.getElementById('systemPrompt').value.trim();
            
            if (!message) return;
            
            addMessage('user', message);
            document.getElementById('messageInput').value = '';
            
            fetch('/api/chat', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({
                    message: message,
                    system_prompt: systemPrompt || null
                })
            })
            .then(response => response.json())
            .then(data => {
                if (data.error) {
                    addMessage('assistant', 'Error: ' + data.error);
                } else {
                    if (data.thinking) {
                        addMessage('thinking', 'Thinking: ' + data.thinking);
                    }
                    addMessage('assistant', data.response);
                }
            })
            .catch(error => addMessage('assistant', 'Error: ' + error));
        }
        
        function addMessage(role, content) {
            const container = document.getElementById('chatContainer');
            const div = document.createElement('div');
            div.className = 'message ' + role;
            div.innerHTML = '<strong>' + role.charAt(0).toUpperCase() + role.slice(1) + ':</strong><br>' + content;
            container.appendChild(div);
            container.scrollTop = container.scrollHeight;
        }
        
        function clearChat() {
            document.getElementById('chatContainer').innerHTML = '';
        }
        
        // Enter to send message
        document.getElementById('messageInput').addEventListener('keydown', function(e) {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                sendMessage();
            }
        });
    </script>
</body>
</html>
    '''
    
    with open('templates/index.html', 'w') as f:
        f.write(html_template)
    
    print("🌐 Starting web UI at http://localhost:5000")
    app.run(debug=True, host='0.0.0.0', port=5000)