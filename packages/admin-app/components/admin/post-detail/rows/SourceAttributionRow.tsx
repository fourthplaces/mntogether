"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { EditableRow, EditorFooter, Empty } from "../EditableRow";

type SourceAttr = { sourceName?: string | null; attribution?: string | null } | null;

export function SourceAttributionRow({
  sourceAttribution,
  onSave,
}: {
  sourceAttribution: SourceAttr;
  onSave: (input: { sourceName: string | null; attribution: string | null }) => Promise<unknown>;
}) {
  const hasContent = !!(sourceAttribution?.sourceName || sourceAttribution?.attribution);
  const display = hasContent ? (
    <span>
      {sourceAttribution?.sourceName && <strong className="font-medium">{sourceAttribution.sourceName}</strong>}
      {sourceAttribution?.sourceName && sourceAttribution?.attribution && " — "}
      {sourceAttribution?.attribution && (
        <span className="text-muted-foreground">{sourceAttribution.attribution}</span>
      )}
    </span>
  ) : <Empty>No source</Empty>;

  return (
    <EditableRow
      label="Source"
      value={display}
      mode="popover"
      editor={({ close }) => (
        <Editor
          sourceAttribution={sourceAttribution}
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
  sourceAttribution,
  onSave,
  onCancel,
}: {
  sourceAttribution: SourceAttr;
  onSave: (input: { sourceName: string | null; attribution: string | null }) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [sourceName, setSourceName] = React.useState(sourceAttribution?.sourceName || "");
  const [attribution, setAttribution] = React.useState(sourceAttribution?.attribution || "");
  const [saving, setSaving] = React.useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave({
        sourceName: sourceName.trim() || null,
        attribution: attribution.trim() || null,
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-2">
      <label className="block text-xs text-muted-foreground">Source name</label>
      <Input
        value={sourceName}
        onChange={(e) => setSourceName(e.target.value)}
        placeholder="e.g. City of Minneapolis"
        className="text-sm"
      />
      <label className="block text-xs text-muted-foreground">Attribution</label>
      <Input
        value={attribution}
        onChange={(e) => setAttribution(e.target.value)}
        placeholder="e.g. Press release dated Apr 1"
        className="text-sm"
      />
      <EditorFooter onSave={handleSave} onCancel={onCancel} saving={saving} />
    </div>
  );
}
