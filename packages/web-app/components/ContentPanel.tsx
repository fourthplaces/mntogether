"use client";

import { SuggestedPrompts } from "@/components/SuggestedPrompts";

interface ContentPanelProps {
  onSuggestedPrompt: (query: string) => void;
}

export function ContentPanel({ onSuggestedPrompt }: ContentPanelProps) {
  return (
    <div className="content-panel">
      <div className="content-panel-inner">
        <h1 className="hero-title">
          MN Together
        </h1>
        <p className="hero-subtitle">
          Find services, volunteer opportunities, and community resources
          across Minnesota.
        </p>
        <SuggestedPrompts onSelect={onSuggestedPrompt} />
      </div>
    </div>
  );
}
