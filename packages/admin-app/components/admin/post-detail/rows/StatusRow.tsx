"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { EditableRow, EditorFooter, Empty } from "../EditableRow";

type PostStatus = { state?: string | null; verified?: string | null } | null;

// Post-op states, simplified per migration 227 plan: open|closed.
// Accept legacy values (available/needed) for display only.
const STATE_OPTIONS = [
  { value: "open", label: "Open" },
  { value: "closed", label: "Closed" },
];

function normalizeState(raw?: string | null): string {
  if (!raw) return "";
  if (raw === "available" || raw === "needed") return "open";
  return raw;
}

function stateDot(state?: string | null) {
  const s = normalizeState(state);
  if (s === "open") return <span className="inline-block w-2 h-2 rounded-full bg-green-500 mr-1.5" />;
  if (s === "closed") return <span className="inline-block w-2 h-2 rounded-full bg-muted-foreground/60 mr-1.5" />;
  return null;
}

export function StatusRow({
  postStatus,
  onSave,
}: {
  postStatus: PostStatus;
  onSave: (input: { state: string | null; verified: string | null }) => Promise<unknown>;
}) {
  const state = normalizeState(postStatus?.state);
  const display = state ? (
    <span className="flex items-center">
      {stateDot(state)}
      <span className="capitalize">{state}</span>
      {postStatus?.verified && (
        <span className="text-muted-foreground ml-2 text-xs">
          verified {new Date(postStatus.verified).toLocaleDateString()}
        </span>
      )}
    </span>
  ) : <Empty>Not set</Empty>;

  return (
    <EditableRow
      label="Status"
      value={display}
      mode="popover"
      editor={({ close }) => (
        <Editor
          postStatus={postStatus}
          onSave={async (val) => {
            await onSave(val);
            close();
          }}
          onCancel={close}
        />
      )}
    />
  );
}

function Editor({
  postStatus,
  onSave,
  onCancel,
}: {
  postStatus: PostStatus;
  onSave: (input: { state: string | null; verified: string | null }) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [state, setState] = React.useState(normalizeState(postStatus?.state));
  const [verified, setVerified] = React.useState(postStatus?.verified || "");
  const [saving, setSaving] = React.useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave({ state: state || null, verified: verified || null });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-2">
      <label className="block text-xs text-muted-foreground">State</label>
      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => setState("")}
          className={`flex-1 px-3 py-1.5 text-sm rounded-md border ${state === "" ? "border-primary bg-primary/5 text-foreground" : "border-border text-muted-foreground hover:bg-muted/40"}`}
        >
          None
        </button>
        {STATE_OPTIONS.map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => setState(opt.value)}
            className={`flex-1 px-3 py-1.5 text-sm rounded-md border ${state === opt.value ? "border-primary bg-primary/5 text-foreground" : "border-border text-muted-foreground hover:bg-muted/40"}`}
          >
            {opt.label}
          </button>
        ))}
      </div>
      <label className="block text-xs text-muted-foreground mt-2">Last verified</label>
      <Input
        type="date"
        value={verified}
        onChange={(e) => setVerified(e.target.value)}
        className="text-sm"
      />
      <EditorFooter onSave={handleSave} onCancel={onCancel} saving={saving} />
    </div>
  );
}
