"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { ContentPanel } from "@/components/public/ContentPanel";
import { ChatPanel } from "@/components/public/ChatPanel";
import { usePublicChatStream } from "@/lib/hooks/usePublicChatStream";
import { callService } from "@/lib/restate/client";
import type { PublicChatMessage, ChatroomResult, ChatMessage } from "@/lib/restate/types";

const STORAGE_KEY = "mnt_public_chat_container_id";

export function PublicChatClient() {
  const [containerId, setContainerId] = useState<string | null>(null);
  const [messages, setMessages] = useState<PublicChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [isWaitingForReply, setIsWaitingForReply] = useState(false);
  const [isInitializing, setIsInitializing] = useState(true);
  const initRef = useRef(false);

  // Connect to public SSE stream — notifies when assistant reply is ready
  usePublicChatStream(containerId, {
    onComplete: () => {
      setIsWaitingForReply(false);
      if (containerId) {
        loadMessages(containerId);
      }
    },
    onLagged: () => {
      if (containerId) {
        loadMessages(containerId);
      }
    },
  });

  const loadMessages = useCallback(async (cid: string) => {
    try {
      const data = await callService<PublicChatMessage[]>("Chat", "get_messages", {
        chatroom_id: cid,
      });
      setMessages(data || []);
    } catch {
      // Silently fail — user can still send new messages
    }
  }, []);

  const createChat = useCallback(async (): Promise<string> => {
    const data = await callService<ChatroomResult>("Chats", "create", {
      language: "en",
      with_agent: "public",
    });
    const id = data.id;
    localStorage.setItem(STORAGE_KEY, id);
    return id;
  }, []);

  // Initialize session
  useEffect(() => {
    if (initRef.current) return;
    initRef.current = true;

    const init = async () => {
      try {
        const existingId = localStorage.getItem(STORAGE_KEY);
        if (existingId) {
          setContainerId(existingId);
          await loadMessages(existingId);
        } else {
          const newId = await createChat();
          setContainerId(newId);
        }
      } catch {
        // If loading fails, create a fresh chat
        try {
          const newId = await createChat();
          setContainerId(newId);
        } catch {
          // Total failure — still show the UI
        }
      } finally {
        setIsInitializing(false);
      }
    };

    init();
  }, [createChat, loadMessages]);

  const handleSendMessage = useCallback(
    async (content: string) => {
      if (!containerId || isSending) return;

      // Optimistic local message
      const optimistic: PublicChatMessage = {
        id: `optimistic-${Date.now()}`,
        chatroom_id: containerId,
        sender_type: "user",
        content,
        created_at: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, optimistic]);
      setIsSending(true);

      try {
        const data = await callService<ChatMessage>(
          "Chat", "send_message",
          { chatroom_id: containerId, content }
        );
        // Replace optimistic with real message
        setMessages((prev) =>
          prev.map((m) => (m.id === optimistic.id ? data : m))
        );
        setIsWaitingForReply(true);
      } catch {
        // Remove optimistic message on failure
        setMessages((prev) => prev.filter((m) => m.id !== optimistic.id));
      } finally {
        setIsSending(false);
      }
    },
    [containerId, isSending]
  );

  const handleNewConversation = useCallback(async () => {
    try {
      localStorage.removeItem(STORAGE_KEY);
      setMessages([]);
      const newId = await createChat();
      setContainerId(newId);
    } catch {
      // Silently fail
    }
  }, [createChat]);

  const handleSuggestedPrompt = useCallback(
    (query: string) => {
      handleSendMessage(query);
    },
    [handleSendMessage]
  );

  if (isInitializing) {
    return (
      <div className="flex items-center justify-center h-screen bg-gradient-to-b from-blue-50 to-white">
        <div className="text-center">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            MN Together
          </h1>
          <p className="text-gray-500">Loading...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-gradient-to-b from-blue-50 to-white">
      {/* Content Panel — top, flexible */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        <ContentPanel onSuggestedPrompt={handleSuggestedPrompt} />
      </div>

      {/* Chat Panel — bottom, fixed height */}
      <div className="h-[40vh] min-h-[280px] max-h-[400px] flex-shrink-0">
        <ChatPanel
          messages={messages}
          isWaitingForReply={isWaitingForReply}
          onSendMessage={handleSendMessage}
          onNewConversation={handleNewConversation}
          isSending={isSending}
        />
      </div>
    </div>
  );
}
