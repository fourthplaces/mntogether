"use client";

import { SuggestedPrompts } from "@/components/public/SuggestedPrompts";

interface ContentPanelProps {
  onSuggestedPrompt: (query: string) => void;
}

export function ContentPanel({ onSuggestedPrompt }: ContentPanelProps) {
  return (
    <div className="flex flex-col items-center justify-center h-full p-6">
      <div className="max-w-2xl w-full text-center mb-8">
        <h1 className="text-3xl sm:text-4xl font-bold text-gray-900 mb-3">
          MN Together
        </h1>
        <p className="text-gray-600 text-lg mb-8">
          Find services, volunteer opportunities, and community resources
          across Minnesota.
        </p>
        <SuggestedPrompts onSelect={onSuggestedPrompt} />
      </div>
    </div>
  );
}
