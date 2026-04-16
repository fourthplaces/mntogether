"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { EditableRow, EditorFooter, Empty } from "../EditableRow";

type Datetime = {
  start?: string | null;
  end?: string | null;
  cost?: string | null;
  recurring?: boolean | null;
} | null;

function formatDt(iso?: string | null) {
  if (!iso) return null;
  const d = new Date(iso);
  return d.toLocaleString("en-US", {
    weekday: "short",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

export function EventDateRow({
  datetime,
  onSave,
}: {
  datetime: Datetime;
  onSave: (input: { start: string | null; end: string | null; cost: string | null; recurring: boolean }) => Promise<unknown>;
}) {
  const startStr = formatDt(datetime?.start);
  const endStr = formatDt(datetime?.end);

  const display = startStr ? (
    <span>
      {startStr}
      {endStr && <> – {endStr}</>}
      {datetime?.cost && <span className="text-muted-foreground ml-2">· {datetime.cost}</span>}
      {datetime?.recurring && <span className="text-muted-foreground ml-2">· Recurring</span>}
    </span>
  ) : <Empty>No date set</Empty>;

  return (
    <EditableRow
      label="Event"
      value={display}
      mode="popover"
      editor={({ close }) => (
        <Editor
          datetime={datetime}
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

function toLocalInput(iso?: string | null): string {
  if (!iso) return "";
  // Format as 'YYYY-MM-DDTHH:MM' for datetime-local input
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function Editor({
  datetime,
  onSave,
  onCancel,
}: {
  datetime: Datetime;
  onSave: (input: { start: string | null; end: string | null; cost: string | null; recurring: boolean }) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [start, setStart] = React.useState(toLocalInput(datetime?.start));
  const [end, setEnd] = React.useState(toLocalInput(datetime?.end));
  const [cost, setCost] = React.useState(datetime?.cost || "");
  const [recurring, setRecurring] = React.useState(!!datetime?.recurring);
  const [saving, setSaving] = React.useState(false);

  const toRfc3339 = (local: string): string | null => {
    if (!local) return null;
    return new Date(local).toISOString();
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave({
        start: toRfc3339(start),
        end: toRfc3339(end),
        cost: cost.trim() || null,
        recurring,
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-2">
      <label className="block text-xs text-muted-foreground">Starts</label>
      <Input type="datetime-local" value={start} onChange={(e) => setStart(e.target.value)} className="text-sm" />
      <label className="block text-xs text-muted-foreground">Ends</label>
      <Input type="datetime-local" value={end} onChange={(e) => setEnd(e.target.value)} className="text-sm" />
      <label className="block text-xs text-muted-foreground">Cost (optional)</label>
      <Input value={cost} onChange={(e) => setCost(e.target.value)} placeholder="e.g. Free, $5" className="text-sm" />
      <label className="flex items-center gap-2 text-sm text-foreground">
        <input
          type="checkbox"
          checked={recurring}
          onChange={(e) => setRecurring(e.target.checked)}
          className="rounded"
        />
        Recurring event
      </label>
      <EditorFooter onSave={handleSave} onCancel={onCancel} saving={saving} />
    </div>
  );
}
