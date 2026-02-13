"use client";

import { useState, useRef, useEffect, useCallback } from "react";
import { callService, useRestate, useRestateObject } from "@/lib/restate/client";
import { useChatStream } from "@/lib/hooks/useChatStream";
import type { ChatMessage, ChatroomResult } from "@/lib/restate/types";

interface ChatroomProps {
  isOpen: boolean;
  onClose: () => void;
  withAgent?: string;
}

export function Chatroom({ isOpen, onClose, withAgent = "admin" }: ChatroomProps) {
  const [containerId, setContainerId] = useState<string | null>(null);
  const [input, setInput] = useState("");
  const [isSending, setIsSending] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [isWaitingForReply, setIsWaitingForReply] = useState(false);
  const [autoStarted, setAutoStarted] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Fetch recent chats to restore session
  const { data: recentChatsData, isLoading: loadingRecent } = useRestate<{ chats: ChatroomResult[] }>(
    "Chats", "list_recent", { limit: 1 },
    { revalidateOnFocus: false }
  );
  const recentChats = recentChatsData?.chats;

  // Fetch messages when container is selected (no polling — streaming replaces it)
  const { data: messagesData, mutate: refetchMessages } = useRestateObject<{ messages: ChatMessage[] }>(
    "Chat", containerId, "get_messages", {},
    { revalidateOnFocus: false }
  );
  const messages = messagesData?.messages;

  // SSE connection — notifies when assistant reply is ready
  useChatStream(containerId, {
    onComplete: () => {
      setIsWaitingForReply(false);
      refetchMessages();
    },
    onLagged: () => refetchMessages(),
  });

  // Start new chat with agent
  const handleStartNewChat = useCallback(async () => {
    setIsCreating(true);
    try {
      const data = await callService<ChatroomResult>("Chats", "create", {
        language: "en",
        with_agent: withAgent || undefined,
      });
      if (data?.id) {
        setContainerId(data.id);
      }
    } catch (error) {
      console.error("Failed to create chat:", error);
    } finally {
      setIsCreating(false);
    }
  }, [withAgent]);

  // Restore last chat session or auto-start new one when panel opens
  useEffect(() => {
    if (!isOpen || loadingRecent || autoStarted) return;

    if (recentChats?.[0]) {
      setContainerId(recentChats[0].id);
      setAutoStarted(true);
    } else if (!isCreating) {
      setAutoStarted(true);
      handleStartNewChat();
    }
  }, [isOpen, recentChats, loadingRecent, autoStarted, isCreating, handleStartNewChat]);

  // Reset auto-started state when panel closes
  useEffect(() => {
    if (!isOpen) {
      setAutoStarted(false);
    }
  }, [isOpen]);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, isWaitingForReply]);

  // Send message
  const handleSendMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim() || !containerId || isSending) return;

    const messageContent = input.trim();
    setInput("");
    setIsSending(true);

    try {
      await callService<ChatMessage>("Chat", "send_message", {
        chatroom_id: containerId,
        content: messageContent,
      });
      setIsWaitingForReply(true);
      refetchMessages();
    } catch (error) {
      console.error("Failed to send message:", error);
    } finally {
      setIsSending(false);
    }
  };

  const formatTime = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  if (!isOpen) return null;

  const messageList = messages || [];

  return (
    <div className="fixed inset-y-0 right-0 w-96 bg-white shadow-xl border-l border-stone-200 flex flex-col z-50">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-stone-200 bg-amber-50">
        <div className="flex items-center gap-2">
          <span className="text-xl">{"\u{1F4AC}"}</span>
          <h2 className="font-semibold text-stone-900">Admin Assistant</h2>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleStartNewChat}
            disabled={isCreating}
            className="text-stone-500 hover:text-stone-700 text-sm px-2 py-1 rounded hover:bg-stone-100"
            title="New chat"
          >
            {isCreating ? "..." : "+ New"}
          </button>
          <button
            onClick={onClose}
            className="text-stone-500 hover:text-stone-700 p-1 rounded hover:bg-stone-100"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
        {!containerId || isCreating || loadingRecent ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <div className="flex space-x-2 mb-4">
              <div className="w-3 h-3 bg-amber-400 rounded-full animate-bounce" />
              <div
                className="w-3 h-3 bg-amber-400 rounded-full animate-bounce"
                style={{ animationDelay: "0.1s" }}
              />
              <div
                className="w-3 h-3 bg-amber-400 rounded-full animate-bounce"
                style={{ animationDelay: "0.2s" }}
              />
            </div>
            <p className="text-sm text-stone-500">Starting assistant...</p>
          </div>
        ) : (
          <>
            {messageList.map((message) => (
              <div
                key={message.id}
                className={`flex ${message.sender_type === "user" ? "justify-end" : "justify-start"}`}
              >
                <div
                  className={`max-w-[80%] rounded-lg px-4 py-2 ${
                    message.sender_type === "user"
                      ? "bg-amber-500 text-white"
                      : "bg-stone-100 text-stone-900"
                  }`}
                >
                  <p className="text-sm whitespace-pre-wrap">{message.content}</p>
                  <p
                    className={`text-xs mt-1 ${
                      message.sender_type === "user" ? "text-amber-200" : "text-stone-400"
                    }`}
                  >
                    {formatTime(message.created_at)}
                  </p>
                </div>
              </div>
            ))}

            {/* Waiting for reply */}
            {isWaitingForReply && (
              <div className="flex justify-start">
                <div className="bg-stone-100 text-stone-900 rounded-lg px-4 py-2 max-w-[80%]">
                  <div className="flex space-x-1">
                    <div className="w-2 h-2 bg-stone-400 rounded-full animate-bounce" />
                    <div
                      className="w-2 h-2 bg-stone-400 rounded-full animate-bounce"
                      style={{ animationDelay: "0.1s" }}
                    />
                    <div
                      className="w-2 h-2 bg-stone-400 rounded-full animate-bounce"
                      style={{ animationDelay: "0.2s" }}
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
        <form onSubmit={handleSendMessage} className="border-t border-stone-200 p-4">
          <div className="flex gap-2">
            <input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="Type a message..."
              className="flex-1 px-4 py-2 border border-stone-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
              disabled={isSending}
            />
            <button
              type="submit"
              disabled={!input.trim() || isSending}
              className="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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

      {/* Quick Actions */}
      {containerId && (
        <div className="border-t border-stone-200 p-3 bg-stone-50">
          <p className="text-xs text-stone-500 mb-2">Quick actions:</p>
          <div className="flex flex-wrap gap-1">
            {["Show pending websites", "Scrape a URL", "Run agent search", "System status"].map(
              (action) => (
                <button
                  key={action}
                  onClick={() => setInput(action)}
                  className="text-xs px-2 py-1 bg-white border border-stone-200 rounded-full text-stone-600 hover:bg-stone-100 hover:border-stone-300"
                >
                  {action}
                </button>
              )
            )}
          </div>
        </div>
      )}
    </div>
  );
}
