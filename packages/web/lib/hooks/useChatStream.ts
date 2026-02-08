"use client";

import { useState, useEffect, useRef } from "react";

const SSE_BASE =
  process.env.NEXT_PUBLIC_SSE_URL || "http://localhost:8081";

interface UseChatStreamOptions {
  /** Called when a new message is available — use to trigger refetch */
  onComplete?: () => void;
  /** Called when client falls behind broadcast — use to trigger refetch */
  onLagged?: () => void;
}

/**
 * Hook that connects to the SSE endpoint for a chat container.
 *
 * Listens for `message_complete` events and signals the parent to refetch.
 */
export function useChatStream(
  containerId: string | null,
  options: UseChatStreamOptions = {}
) {
  const [isConnected, setIsConnected] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);
  const onCompleteRef = useRef(options.onComplete);
  const onLaggedRef = useRef(options.onLagged);

  onCompleteRef.current = options.onComplete;
  onLaggedRef.current = options.onLagged;

  useEffect(() => {
    if (!containerId) return;

    const url = `${SSE_BASE}/api/streams/chat:${containerId}`;
    const es = new EventSource(url);
    eventSourceRef.current = es;

    es.addEventListener("message_complete", () => {
      onCompleteRef.current?.();
    });

    es.addEventListener("lagged", () => {
      onLaggedRef.current?.();
    });

    es.onopen = () => setIsConnected(true);
    es.onerror = () => setIsConnected(false);

    return () => {
      es.close();
      eventSourceRef.current = null;
      setIsConnected(false);
    };
  }, [containerId]);

  return { isConnected };
}
