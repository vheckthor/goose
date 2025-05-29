// WebSocket connection and chat functionality
let socket = null;
let sessionId = generateSessionId();
let isConnected = false;

// DOM elements
const messagesContainer = document.getElementById('messages');
const messageInput = document.getElementById('message-input');
const sendButton = document.getElementById('send-button');
const connectionStatus = document.getElementById('connection-status');

// Generate a random session ID
function generateSessionId() {
    return 'session_' + Math.random().toString(36).substr(2, 9);
}

// Format timestamp
function formatTimestamp(date) {
    return date.toLocaleTimeString('en-US', { 
        hour: '2-digit', 
        minute: '2-digit' 
    });
}

// Create message element
function createMessageElement(content, role, timestamp) {
    const messageDiv = document.createElement('div');
    messageDiv.className = `message ${role}`;
    
    // Parse content for code blocks and format
    const formattedContent = formatMessageContent(content);
    messageDiv.innerHTML = formattedContent;
    
    // Add timestamp
    const timestampDiv = document.createElement('div');
    timestampDiv.className = 'timestamp';
    timestampDiv.textContent = formatTimestamp(new Date(timestamp || Date.now()));
    messageDiv.appendChild(timestampDiv);
    
    return messageDiv;
}

// Format message content (handle markdown-like formatting)
function formatMessageContent(content) {
    // Escape HTML
    let formatted = content
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
    
    // Handle code blocks
    formatted = formatted.replace(/```(\w+)?\n([\s\S]*?)```/g, (match, lang, code) => {
        return `<pre><code class="language-${lang || 'plaintext'}">${code.trim()}</code></pre>`;
    });
    
    // Handle inline code
    formatted = formatted.replace(/`([^`]+)`/g, '<code>$1</code>');
    
    // Handle line breaks
    formatted = formatted.replace(/\n/g, '<br>');
    
    return formatted;
}

// Add message to chat
function addMessage(content, role, timestamp) {
    // Remove welcome message if it exists
    const welcomeMessage = messagesContainer.querySelector('.welcome-message');
    if (welcomeMessage) {
        welcomeMessage.remove();
    }
    
    const messageElement = createMessageElement(content, role, timestamp);
    messagesContainer.appendChild(messageElement);
    
    // Scroll to bottom
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Add loading indicator
function addLoadingIndicator() {
    const loadingDiv = document.createElement('div');
    loadingDiv.className = 'message assistant loading-message';
    loadingDiv.innerHTML = '<span class="loading"></span> Thinking...';
    messagesContainer.appendChild(loadingDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
    return loadingDiv;
}

// Remove loading indicator
function removeLoadingIndicator() {
    const loadingMessage = messagesContainer.querySelector('.loading-message');
    if (loadingMessage) {
        loadingMessage.remove();
    }
}

// Connect to WebSocket
function connectWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;
    
    socket = new WebSocket(wsUrl);
    
    socket.onopen = () => {
        console.log('WebSocket connected');
        isConnected = true;
        connectionStatus.textContent = 'Connected';
        connectionStatus.className = 'status connected';
        sendButton.disabled = false;
    };
    
    socket.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            handleServerMessage(data);
        } catch (e) {
            console.error('Failed to parse message:', e);
        }
    };
    
    socket.onclose = () => {
        console.log('WebSocket disconnected');
        isConnected = false;
        connectionStatus.textContent = 'Disconnected';
        connectionStatus.className = 'status disconnected';
        sendButton.disabled = true;
        
        // Attempt to reconnect after 3 seconds
        setTimeout(connectWebSocket, 3000);
    };
    
    socket.onerror = (error) => {
        console.error('WebSocket error:', error);
    };
}

// Handle messages from server
function handleServerMessage(data) {
    switch (data.type) {
        case 'response':
            // For streaming responses, we need to handle partial messages
            handleStreamingResponse(data);
            break;
        case 'tool_request':
            handleToolRequest(data);
            break;
        case 'tool_response':
            handleToolResponse(data);
            break;
        case 'tool_confirmation':
            handleToolConfirmation(data);
            break;
        case 'thinking':
            handleThinking(data);
            break;
        case 'context_exceeded':
            handleContextExceeded(data);
            break;
        case 'error':
            removeLoadingIndicator();
            addMessage(`Error: ${data.message}`, 'assistant', Date.now());
            break;
        default:
            console.log('Unknown message type:', data.type);
    }
}

// Track current streaming message
let currentStreamingMessage = null;

// Handle streaming responses
function handleStreamingResponse(data) {
    removeLoadingIndicator();
    
    // If this is the first chunk of a new message
    if (!currentStreamingMessage || data.role !== currentStreamingMessage.role) {
        // If there was a previous streaming message, finalize it
        if (currentStreamingMessage) {
            currentStreamingMessage = null;
        }
        
        // Create a new message element
        const messageElement = createMessageElement(data.content, data.role || 'assistant', data.timestamp);
        messageElement.setAttribute('data-streaming', 'true');
        messagesContainer.appendChild(messageElement);
        
        currentStreamingMessage = {
            element: messageElement,
            content: data.content,
            role: data.role || 'assistant',
            timestamp: data.timestamp
        };
    } else {
        // Append to existing streaming message
        currentStreamingMessage.content += data.content;
        
        // Update the message content
        const contentDiv = currentStreamingMessage.element.querySelector('div:first-child') || currentStreamingMessage.element;
        contentDiv.innerHTML = formatMessageContent(currentStreamingMessage.content);
    }
    
    // Scroll to bottom
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle tool requests
function handleToolRequest(data) {
    const toolDiv = document.createElement('div');
    toolDiv.className = 'message assistant tool-message';
    
    const headerDiv = document.createElement('div');
    headerDiv.className = 'tool-header';
    headerDiv.innerHTML = `🔧 <strong>${data.tool_name}</strong>`;
    
    const contentDiv = document.createElement('div');
    contentDiv.className = 'tool-content';
    
    // Format the arguments
    if (data.tool_name === 'developer__shell' && data.arguments.command) {
        contentDiv.innerHTML = `<pre><code>${escapeHtml(data.arguments.command)}</code></pre>`;
    } else if (data.tool_name === 'developer__text_editor') {
        const action = data.arguments.command || 'unknown';
        const path = data.arguments.path || 'unknown';
        contentDiv.innerHTML = `<div class="tool-param"><strong>action:</strong> ${action}</div>`;
        contentDiv.innerHTML += `<div class="tool-param"><strong>path:</strong> ${escapeHtml(path)}</div>`;
        if (data.arguments.file_text) {
            contentDiv.innerHTML += `<div class="tool-param"><strong>content:</strong> <pre><code>${escapeHtml(data.arguments.file_text.substring(0, 200))}${data.arguments.file_text.length > 200 ? '...' : ''}</code></pre></div>`;
        }
    } else {
        contentDiv.innerHTML = `<pre><code>${JSON.stringify(data.arguments, null, 2)}</code></pre>`;
    }
    
    toolDiv.appendChild(headerDiv);
    toolDiv.appendChild(contentDiv);
    
    messagesContainer.appendChild(toolDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle tool responses
function handleToolResponse(data) {
    if (data.is_error) {
        const errorDiv = document.createElement('div');
        errorDiv.className = 'message tool-error';
        errorDiv.innerHTML = `<strong>Tool Error:</strong> ${escapeHtml(data.result.error || 'Unknown error')}`;
        messagesContainer.appendChild(errorDiv);
    } else {
        // Handle successful tool response
        if (Array.isArray(data.result)) {
            data.result.forEach(content => {
                if (content.type === 'text' && content.text) {
                    const responseDiv = document.createElement('div');
                    responseDiv.className = 'message tool-result';
                    responseDiv.innerHTML = formatMessageContent(content.text);
                    messagesContainer.appendChild(responseDiv);
                }
            });
        }
    }
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle tool confirmations
function handleToolConfirmation(data) {
    const confirmDiv = document.createElement('div');
    confirmDiv.className = 'message tool-confirmation';
    confirmDiv.innerHTML = `
        <div class="tool-confirm-header">⚠️ Tool Confirmation Required</div>
        <div class="tool-confirm-content">
            <strong>${data.tool_name}</strong> wants to execute with:
            <pre><code>${JSON.stringify(data.arguments, null, 2)}</code></pre>
        </div>
        <div class="tool-confirm-note">Auto-approved in web mode (UI coming soon)</div>
    `;
    messagesContainer.appendChild(confirmDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle thinking messages
function handleThinking(data) {
    // For now, just log thinking messages
    console.log('Thinking:', data.message);
}

// Handle context exceeded
function handleContextExceeded(data) {
    const contextDiv = document.createElement('div');
    contextDiv.className = 'message context-warning';
    contextDiv.innerHTML = `
        <div class="context-header">⚠️ Context Length Exceeded</div>
        <div class="context-content">${escapeHtml(data.message)}</div>
        <div class="context-note">Auto-summarizing conversation...</div>
    `;
    messagesContainer.appendChild(contextDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Escape HTML to prevent XSS
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Send message
function sendMessage() {
    const message = messageInput.value.trim();
    if (!message || !isConnected) return;
    
    // Add user message to chat
    addMessage(message, 'user', Date.now());
    
    // Clear input
    messageInput.value = '';
    messageInput.style.height = 'auto';
    
    // Add loading indicator
    addLoadingIndicator();
    
    // Send message through WebSocket
    socket.send(JSON.stringify({
        type: 'message',
        content: message,
        session_id: sessionId,
        timestamp: Date.now()
    }));
}

// Event listeners
sendButton.addEventListener('click', sendMessage);

messageInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        sendMessage();
    }
});

// Auto-resize textarea
messageInput.addEventListener('input', () => {
    messageInput.style.height = 'auto';
    messageInput.style.height = messageInput.scrollHeight + 'px';
});

// Initialize WebSocket connection
connectWebSocket();

// Focus on input
messageInput.focus();