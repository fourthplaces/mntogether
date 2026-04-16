"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { EditableRow, EditorFooter, Empty } from "../EditableRow";

type Link = { label?: string | null; url?: string | null; deadline?: string | null } | null;

export function LinkRow({
  link,
  onSave,
}: {
  link: Link;
  onSave: (input: { label: string | null; url: string | null; deadline: string | null }) => Promise<unknown>;
}) {
  const display = link?.url ? (
    <span>
      <a
        href={link.url.startsWith("http") ? link.url : `https://${link.url}`}
        target="_blank"
        rel="noopener noreferrer"
        className="text-link hover:text-link-hover"
        onClick={(e) => e.stopPropagation()}
      >
        {link.label || link.url.replace(/^https?:\/\//, "")}
      </a>
      {link.deadline && (
        <span className="text-muted-foreground ml-2 text-xs">
          Deadline {new Date(link.deadline).toLocaleDateString()}
        </span>
      )}
    </span>
  ) : <Empty>No link</Empty>;

  return (
    <EditableRow
      label="Link"
      value={display}
      mode="popover"
      editor={({ close }) => (
        <Editor
          link={link}
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
  link,
  onSave,
  onCancel,
}: {
  link: Link;
  onSave: (input: { label: string | null; url: string | null; deadline: string | null }) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [label, setLabel] = React.useState(link?.label || "");
  const [url, setUrl] = React.useState(link?.url || "");
  const [deadline, setDeadline] = React.useState(link?.deadline || "");
  const [saving, setSaving] = React.useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave({
        label: label.trim() || null,
        url: url.trim() || null,
        deadline: deadline.trim() || null,
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-2">
      <label className="block text-xs text-muted-foreground">Label</label>
      <Input
        value={label}
        onChange={(e) => setLabel(e.target.value)}
        placeholder="e.g. Apply Now"
        className="text-sm"
      />
      <label className="block text-xs text-muted-foreground">URL</label>
      <Input
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="https://…"
        className="text-sm"
      />
      <label className="block text-xs text-muted-foreground">Deadline (optional)</label>
      <Input
        type="date"
        value={deadline}
        onChange={(e) => setDeadline(e.target.value)}
        className="text-sm"
      />
      <EditorFooter onSave={handleSave} onCancel={onCancel} saving={saving} />
    </div>
  );
}
