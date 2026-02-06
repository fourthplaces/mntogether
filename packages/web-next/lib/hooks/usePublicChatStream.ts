"use client";

import { useState, useEffect, useRef, useCallback } from "react";

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

export interface ToolResult {
  tool_name: string;
  call_id: string;
  container_id: string;
  results: unknown[];
}

interface UsePublicChatStreamOptions {
  /** Called when generation completes */
  onComplete?: () => void;
  /** Called when client falls behind broadcast */
  onLagged?: () => void;
}

/**
 * Hook that connects to the public SSE streaming endpoint for a chat container.
 *
 * Opens EventSource to GET /api/streams/public_chat:{containerId} (no JWT).
 * Accumulates token deltas into a streaming message.
 * Listens for tool_result events and exposes them as state.
 */
export function usePublicChatStream(
  containerId: string | null,
  options: UsePublicChatStreamOptions = {}
) {
  const [streamingMessage, setStreamingMessage] =
    useState<StreamingMessage | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [toolResults, setToolResults] = useState<ToolResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);
  const onCompleteRef = useRef(options.onComplete);
  const onLaggedRef = useRef(options.onLagged);

  // Keep refs current
  onCompleteRef.current = options.onComplete;
  onLaggedRef.current = options.onLagged;

  const clearToolResults = useCallback(() => {
    setToolResults([]);
    setIsSearching(false);
  }, []);

  useEffect(() => {
    if (!containerId) return;

    // Public SSE: no token needed
    const url = `${API_BASE}/api/streams/public_chat:${containerId}`;
    const es = new EventSource(url);
    eventSourceRef.current = es;

    es.addEventListener("connected", () => {
      setIsConnected(true);
    });

    es.addEventListener("generation_started", () => {
      setStreamingMessage({ content: "", isStreaming: true });
      // Clear previous tool results when a new generation starts
      setToolResults([]);
      setIsSearching(true);
    });

    es.addEventListener("tool_result", (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data);
        setToolResults((prev) => [...prev, data as ToolResult]);
        setIsSearching(false);
      } catch {
        // Ignore parse errors
      }
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
        setIsSearching(false);
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
        setIsSearching(false);
        setTimeout(() => setStreamingMessage(null), 5000);
      } catch {
        setStreamingMessage(null);
      }
    });

    es.addEventListener("lagged", () => {
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

  return { streamingMessage, isConnected, toolResults, isSearching, clearToolResults };
}
