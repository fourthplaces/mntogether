"use client";

import { useState, useRef, useCallback, useEffect } from "react";

export type SplitMode = "editor" | "split" | "preview";

interface SplitPaneProps {
  left: React.ReactNode;
  right: React.ReactNode;
  mode: SplitMode;
}

const STORAGE_KEY = "editor-split-ratio";
const DEFAULT_RATIO = 0.5;
const MIN_RATIO = 0.25;
const MAX_RATIO = 0.75;

function loadRatio(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const val = parseFloat(raw);
      if (!isNaN(val) && val >= MIN_RATIO && val <= MAX_RATIO) return val;
    }
  } catch {
    // ignore
  }
  return DEFAULT_RATIO;
}

export function SplitPane({ left, right, mode }: SplitPaneProps) {
  const [ratio, setRatio] = useState(DEFAULT_RATIO);
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Hydrate ratio from localStorage after mount
  useEffect(() => {
    setRatio(loadRatio());
  }, []);

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      e.preventDefault();
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      setIsDragging(true);
    },
    []
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!isDragging || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const newRatio = Math.min(MAX_RATIO, Math.max(MIN_RATIO, x / rect.width));
      setRatio(newRatio);
    },
    [isDragging]
  );

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      if (!isDragging) return;
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
      setIsDragging(false);
      try {
        localStorage.setItem(STORAGE_KEY, ratio.toString());
      } catch {
        // ignore
      }
    },
    [isDragging, ratio]
  );

  // Editor-only mode
  if (mode === "editor") {
    return (
      <div className="flex-1 overflow-y-auto">{left}</div>
    );
  }

  // Preview-only mode
  if (mode === "preview") {
    return (
      <div className="flex-1 overflow-y-auto">{right}</div>
    );
  }

  // Split mode
  return (
    <div
      ref={containerRef}
      className={`flex-1 min-h-0 ${isDragging ? "select-none" : ""}`}
      style={{
        display: "grid",
        gridTemplateColumns: `${ratio}fr 4px ${1 - ratio}fr`,
      }}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    >
      {/* Left pane — editor */}
      <div className="overflow-y-auto min-w-0">{left}</div>

      {/* Drag handle */}
      <div
        className={`cursor-col-resize flex items-center justify-center transition-colors duration-150 ${
          isDragging
            ? "bg-admin-accent"
            : "bg-border-subtle hover:bg-border-strong"
        }`}
        onPointerDown={handlePointerDown}
      >
        {/* Grip dots */}
        <div className="flex flex-col gap-1">
          <div className={`w-1 h-1 rounded-full ${isDragging ? "bg-white" : "bg-text-faint"}`} />
          <div className={`w-1 h-1 rounded-full ${isDragging ? "bg-white" : "bg-text-faint"}`} />
          <div className={`w-1 h-1 rounded-full ${isDragging ? "bg-white" : "bg-text-faint"}`} />
        </div>
      </div>

      {/* Right pane — preview */}
      <div className="overflow-y-auto min-w-0 bg-editor-bg">{right}</div>
    </div>
  );
}
