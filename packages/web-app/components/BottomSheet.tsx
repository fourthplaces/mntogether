"use client";

import { useState, useEffect, useCallback } from "react";

interface BottomSheetProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
}

export function BottomSheet({ isOpen, onClose, title, children }: BottomSheetProps) {
  // Two-phase state: `render` controls mount, `visible` controls animation
  const [render, setRender] = useState(false);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setRender(true);
      // Next frame: trigger enter animation
      requestAnimationFrame(() => {
        requestAnimationFrame(() => setVisible(true));
      });
      document.body.style.overflow = "hidden";
    } else {
      setVisible(false);
      document.body.style.overflow = "";
    }

    return () => {
      document.body.style.overflow = "";
    };
  }, [isOpen]);

  const handleTransitionEnd = useCallback(() => {
    if (!visible) setRender(false);
  }, [visible]);

  if (!render) return null;

  return (
    <div className="overlay">
      {/* Backdrop */}
      <div
        className={`backdrop ${visible ? "backdrop--visible" : "backdrop--hidden"}`}
        onClick={onClose}
      />

      {/* Panel */}
      <div
        className={`bottom-sheet-panel ${visible ? "bottom-sheet-panel--visible" : "bottom-sheet-panel--hidden"}`}
        onTransitionEnd={handleTransitionEnd}
      >
        {/* Drag handle + close */}
        <div className="bottom-sheet-handle-bar">
          <div className="bottom-sheet-spacer" />
          <div className="bottom-sheet-handle" />
          <div className="bottom-sheet-close-wrapper">
            <button
              onClick={onClose}
              className="btn-close"
              aria-label="Close"
            >
              &times;
            </button>
          </div>
        </div>

        {/* Optional title */}
        {title && (
          <div className="bottom-sheet-title">
            <h2>{title}</h2>
          </div>
        )}

        {/* Content */}
        <div className="bottom-sheet-content">{children}</div>
      </div>
    </div>
  );
}
