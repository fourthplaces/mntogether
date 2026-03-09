"use client";

const PROMPTS = [
  {
    label: "Food Help",
    icon: "\u{1F35E}",
    query: "Where can I find food assistance or food shelves?",
  },
  {
    label: "Housing",
    icon: "\u{1F3E0}",
    query: "I need help with housing or rent assistance",
  },
  {
    label: "Legal Aid",
    icon: "\u{2696}\u{FE0F}",
    query: "Where can I get free legal help?",
  },
  {
    label: "Volunteer",
    icon: "\u{1F91D}",
    query: "How can I volunteer in my community?",
  },
  {
    label: "Healthcare",
    icon: "\u{1FA7A}",
    query: "I need affordable healthcare or mental health services",
  },
  {
    label: "Small Businesses",
    icon: "\u{1F3EA}",
    query: "Show me local businesses giving back to the community",
  },
] as const;

interface SuggestedPromptsProps {
  onSelect: (query: string) => void;
}

export function SuggestedPrompts({ onSelect }: SuggestedPromptsProps) {
  return (
    <div className="prompt-grid">
      {PROMPTS.map((prompt) => (
        <button
          key={prompt.label}
          onClick={() => onSelect(prompt.query)}
          className="prompt-button"
        >
          <span className="prompt-icon">{prompt.icon}</span>
          <span className="prompt-label">
            {prompt.label}
          </span>
        </button>
      ))}
    </div>
  );
}
