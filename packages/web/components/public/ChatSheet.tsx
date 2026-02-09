"use client";

import { useState, useCallback, useEffect, useRef } from "react";
import { BottomSheet } from "@/components/public/BottomSheet";
import { ChatPanel } from "@/components/public/ChatPanel";
import { usePublicChatStream } from "@/lib/hooks/usePublicChatStream";
import { callService } from "@/lib/restate/client";
import type { PublicChatMessage, ChatroomResult, ChatMessage } from "@/lib/restate/types";

interface ChatSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

export function ChatSheet({ isOpen, onClose }: ChatSheetProps) {
  const [containerId, setContainerId] = useState<string | null>(null);
  const [messages, setMessages] = useState<PublicChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [isWaitingForReply, setIsWaitingForReply] = useState(false);
  const initRef = useRef(false);

  const loadMessages = useCallback(async (cid: string) => {
    try {
      const data = await callService<PublicChatMessage[]>("Chat", "get_messages", {
        chatroom_id: cid,
      });
      setMessages(data || []);
    } catch {
      // Silently fail
    }
  }, []);

  // SSE stream — reload messages when assistant replies
  usePublicChatStream(containerId, {
    onComplete: () => {
      setIsWaitingForReply(false);
      if (containerId) loadMessages(containerId);
    },
    onLagged: () => {
      if (containerId) loadMessages(containerId);
    },
  });

  // Create chat container when sheet opens
  useEffect(() => {
    if (!isOpen) return;
    if (containerId) return; // already have one

    // Guard against double-init in strict mode
    if (initRef.current) return;
    initRef.current = true;

    const create = async () => {
      try {
        const data = await callService<ChatroomResult>("Chats", "create", {
          language: "en",
          with_agent: "public",
        });
        setContainerId(data.id);
      } catch {
        // Creation failed — user can still see the sheet
      }
    };

    create();
  }, [isOpen, containerId]);

  // Reset everything when sheet closes
  useEffect(() => {
    if (!isOpen) {
      setContainerId(null);
      setMessages([]);
      setIsSending(false);
      setIsWaitingForReply(false);
      initRef.current = false;
    }
  }, [isOpen]);

  const handleSendMessage = useCallback(
    async (content: string) => {
      if (!containerId || isSending) return;

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
        const data = await callService<ChatMessage>("Chat", "send_message", {
          chatroom_id: containerId,
          content,
        });
        setMessages((prev) =>
          prev.map((m) => (m.id === optimistic.id ? data : m))
        );
        setIsWaitingForReply(true);
      } catch {
        setMessages((prev) => prev.filter((m) => m.id !== optimistic.id));
      } finally {
        setIsSending(false);
      }
    },
    [containerId, isSending]
  );

  const handleNewConversation = useCallback(async () => {
    setMessages([]);
    setContainerId(null);
    initRef.current = false;

    try {
      const data = await callService<ChatroomResult>("Chats", "create", {
        language: "en",
        with_agent: "public",
      });
      setContainerId(data.id);
      initRef.current = true;
    } catch {
      // Silently fail
    }
  }, []);

  return (
    <BottomSheet isOpen={isOpen} onClose={onClose}>
      <div className="h-[70vh] flex flex-col">
        <ChatPanel
          messages={messages}
          isWaitingForReply={isWaitingForReply}
          onSendMessage={handleSendMessage}
          onNewConversation={handleNewConversation}
          isSending={isSending}
        />
      </div>
    </BottomSheet>
  );
}
