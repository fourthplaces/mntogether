"use client";

const PROMPTS = [
  {
    label: "Food Help",
    icon: "\u{1F35E}",
    query: "Where can I find food assistance or food shelves?",
    color: "bg-gray-50 border-gray-200 hover:bg-gray-100",
  },
  {
    label: "Housing",
    icon: "\u{1F3E0}",
    query: "I need help with housing or rent assistance",
    color: "bg-gray-50 border-gray-200 hover:bg-gray-100",
  },
  {
    label: "Legal Aid",
    icon: "\u{2696}\u{FE0F}",
    query: "Where can I get free legal help?",
    color: "bg-gray-50 border-gray-200 hover:bg-gray-100",
  },
  {
    label: "Volunteer",
    icon: "\u{1F91D}",
    query: "How can I volunteer in my community?",
    color: "bg-gray-50 border-gray-200 hover:bg-gray-100",
  },
  {
    label: "Healthcare",
    icon: "\u{1FA7A}",
    query: "I need affordable healthcare or mental health services",
    color: "bg-gray-50 border-gray-200 hover:bg-gray-100",
  },
  {
    label: "Small Businesses",
    icon: "\u{1F3EA}",
    query: "Show me local businesses giving back to the community",
    color: "bg-gray-50 border-gray-200 hover:bg-gray-100",
  },
] as const;

interface SuggestedPromptsProps {
  onSelect: (query: string) => void;
}

export function SuggestedPrompts({ onSelect }: SuggestedPromptsProps) {
  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
      {PROMPTS.map((prompt) => (
        <button
          key={prompt.label}
          onClick={() => onSelect(prompt.query)}
          className={`flex flex-col items-center gap-2 p-4 rounded-xl border ${prompt.color} transition-all duration-200 text-center`}
        >
          <span className="text-2xl">{prompt.icon}</span>
          <span className="text-sm font-medium text-gray-700">
            {prompt.label}
          </span>
        </button>
      ))}
    </div>
  );
}
