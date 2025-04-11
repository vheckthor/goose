import React, { useEffect, useRef, useState, useMemo } from 'react';
import { getApiUrl } from '../config';
import BottomMenu from './BottomMenu';
import FlappyGoose from './FlappyGoose';
import GooseMessage from './GooseMessage';
import Input from './Input';
import { type View, ViewOptions } from '../App';
import LoadingGoose from './LoadingGoose';
import MoreMenuLayout from './more_menu/MoreMenuLayout';
import { Card } from './ui/card';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import UserMessage from './UserMessage';
import Splash from './Splash';
import { SearchView } from './conversation/SearchView';
import { DeepLinkModal } from './ui/DeepLinkModal';
import 'react-toastify/dist/ReactToastify.css';
import { useMessageStream } from '../hooks/useMessageStream';
import { BotConfig } from '../botConfig';
import {
  Message,
  createUserMessage,
  ToolCall,
  ToolCallResult,
  ToolRequestMessageContent,
  ToolResponseMessageContent,
  ToolConfirmationRequestMessageContent,
  FrontendToolRequestMessageContent,
  getTextContent
} from '../types/message';
import { executeFrontendTool } from '../utils/frontendTools';

export interface ChatType {
  id: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
}

interface GeneratedBotConfig {
  id: string;
  name: string;
  description: string;
  instructions: string;
  activities: string[];
}

type DeepLinkBotConfig = BotConfig & {
  id: string;
  name: string;
  description: string;
  [key: string]: string | string[] | null;
}

// Helper function to determine if a message is a user message
const isUserMessage = (message: Message): boolean => {
  if (message.role === 'assistant') {
    return false;
  }

  if (message.content.every((c) => c.type === 'toolConfirmationRequest')) {
    return false;
  }
  return true;
};

export default function ChatView({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  const [hasMessages, setHasMessages] = useState(false);
  const [lastInteractionTime, setLastInteractionTime] = useState<number>(Date.now());
  const [showGame, setShowGame] = useState(false);
  const [waitingForAgentResponse, setWaitingForAgentResponse] = useState(false);
  const [showShareableBotModal, setshowShareableBotModal] = useState(false);
  const [generatedBotConfig, setGeneratedBotConfig] = useState<GeneratedBotConfig | null>(null);
  const scrollRef = useRef<ScrollAreaHandle>(null);

  // Get botConfig directly from appConfig
  const botConfig = window.appConfig.get('botConfig') as BotConfig | null;

  const {
    messages,
    append,
    stop,
    isLoading,
    error,
    setMessages,
    input: _input,
    setInput: _setInput,
    handleInputChange: _handleInputChange,
    handleSubmit: _submitMessage,
  } = useMessageStream({
    api: getApiUrl('/reply'),
    initialMessages: chat.messages,
    body: { session_id: chat.id, session_working_dir: window.appConfig.get('GOOSE_WORKING_DIR') },
    onFinish: async (_message, _reason) => {
      window.electron.stopPowerSaveBlocker();

      const timeSinceLastInteraction = Date.now() - lastInteractionTime;
      window.electron.logInfo('last interaction:' + lastInteractionTime);
      if (timeSinceLastInteraction > 60000) {
        window.electron.showNotification({
          title: 'Goose finished the task.',
          body: 'Click here to expand.',
        });
      }
    },
    _onToolCall: (toolCall: Record<string, unknown>) => {
      console.log('Tool call received:', toolCall);
    },
  });

  // Listen for make-agent-from-chat event
  useEffect(() => {
    const handleMakeAgent = async () => {
      window.electron.logInfo('Making agent from chat...');

      window.electron.logInfo('Current messages:');
      chat.messages.forEach((message, index) => {
        const role = isUserMessage(message) ? 'user' : 'assistant';
        const content = getTextContent(message);
        window.electron.logInfo(`Message ${index} (${role}): ${content}`);
      });

      const instructionsPrompt =
        'Based on our conversation so far, could you create:\n' +
        "1. A concise set of instructions (1-2 paragraphs) that describe what you've been helping with. Pay special attention if any output styles or formats are requested (and make it clear), and note any non standard tools used or required.\n" +
        '2. A list of 3-5 example activities (as a few words each at most) that would be relevant to this topic\n\n' +
        "Format your response with clear headings for 'Instructions:' and 'Activities:' sections." +
        'For example, perhaps we have been discussing fruit and you might write:\n\n' +
        'Instructions:\nUsing web searches we find pictures of fruit, and always check what language to reply in.' +
        'Activities:\nShow pics of apples, say a random fruit, share a fruit fact';

      setWaitingForAgentResponse(true);
      append(createUserMessage(instructionsPrompt));
      window.electron.logInfo('Injected instructions prompt into chat');
    };

    window.addEventListener('make-agent-from-chat', handleMakeAgent);
    return () => {
      window.removeEventListener('make-agent-from-chat', handleMakeAgent);
    };
  }, [append, chat.messages, setWaitingForAgentResponse]);

  // Listen for new messages and process agent response
  useEffect(() => {
    if (!waitingForAgentResponse || messages.length === 0) {
      return;
    }

    const lastMessage = messages[messages.length - 1];
    if (lastMessage.role === 'assistant') {
      const content = getTextContent(lastMessage);
      if (content) {
        window.electron.logInfo('Received agent response:');
        window.electron.logInfo(content);

        const instructionsMatch = content.match(/Instructions:(.*?)(?=Activities:|$)/s);
        const activitiesMatch = content.match(/Activities:(.*?)$/s);

        const instructions = instructionsMatch ? instructionsMatch[1].trim() : '';
        const activitiesText = activitiesMatch ? activitiesMatch[1].trim() : '';

        const activities = activitiesText
          .split(/\n+/)
          .map((line) => line.replace(/^[•\-*\d]+\.?\s*/, '').trim())
          .filter((activity) => activity.length > 0);

        const generatedConfig: GeneratedBotConfig = {
          id: `bot-${Date.now()}`,
          name: 'Custom Bot',
          description: 'Bot created from chat',
          instructions: instructions,
          activities: activities,
        };

        window.electron.logInfo('Extracted bot config:');
        window.electron.logInfo(JSON.stringify(generatedConfig, null, 2));

        setGeneratedBotConfig(generatedConfig);
        setshowShareableBotModal(true);
        window.electron.logInfo('Generated bot config for agent creation');
        setWaitingForAgentResponse(false);
      }
    }
  }, [messages, waitingForAgentResponse, setshowShareableBotModal, setGeneratedBotConfig]);

  useEffect(() => {
    setChat({
      ...chat,
      messages,
      id: chat.id,
      title: chat.title,
      messageHistoryIndex: chat.messageHistoryIndex
    });
  }, [messages, setChat, chat]);

  useEffect(() => {
    if (messages.length > 0) {
      setHasMessages(true);
    }
  }, [messages]);

  const handleSubmit = (e: CustomEvent<{ value: string }>) => {
    window.electron.startPowerSaveBlocker();
    const content = e.detail.value || '';
    if (content.trim()) {
      setLastInteractionTime(Date.now());
      append(createUserMessage(content));
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    }
  };

  if (error) {
    console.log('Error:', error);
  }

  const onStopGoose = async () => {
    stop();
    setLastInteractionTime(Date.now());
    window.electron.stopPowerSaveBlocker();

    const lastMessage = messages[messages.length - 1];
    const isToolResponse = lastMessage.content.some(
      (content): content is ToolResponseMessageContent => content.type === 'toolResponse'
    );

    if (lastMessage && isUserMessage(lastMessage) && !isToolResponse) {
      const textContent = lastMessage.content.find((c) => c.type === 'text')?.text || '';
      _setInput(textContent);

      if (messages.length > 1) {
        setMessages(messages.slice(0, -1));
      } else {
        setMessages([]);
      }
    } else if (!isUserMessage(lastMessage)) {
      const toolRequests: [string, ToolCallResult<ToolCall>][] = lastMessage.content
        .filter(
          (content): content is ToolRequestMessageContent | ToolConfirmationRequestMessageContent | FrontendToolRequestMessageContent =>
            content.type === 'toolRequest' || content.type === 'toolConfirmationRequest' || content.type === 'frontendToolRequest'
        )
        .map((content) => {
          if (content.type === 'toolRequest' || content.type === 'frontendToolRequest') {
            return [content.id, content.toolCall];
          } else {
            const toolCall: ToolCallResult<ToolCall> = {
              status: 'success',
              value: {
                name: content.toolName,
                arguments: content.arguments,
              },
            };
            return [content.id, toolCall];
          }
        });

      if (toolRequests.length !== 0) {
        let responseMessage: Message = {
          role: 'user',
          created: Date.now(),
          content: [],
        };

        const notification = 'Interrupted by the user to make a correction';

        for (const [reqId, toolCall] of toolRequests) {
          if (toolCall.status === 'success' && toolCall.value && 'name' in toolCall.value) {
            try {
              await executeFrontendTool(reqId, toolCall);
            } catch (error) {
              const toolResponse: ToolResponseMessageContent = {
                type: 'toolResponse',
                id: reqId,
                toolResult: {
                  status: 'error',
                  error: notification,
                },
              };
              responseMessage.content.push(toolResponse);
            }
          } else {
            const toolResponse: ToolResponseMessageContent = {
              type: 'toolResponse',
              id: reqId,
              toolResult: {
                status: 'error',
                error: notification,
              },
            };
            responseMessage.content.push(toolResponse);
          }
        }

        setMessages([...messages, responseMessage]);
      }
    }
  };

  const filteredMessages = messages.filter((message) => {
    if (message.role === 'assistant') return true;

    if (message.role === 'user') {
      const hasOnlyToolResponses = message.content.every((c) => c.type === 'toolResponse');
      const hasTextContent = message.content.some((c) => c.type === 'text');
      const hasToolConfirmation = message.content.every(
        (c) => c.type === 'toolConfirmationRequest'
      );

      return hasTextContent || !hasOnlyToolResponses || hasToolConfirmation;
    }

    return true;
  });

  const commandHistory = useMemo(() => {
    return filteredMessages
      .reduce<string[]>((history, message) => {
        if (isUserMessage(message)) {
          const text = message.content.find((c) => c.type === 'text')?.text?.trim();
          if (text) {
            history.push(text);
          }
        }
        return history;
      }, [])
      .reverse();
  }, [filteredMessages]);

  return (
    <div className="flex flex-col w-full h-screen items-center justify-center">
      <div className="relative flex items-center h-[36px] w-full">
        <MoreMenuLayout setView={setView} setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
      </div>

      <Card className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative">
        {messages.length === 0 ? (
          <Splash
            append={async (text: string) => append(createUserMessage(text))}
            activities={botConfig?.activities || null}
          />
        ) : (
          <ScrollArea ref={scrollRef} className="flex-1" autoScroll>
            <SearchView>
              {filteredMessages.map((message, index) => (
                <div key={message.id || index} className="mt-4 px-4">
                  {isUserMessage(message) ? (
                    <UserMessage message={message} />
                  ) : (
                    <GooseMessage
                      messageHistoryIndex={chat?.messageHistoryIndex}
                      message={message}
                      messages={messages}
                      metadata={null}
                      append={async (text: string) => append(createUserMessage(text))}
                      appendMessage={(newMessage: Message) => {
                        const updatedMessages = [...messages, newMessage];
                        setMessages(updatedMessages);
                      }}
                    />
                  )}
                </div>
              ))}
            </SearchView>
            {error && (
              <div className="flex flex-col items-center justify-center p-4">
                <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-3 rounded-lg mb-2">
                  {error.message || 'Honk! Goose experienced an error while responding'}
                </div>
                <div
                  className="px-3 py-2 mt-2 text-center whitespace-nowrap cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-full inline-block transition-all duration-150"
                  onClick={async () => {
                    const lastUserMessage = messages.reduceRight(
                      (found, m) => found || (m.role === 'user' ? m : null),
                      null as Message | null
                    );
                    if (lastUserMessage) {
                      append(lastUserMessage);
                    }
                  }}
                >
                  Retry Last Message
                </div>
              </div>
            )}
            <div className="block h-16" />
          </ScrollArea>
        )}

        <div className="relative">
          {isLoading && <LoadingGoose />}
          <Input
            handleSubmit={handleSubmit}
            isLoading={isLoading}
            onStop={onStopGoose}
            commandHistory={commandHistory}
            initialValue={_input}
          />
          <BottomMenu hasMessages={hasMessages} setView={setView} />
        </div>
      </Card>

      {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}

      {showShareableBotModal && generatedBotConfig && (
        <DeepLinkModal
          botConfig={generatedBotConfig as DeepLinkBotConfig}
          onClose={() => {
            setshowShareableBotModal(false);
            setGeneratedBotConfig(null);
          }}
        />
      )}
    </div>
  );
}
