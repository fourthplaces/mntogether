"use client";

const PROMPTS = [
  {
    label: "Food Help",
    icon: "\u{1F35E}",
    query: "Where can I find food assistance or food shelves?",
    color: "bg-orange-50 border-orange-200 hover:bg-orange-100",
  },
  {
    label: "Housing",
    icon: "\u{1F3E0}",
    query: "I need help with housing or rent assistance",
    color: "bg-blue-50 border-blue-200 hover:bg-blue-100",
  },
  {
    label: "Legal Aid",
    icon: "\u{2696}\u{FE0F}",
    query: "Where can I get free legal help?",
    color: "bg-purple-50 border-purple-200 hover:bg-purple-100",
  },
  {
    label: "Volunteer",
    icon: "\u{1F91D}",
    query: "How can I volunteer in my community?",
    color: "bg-emerald-50 border-emerald-200 hover:bg-emerald-100",
  },
  {
    label: "Healthcare",
    icon: "\u{1FA7A}",
    query: "I need affordable healthcare or mental health services",
    color: "bg-red-50 border-red-200 hover:bg-red-100",
  },
  {
    label: "Small Businesses",
    icon: "\u{1F3EA}",
    query: "Show me local businesses giving back to the community",
    color: "bg-amber-50 border-amber-200 hover:bg-amber-100",
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
