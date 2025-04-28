import React, { useState, useRef, useEffect } from 'react';
import { A2AClient } from '../a2a/client';

// Use crypto.randomUUID() for generating IDs since it's available in modern browsers
const generateId = () => crypto.randomUUID();

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  status?: 'sending' | 'sent' | 'error';
}

const ChatComponent: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputMessage, setInputMessage] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  
  const client = new A2AClient('http://localhost:41241', window.fetch.bind(window));

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputMessage.trim() || isLoading) return;

    const messageId = generateId();
    const userMessage: Message = {
      id: messageId,
      role: 'user',
      content: inputMessage,
      status: 'sending'
    };

    setMessages(prev => [...prev, userMessage]);
    setInputMessage('');
    setIsLoading(true);

    try {
      // Send message to server
      const taskResult = await client.sendTask({
        id: messageId,
        message: {
          role: 'user',
          parts: [{ text: inputMessage, type: 'text' }]
        }
      });

      // Update message status
      setMessages(prev => 
        prev.map(msg => 
          msg.id === messageId ? { ...msg, status: 'sent' } : msg
        )
      );

      // Get task response
      if (taskResult?.status?.state === 'completed' && taskResult?.response) {
        const assistantMessage: Message = {
          id: generateId(),
          role: 'assistant',
          content: taskResult.response.parts[0].text,
          status: 'sent'
        };
        setMessages(prev => [...prev, assistantMessage]);
      } else {
        // Poll for completion if task is still processing
        let attempts = 0;
        const maxAttempts = 30; // 30 seconds timeout
        const pollInterval = 1000; // 1 second

        while (attempts < maxAttempts) {
          const task = await client.getTask({ id: messageId });
          
          if (task?.status?.state === 'completed' && task?.response) {
            const assistantMessage: Message = {
              id: generateId(),
              role: 'assistant',
              content: task.response.parts[0].text,
              status: 'sent'
            };
            setMessages(prev => [...prev, assistantMessage]);
            break;
          } else if (task?.status?.state === 'failed') {
            throw new Error(task.error || 'Task failed');
          }

          await new Promise(resolve => setTimeout(resolve, pollInterval));
          attempts++;
        }

        if (attempts >= maxAttempts) {
          throw new Error('Task timed out');
        }
      }
    } catch (error) {
      console.error('Error sending message:', error);
      setMessages(prev =>
        prev.map(msg =>
          msg.id === messageId ? { ...msg, status: 'error' } : msg
        )
      );
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-white dark:bg-gray-800">
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.map((message) => (
          <div
            key={message.id}
            className={`flex ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            <div
              className={`max-w-[70%] p-3 rounded-lg ${
                message.role === 'user'
                  ? 'bg-blue-500 text-white'
                  : 'bg-gray-100 dark:bg-gray-700 dark:text-white'
              }`}
            >
              <div>{message.content}</div>
              {message.status === 'sending' && (
                <div className="text-xs opacity-70">Sending...</div>
              )}
              {message.status === 'error' && (
                <div className="text-xs text-red-500">Error sending message</div>
              )}
            </div>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>
      
      <form onSubmit={handleSubmit} className="border-t border-gray-200 dark:border-gray-700 p-4">
        <div className="flex space-x-2">
          <input
            type="text"
            value={inputMessage}
            onChange={(e) => setInputMessage(e.target.value)}
            placeholder="Type your message..."
            disabled={isLoading}
            className="flex-1 p-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
          />
          <button
            type="submit"
            disabled={isLoading || !inputMessage.trim()}
            className={`px-4 py-2 rounded-lg text-white ${
              isLoading || !inputMessage.trim()
                ? 'bg-blue-300 cursor-not-allowed'
                : 'bg-blue-500 hover:bg-blue-600'
            }`}
          >
            Send
          </button>
        </div>
      </form>
    </div>
  );
};

export default ChatComponent;