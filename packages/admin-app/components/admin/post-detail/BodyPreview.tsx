"use client";

export function BodyPreview({ label, text }: { label: string; text?: string | null }) {
  return (
    <div className="border-t border-border pt-4">
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
        {label}
      </h3>
      {text ? (
        <p className="text-sm text-text-body leading-relaxed whitespace-pre-wrap">{text}</p>
      ) : (
        <p className="text-sm text-muted-foreground italic">Not yet generated</p>
      )}
    </div>
  );
}
