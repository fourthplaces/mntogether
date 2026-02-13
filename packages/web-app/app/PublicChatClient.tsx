"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { ContentPanel } from "@/components/ContentPanel";
import { ChatPanel } from "@/components/ChatPanel";
import { usePublicChatStream } from "@/lib/hooks/usePublicChatStream";
import { useMutation, useClient } from "urql";
import { CreateChatMutation, SendChatMessageMutation, ChatMessagesQuery } from "@/lib/graphql/chat";

interface ChatMessage {
  id: string;
  chatroomId: string;
  senderType: string;
  content: string;
  createdAt: string;
}

const STORAGE_KEY = "mnt_public_chat_container_id";

export function PublicChatClient() {
  const [containerId, setContainerId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [isWaitingForReply, setIsWaitingForReply] = useState(false);
  const [isInitializing, setIsInitializing] = useState(true);
  const initRef = useRef(false);
  const client = useClient();
  const [, createChatMut] = useMutation(CreateChatMutation);
  const [, sendMessageMut] = useMutation(SendChatMessageMutation);

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
      const result = await client.query(ChatMessagesQuery, { chatroomId: cid }).toPromise();
      if (result.data?.chatMessages) {
        setMessages(result.data.chatMessages);
      }
    } catch {
      // Silently fail — user can still send new messages
    }
  }, [client]);

  const createChat = useCallback(async (): Promise<string> => {
    const result = await createChatMut({ language: "en", withAgent: "public" });
    if (result.error) throw result.error;
    const id = result.data!.createChat.id;
    localStorage.setItem(STORAGE_KEY, id);
    return id;
  }, [createChatMut]);

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
      const optimistic: ChatMessage = {
        id: `optimistic-${Date.now()}`,
        chatroomId: containerId,
        senderType: "user",
        content,
        createdAt: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, optimistic]);
      setIsSending(true);

      try {
        const result = await sendMessageMut({
          chatroomId: containerId,
          content,
        });
        if (result.error) throw result.error;
        const msg = result.data!.sendChatMessage;
        // Replace optimistic with real message
        setMessages((prev) =>
          prev.map((m) => (m.id === optimistic.id ? msg : m))
        );
        setIsWaitingForReply(true);
      } catch {
        // Remove optimistic message on failure
        setMessages((prev) => prev.filter((m) => m.id !== optimistic.id));
      } finally {
        setIsSending(false);
      }
    },
    [containerId, isSending, sendMessageMut]
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
