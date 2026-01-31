import { useState, useRef, useEffect } from 'react';
import { useMutation, useQuery } from '@apollo/client';
import { CREATE_CHAT, SEND_MESSAGE } from '../graphql/mutations';
import { GET_MESSAGES, GET_RECENT_CHATS } from '../graphql/queries';

interface Message {
  id: string;
  containerId: string;
  role: string;
  content: string;
  authorId?: string;
  createdAt: string;
}

interface Container {
  id: string;
  containerType: string;
  language: string;
  lastActivityAt: string;
}

interface ChatroomProps {
  isOpen: boolean;
  onClose: () => void;
}

export function Chatroom({ isOpen, onClose }: ChatroomProps) {
  const [containerId, setContainerId] = useState<string | null>(null);
  const [input, setInput] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Fetch recent chats to restore session
  const { data: recentChatsData } = useQuery<{ recentChats: Container[] }>(
    GET_RECENT_CHATS,
    { variables: { limit: 1 } }
  );

  // Fetch messages when container is selected
  const { data: messagesData, refetch: refetchMessages } = useQuery<{ messages: Message[] }>(
    GET_MESSAGES,
    {
      variables: { containerId },
      skip: !containerId,
      pollInterval: 2000, // Poll for new messages (replace with subscriptions later)
    }
  );

  // Mutations
  const [createChat, { loading: creatingChat }] = useMutation(CREATE_CHAT);
  const [sendMessage, { loading: sendingMessage }] = useMutation(SEND_MESSAGE);

  // Restore last chat session on mount
  useEffect(() => {
    if (recentChatsData?.recentChats?.[0]) {
      setContainerId(recentChatsData.recentChats[0].id);
    }
  }, [recentChatsData]);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messagesData?.messages]);

  // Start new chat
  const handleStartNewChat = async () => {
    try {
      const { data } = await createChat({ variables: { language: 'en' } });
      if (data?.createChat?.id) {
        setContainerId(data.createChat.id);
      }
    } catch (error) {
      console.error('Failed to create chat:', error);
    }
  };

  // Send message
  const handleSendMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim() || !containerId || sendingMessage) return;

    const messageContent = input.trim();
    setInput('');
    setIsTyping(true);

    try {
      await sendMessage({
        variables: { containerId, content: messageContent },
      });
      // Refetch to get the new message and any AI response
      setTimeout(() => {
        refetchMessages();
        setIsTyping(false);
      }, 500);
    } catch (error) {
      console.error('Failed to send message:', error);
      setIsTyping(false);
    }
  };

  // Format timestamp
  const formatTime = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-y-0 right-0 w-96 bg-white shadow-xl border-l border-stone-200 flex flex-col z-50">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-stone-200 bg-amber-50">
        <div className="flex items-center gap-2">
          <span className="text-xl">💬</span>
          <h2 className="font-semibold text-stone-900">Admin Assistant</h2>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleStartNewChat}
            disabled={creatingChat}
            className="text-stone-500 hover:text-stone-700 text-sm px-2 py-1 rounded hover:bg-stone-100"
            title="New chat"
          >
            {creatingChat ? '...' : '+ New'}
          </button>
          <button
            onClick={onClose}
            className="text-stone-500 hover:text-stone-700 p-1 rounded hover:bg-stone-100"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {!containerId ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <span className="text-4xl mb-4">🤖</span>
            <h3 className="font-medium text-stone-900 mb-2">
              Admin Assistant
            </h3>
            <p className="text-sm text-stone-600 mb-4">
              I can help you manage websites, approve listings, run scrapers,
              and more.
            </p>
            <button
              onClick={handleStartNewChat}
              disabled={creatingChat}
              className="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors disabled:opacity-50"
            >
              {creatingChat ? 'Starting...' : 'Start Chat'}
            </button>
          </div>
        ) : (
          <>
            {messagesData?.messages?.map((message) => (
              <div
                key={message.id}
                className={`flex ${
                  message.role === 'user' ? 'justify-end' : 'justify-start'
                }`}
              >
                <div
                  className={`max-w-[80%] rounded-lg px-4 py-2 ${
                    message.role === 'user'
                      ? 'bg-amber-500 text-white'
                      : 'bg-stone-100 text-stone-900'
                  }`}
                >
                  <p className="text-sm whitespace-pre-wrap">{message.content}</p>
                  <p
                    className={`text-xs mt-1 ${
                      message.role === 'user'
                        ? 'text-amber-200'
                        : 'text-stone-400'
                    }`}
                  >
                    {formatTime(message.createdAt)}
                  </p>
                </div>
              </div>
            ))}

            {/* Typing indicator */}
            {isTyping && (
              <div className="flex justify-start">
                <div className="bg-stone-100 text-stone-900 rounded-lg px-4 py-2">
                  <div className="flex space-x-1">
                    <div className="w-2 h-2 bg-stone-400 rounded-full animate-bounce" />
                    <div
                      className="w-2 h-2 bg-stone-400 rounded-full animate-bounce"
                      style={{ animationDelay: '0.1s' }}
                    />
                    <div
                      className="w-2 h-2 bg-stone-400 rounded-full animate-bounce"
                      style={{ animationDelay: '0.2s' }}
                    />
                  </div>
                </div>
              </div>
            )}

            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Input */}
      {containerId && (
        <form
          onSubmit={handleSendMessage}
          className="border-t border-stone-200 p-4"
        >
          <div className="flex gap-2">
            <input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="Type a message..."
              className="flex-1 px-4 py-2 border border-stone-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
              disabled={sendingMessage}
            />
            <button
              type="submit"
              disabled={!input.trim() || sendingMessage}
              className="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <svg
                className="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
                />
              </svg>
            </button>
          </div>
        </form>
      )}

      {/* Quick Actions for Admin */}
      {containerId && (
        <div className="border-t border-stone-200 p-3 bg-stone-50">
          <p className="text-xs text-stone-500 mb-2">Quick actions:</p>
          <div className="flex flex-wrap gap-1">
            {[
              'Show pending websites',
              'Scrape a URL',
              'Run agent search',
              'System status',
            ].map((action) => (
              <button
                key={action}
                onClick={() => setInput(action)}
                className="text-xs px-2 py-1 bg-white border border-stone-200 rounded-full text-stone-600 hover:bg-stone-100 hover:border-stone-300"
              >
                {action}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
