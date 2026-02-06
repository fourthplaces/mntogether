"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { ContentPanel } from "@/components/public/ContentPanel";
import { ChatPanel } from "@/components/public/ChatPanel";
import { usePublicChatStream } from "@/lib/hooks/usePublicChatStream";
import { graphqlFetchClient } from "@/lib/graphql/client";
import { CREATE_CHAT, SEND_MESSAGE } from "@/lib/graphql/mutations";
import { GET_MESSAGES } from "@/lib/graphql/queries";
import type {
  ChatMessage,
  CreateChatResult,
  SendMessageResult,
  GetMessagesResult,
} from "@/lib/types";

const STORAGE_KEY = "mnt_public_chat_container_id";

export function PublicChatClient() {
  const [containerId, setContainerId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [isInitializing, setIsInitializing] = useState(true);
  const initRef = useRef(false);

  // Connect to public SSE stream
  const { streamingMessage, toolResults, isSearching } = usePublicChatStream(
    containerId,
    {
      onComplete: () => {
        // Refetch messages from DB when generation completes
        if (containerId) {
          loadMessages(containerId);
        }
      },
      onLagged: () => {
        if (containerId) {
          loadMessages(containerId);
        }
      },
    }
  );

  const loadMessages = useCallback(async (cid: string) => {
    try {
      const data = await graphqlFetchClient<GetMessagesResult>(GET_MESSAGES, {
        containerId: cid,
      });
      setMessages(data.messages || []);
    } catch {
      // Silently fail — user can still send new messages
    }
  }, []);

  const createChat = useCallback(async (): Promise<string> => {
    const data = await graphqlFetchClient<CreateChatResult>(CREATE_CHAT, {
      language: "en",
      withAgent: "public",
    });
    const id = data.createChat.id;
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
      const optimistic: ChatMessage = {
        id: `optimistic-${Date.now()}`,
        containerId,
        role: "user",
        content,
        createdAt: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, optimistic]);
      setIsSending(true);

      try {
        const data = await graphqlFetchClient<SendMessageResult>(
          SEND_MESSAGE,
          { containerId, content }
        );
        // Replace optimistic with real message
        setMessages((prev) =>
          prev.map((m) => (m.id === optimistic.id ? data.sendMessage : m))
        );
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
        <ContentPanel
          toolResults={toolResults}
          isSearching={isSearching}
          onSuggestedPrompt={handleSuggestedPrompt}
        />
      </div>

      {/* Chat Panel — bottom, fixed height */}
      <div className="h-[40vh] min-h-[280px] max-h-[400px] flex-shrink-0">
        <ChatPanel
          messages={messages}
          streamingContent={
            streamingMessage ? streamingMessage.content : null
          }
          isStreaming={streamingMessage?.isStreaming ?? false}
          onSendMessage={handleSendMessage}
          onNewConversation={handleNewConversation}
          isSending={isSending}
        />
      </div>
    </div>
  );
}
