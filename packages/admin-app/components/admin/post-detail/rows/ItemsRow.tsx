"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Plus, X, GripVertical } from "lucide-react";
import { EditableRow, EditorFooter, Empty } from "../EditableRow";

type Item = { name: string; detail?: string | null };

export function ItemsRow({
  items,
  onSave,
}: {
  items: Item[];
  onSave: (items: Item[]) => Promise<unknown>;
}) {
  const display = items.length > 0 ? (
    <div className="flex flex-col min-w-0">
      <span className="text-sm">
        {items.slice(0, 3).map((i) => i.name).join(", ")}
        {items.length > 3 && <span className="text-muted-foreground"> + {items.length - 3} more</span>}
      </span>
      <span className="text-xs text-muted-foreground">{items.length} total</span>
    </div>
  ) : <Empty>No items</Empty>;

  return (
    <EditableRow
      label="Items"
      value={display}
      mode="sheet"
      sheetTitle="Items list"
      editor={({ close }) => (
        <Editor
          initialItems={items}
          onSave={async (next) => {
            await onSave(next);
            close();
          }}
          onCancel={close}
        />
      )}
    />
  );
}

function Editor({
  initialItems,
  onSave,
  onCancel,
}: {
  initialItems: Item[];
  onSave: (items: Item[]) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [items, setItems] = React.useState<Item[]>(initialItems);
  const [saving, setSaving] = React.useState(false);
  const [newName, setNewName] = React.useState("");
  const [newDetail, setNewDetail] = React.useState("");

  const addItem = () => {
    if (!newName.trim()) return;
    setItems([...items, { name: newName.trim(), detail: newDetail.trim() || null }]);
    setNewName("");
    setNewDetail("");
  };

  const removeItem = (idx: number) => {
    setItems(items.filter((_, i) => i !== idx));
  };

  const updateItem = (idx: number, patch: Partial<Item>) => {
    setItems(items.map((it, i) => (i === idx ? { ...it, ...patch } : it)));
  };

  const move = (idx: number, dir: -1 | 1) => {
    const newIdx = idx + dir;
    if (newIdx < 0 || newIdx >= items.length) return;
    const next = [...items];
    [next[idx], next[newIdx]] = [next[newIdx], next[idx]];
    setItems(next);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave(items.map((i) => ({ name: i.name, detail: i.detail || null })));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-4 py-2">
      <div className="space-y-1.5">
        {items.length === 0 ? (
          <p className="text-sm text-muted-foreground italic">No items yet.</p>
        ) : items.map((item, idx) => (
          <div key={idx} className="flex items-start gap-1.5 group">
            <div className="flex flex-col mt-1">
              <button
                type="button"
                onClick={() => move(idx, -1)}
                disabled={idx === 0}
                className="text-muted-foreground hover:text-foreground disabled:opacity-30"
                title="Move up"
              >
                <GripVertical className="h-3 w-3 -mb-1" />
              </button>
            </div>
            <div className="flex-1 space-y-1 min-w-0">
              <Input
                value={item.name}
                onChange={(e) => updateItem(idx, { name: e.target.value })}
                placeholder="Name"
                className="text-sm"
              />
              <Input
                value={item.detail || ""}
                onChange={(e) => updateItem(idx, { detail: e.target.value })}
                placeholder="Detail (optional)"
                className="text-sm"
              />
            </div>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => removeItem(idx)}
              className="h-6 w-6 mt-1 text-muted-foreground hover:text-danger-text"
              title="Remove"
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </div>
        ))}
      </div>

      <div className="border-t border-border pt-3">
        <div className="text-xs uppercase tracking-wide text-muted-foreground mb-2">Add item</div>
        <div className="space-y-2">
          <Input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) addItem(); }}
            placeholder="Name…"
            className="text-sm"
          />
          <Input
            value={newDetail}
            onChange={(e) => setNewDetail(e.target.value)}
            placeholder="Detail (optional)"
            className="text-sm"
          />
          <Button onClick={addItem} disabled={!newName.trim()} size="sm" className="w-full">
            <Plus className="h-4 w-4 mr-1" />
            Add item
          </Button>
        </div>
      </div>

      <EditorFooter onSave={handleSave} onCancel={onCancel} saving={saving} />
    </div>
  );
}
