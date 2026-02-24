"use client";

import { useState, useCallback, useEffect, useRef } from "react";
import { BottomSheet } from "@/components/BottomSheet";
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

interface ChatSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

export function ChatSheet({ isOpen, onClose }: ChatSheetProps) {
  const [containerId, setContainerId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [isWaitingForReply, setIsWaitingForReply] = useState(false);
  const initRef = useRef(false);
  const client = useClient();
  const [, createChatMut] = useMutation(CreateChatMutation);
  const [, sendMessageMut] = useMutation(SendChatMessageMutation);

  const loadMessages = useCallback(async (cid: string) => {
    try {
      const result = await client.query(ChatMessagesQuery, { chatroomId: cid }).toPromise();
      if (result.data?.chatMessages) {
        setMessages(result.data.chatMessages);
      }
    } catch {
      // Silently fail
    }
  }, [client]);

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
        const result = await createChatMut({ language: "en", withAgent: "public" });
        if (result.error) throw result.error;
        setContainerId(result.data!.createChat.id);
      } catch {
        // Creation failed — user can still see the sheet
      }
    };

    create();
  }, [isOpen, containerId, createChatMut]);

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
        setMessages((prev) =>
          prev.map((m) => (m.id === optimistic.id ? msg : m))
        );
        setIsWaitingForReply(true);
      } catch {
        setMessages((prev) => prev.filter((m) => m.id !== optimistic.id));
      } finally {
        setIsSending(false);
      }
    },
    [containerId, isSending, sendMessageMut]
  );

  const handleNewConversation = useCallback(async () => {
    setMessages([]);
    setContainerId(null);
    initRef.current = false;

    try {
      const result = await createChatMut({ language: "en", withAgent: "public" });
      if (result.error) throw result.error;
      setContainerId(result.data!.createChat.id);
      initRef.current = true;
    } catch {
      // Silently fail
    }
  }, [createChatMut]);

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
