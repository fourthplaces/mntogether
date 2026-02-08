"use client";

import { useState, useEffect, useRef } from "react";

const SSE_BASE =
  process.env.NEXT_PUBLIC_SSE_URL || "http://localhost:8081";

interface UsePublicChatStreamOptions {
  /** Called when a new message is available */
  onComplete?: () => void;
  /** Called when client falls behind broadcast */
  onLagged?: () => void;
}

/**
 * Hook that connects to the public SSE endpoint for a chat container.
 *
 * Listens for `message_complete` events and signals the parent to refetch.
 */
export function usePublicChatStream(
  containerId: string | null,
  options: UsePublicChatStreamOptions = {}
) {
  const [isConnected, setIsConnected] = useState(false);
  const onCompleteRef = useRef(options.onComplete);
  const onLaggedRef = useRef(options.onLagged);

  onCompleteRef.current = options.onComplete;
  onLaggedRef.current = options.onLagged;

  useEffect(() => {
    if (!containerId) return;

    const url = `${SSE_BASE}/api/streams/chat:${containerId}`;
    const es = new EventSource(url);

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
      setIsConnected(false);
    };
  }, [containerId]);

  return { isConnected };
}
