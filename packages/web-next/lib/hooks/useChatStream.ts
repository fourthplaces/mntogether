"use client";

import { useState, useEffect, useRef } from "react";

// Derive SSE base URL from the GraphQL API URL (strip /graphql suffix)
const API_BASE = (
  process.env.NEXT_PUBLIC_API_URL || "http://100.110.4.74:8080/graphql"
).replace(/\/graphql$/, "");

interface StreamingMessage {
  content: string;
  isStreaming: boolean;
  messageId?: string;
  error?: string;
}

interface UseChatStreamOptions {
  /** Called when generation completes — use to trigger SWR refetch */
  onComplete?: () => void;
  /** Called when client falls behind broadcast — use to trigger SWR refetch */
  onLagged?: () => void;
}

/**
 * Read auth_token from document.cookie.
 * The cookie is httpOnly: false so JS can read it.
 */
function getAuthToken(): string | null {
  if (typeof document === "undefined") return null;
  const match = document.cookie.match(/(?:^|; )auth_token=([^;]*)/);
  return match ? decodeURIComponent(match[1]) : null;
}

/**
 * Hook that connects to the SSE streaming endpoint for a chat container.
 *
 * Opens EventSource to GET /api/streams/chat:{containerId}?token=JWT
 * Accumulates token deltas into a streaming message.
 * Returns null when no generation is in progress.
 */
export function useChatStream(
  containerId: string | null,
  options: UseChatStreamOptions = {}
) {
  const [streamingMessage, setStreamingMessage] =
    useState<StreamingMessage | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);
  const onCompleteRef = useRef(options.onComplete);
  const onLaggedRef = useRef(options.onLagged);

  // Keep refs current
  onCompleteRef.current = options.onComplete;
  onLaggedRef.current = options.onLagged;

  useEffect(() => {
    if (!containerId) return;

    const token = getAuthToken();
    if (!token) {
      console.warn("useChatStream: no auth_token cookie, skipping SSE connection");
      return;
    }

    const url = `${API_BASE}/api/streams/chat:${containerId}?token=${encodeURIComponent(token)}`;
    const es = new EventSource(url);
    eventSourceRef.current = es;

    es.addEventListener("connected", () => {
      setIsConnected(true);
    });

    es.addEventListener("generation_started", () => {
      setStreamingMessage({ content: "", isStreaming: true });
    });

    es.addEventListener("token_delta", (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data);
        setStreamingMessage((prev) => ({
          content: (prev?.content || "") + (data.delta || ""),
          isStreaming: true,
        }));
      } catch {
        // Ignore parse errors for individual tokens
      }
    });

    es.addEventListener("message_complete", (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data);
        setStreamingMessage({
          content: data.content,
          isStreaming: false,
          messageId: data.message_id,
        });
        // Signal completion so parent can refetch from DB
        onCompleteRef.current?.();
        // Clear streaming message after a brief delay
        setTimeout(() => setStreamingMessage(null), 100);
      } catch {
        setStreamingMessage(null);
      }
    });

    es.addEventListener("generation_error", (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data);
        setStreamingMessage({
          content: "",
          isStreaming: false,
          error: data.error,
        });
        setTimeout(() => setStreamingMessage(null), 5000);
      } catch {
        setStreamingMessage(null);
      }
    });

    es.addEventListener("lagged", () => {
      // Client fell behind — refetch from DB
      onLaggedRef.current?.();
    });

    es.onerror = () => {
      setIsConnected(false);
      // EventSource auto-reconnects
    };

    return () => {
      es.close();
      eventSourceRef.current = null;
      setIsConnected(false);
      setStreamingMessage(null);
    };
  }, [containerId]);

  return { streamingMessage, isConnected };
}
