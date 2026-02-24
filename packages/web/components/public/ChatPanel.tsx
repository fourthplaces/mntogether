"use client";

import { useState, useRef, useEffect } from "react";
import type { PublicChatMessage } from "@/lib/restate/types";

interface ChatPanelProps {
  messages: PublicChatMessage[];
  isWaitingForReply: boolean;
  onSendMessage: (content: string) => void;
  onNewConversation: () => void;
  isSending: boolean;
}

export function ChatPanel({
  messages,
  isWaitingForReply,
  onSendMessage,
  onNewConversation,
  isSending,
}: ChatPanelProps) {
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, isWaitingForReply]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = input.trim();
    if (!trimmed || isSending || isWaitingForReply) return;
    onSendMessage(trimmed);
    setInput("");
  };

  return (
    <div className="flex flex-col h-full bg-surface-raised border-t border-border">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border">
        <div className="flex items-center gap-2">
          <div className="h-2 w-2 bg-border-strong" />
          <span className="text-xs font-medium text-text-muted">
            MN Together Guide
          </span>
        </div>
        <button
          onClick={onNewConversation}
          className="text-xs text-text-muted hover:text-text-primary transition-colors"
        >
          New conversation
        </button>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3 min-h-0">
        {messages.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${
              msg.sender_type === "user" ? "justify-end" : "justify-start"
            }`}
          >
            <div
              className={`max-w-[80%] px-3 py-2 rounded-2xl text-sm ${
                msg.sender_type === "user"
                  ? "bg-action text-text-on-action"
                  : "bg-surface-muted text-text-primary"
              }`}
            >
              {msg.content}
            </div>
          </div>
        ))}

        {/* Waiting indicator */}
        {isWaitingForReply && (
          <div className="flex justify-start">
            <div className="max-w-[80%] px-3 py-2 rounded-2xl rounded-bl-md bg-surface-muted text-text-primary text-sm">
              <span className="inline-flex gap-1">
                <span className="w-1.5 h-1.5 bg-border-strong rounded-full animate-bounce" />
                <span
                  className="w-1.5 h-1.5 bg-border-strong rounded-full animate-bounce"
                  style={{ animationDelay: "0.1s" }}
                />
                <span
                  className="w-1.5 h-1.5 bg-border-strong rounded-full animate-bounce"
                  style={{ animationDelay: "0.2s" }}
                />
              </span>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <form
        onSubmit={handleSubmit}
        className="px-4 py-3 border-t border-border"
      >
        <div className="flex items-center gap-2">
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Ask about services, housing, food help..."
            disabled={isSending || isWaitingForReply}
            className="flex-1 px-4 py-2.5 bg-surface-muted border border-border rounded-xl text-sm text-text-primary placeholder-text-muted focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:opacity-50"
          />
          <button
            type="submit"
            disabled={!input.trim() || isSending || isWaitingForReply}
            className="px-3 py-2 bg-action text-text-on-action text-sm font-semibold hover:bg-action-hover disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Send
          </button>
        </div>
      </form>
    </div>
  );
}
