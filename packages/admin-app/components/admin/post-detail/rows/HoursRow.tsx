"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { Plus, X } from "lucide-react";
import { EditableRow, Empty } from "../EditableRow";

type ScheduleEntry = {
  id?: string | null;
  day: string;
  opens: string;
  closes: string;
};

const DAYS = [
  { value: "sunday", label: "Sun", dayOfWeek: 0 },
  { value: "monday", label: "Mon", dayOfWeek: 1 },
  { value: "tuesday", label: "Tue", dayOfWeek: 2 },
  { value: "wednesday", label: "Wed", dayOfWeek: 3 },
  { value: "thursday", label: "Thu", dayOfWeek: 4 },
  { value: "friday", label: "Fri", dayOfWeek: 5 },
  { value: "saturday", label: "Sat", dayOfWeek: 6 },
];

function dayToInt(day: string): number {
  return DAYS.find((d) => d.value === day.toLowerCase())?.dayOfWeek ?? 1;
}

function dayLabel(day: string): string {
  return DAYS.find((d) => d.value === day.toLowerCase())?.label ?? day;
}

function normalizeTime(s: string): string {
  // Accept "HH:MM", "HH:MM:SS", etc., always return "HH:MM"
  if (!s) return "";
  return s.slice(0, 5);
}

function formatHours(time: string): string {
  const [h, m] = time.split(":").map(Number);
  if (isNaN(h)) return time;
  const ampm = h >= 12 ? "pm" : "am";
  const hour12 = h % 12 || 12;
  return m ? `${hour12}:${String(m).padStart(2, "0")}${ampm}` : `${hour12}${ampm}`;
}

function groupedDisplay(entries: ScheduleEntry[]): React.ReactNode {
  if (entries.length === 0) return null;
  // Sort by day-of-week
  const sorted = [...entries].sort((a, b) => dayToInt(a.day) - dayToInt(b.day));
  return (
    <div className="flex flex-col text-sm">
      {sorted.slice(0, 3).map((e, i) => (
        <span key={e.id ?? i}>
          <span className="text-muted-foreground">{dayLabel(e.day)}</span>{" "}
          {formatHours(normalizeTime(e.opens))}–{formatHours(normalizeTime(e.closes))}
        </span>
      ))}
      {sorted.length > 3 && (
        <span className="text-xs text-muted-foreground">+ {sorted.length - 3} more</span>
      )}
    </div>
  );
}

export function HoursRow({
  schedule,
  onAdd,
  onDelete,
}: {
  schedule: ScheduleEntry[];
  onAdd: (input: { dayOfWeek: number; opensAt: string; closesAt: string }) => Promise<unknown>;
  onDelete: (scheduleId: string) => Promise<unknown>;
}) {
  const display = schedule.length > 0 ? groupedDisplay(schedule) : <Empty>No hours</Empty>;

  return (
    <EditableRow
      label="Hours"
      value={display}
      mode="sheet"
      sheetTitle="Hours of operation"
      editor={({ close }) => (
        <Editor
          schedule={schedule}
          onAdd={onAdd}
          onDelete={onDelete}
          onDone={close}
        />
      )}
    />
  );
}

function Editor({
  schedule,
  onAdd,
  onDelete,
  onDone,
}: {
  schedule: ScheduleEntry[];
  onAdd: (input: { dayOfWeek: number; opensAt: string; closesAt: string }) => Promise<unknown>;
  onDelete: (scheduleId: string) => Promise<unknown>;
  onDone: () => void;
}) {
  const [newDay, setNewDay] = React.useState("monday");
  const [newOpens, setNewOpens] = React.useState("09:00");
  const [newCloses, setNewCloses] = React.useState("17:00");
  const [busy, setBusy] = React.useState(false);

  const handleAdd = async () => {
    setBusy(true);
    try {
      await onAdd({
        dayOfWeek: dayToInt(newDay),
        opensAt: newOpens,
        closesAt: newCloses,
      });
    } finally {
      setBusy(false);
    }
  };

  const sorted = [...schedule].sort((a, b) => dayToInt(a.day) - dayToInt(b.day));

  return (
    <div className="space-y-4 py-2">
      <div className="space-y-1.5">
        {sorted.length === 0 ? (
          <p className="text-sm text-muted-foreground italic">No hours set yet.</p>
        ) : sorted.map((entry) => (
          <div key={entry.id ?? `${entry.day}-${entry.opens}`} className="flex items-center gap-2 py-1.5">
            <span className="text-xs uppercase tracking-wider text-muted-foreground w-10">
              {dayLabel(entry.day)}
            </span>
            <span className="text-sm flex-1">
              {formatHours(normalizeTime(entry.opens))} – {formatHours(normalizeTime(entry.closes))}
            </span>
            {entry.id && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => onDelete(entry.id!)}
                className="h-6 w-6 text-muted-foreground hover:text-danger-text"
                title="Remove"
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        ))}
      </div>

      <div className="border-t border-border pt-3">
        <div className="text-xs uppercase tracking-wide text-muted-foreground mb-2">Add hours</div>
        <div className="space-y-2">
          <Select value={newDay} onValueChange={(v) => v && setNewDay(v)}>
            <SelectTrigger className="text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {DAYS.map((d) => (
                <SelectItem key={d.value} value={d.value}>{d.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="flex items-center gap-2">
            <Input type="time" value={newOpens} onChange={(e) => setNewOpens(e.target.value)} className="text-sm" />
            <span className="text-muted-foreground">–</span>
            <Input type="time" value={newCloses} onChange={(e) => setNewCloses(e.target.value)} className="text-sm" />
          </div>
          <Button onClick={handleAdd} disabled={busy} size="sm" className="w-full">
            <Plus className="h-4 w-4 mr-1" />
            Add hours
          </Button>
        </div>
      </div>

      <div className="flex justify-end pt-2">
        <Button variant="outline" size="sm" onClick={onDone}>Done</Button>
      </div>
    </div>
  );
}
