"use client";

import * as React from "react";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetFooter } from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type EditorRenderProps = {
  close: () => void;
};

type CommonProps = {
  label: string;
  value: React.ReactNode;
  /**
   * Children rendered inside the popover/sheet body. Receives `close` for Save/Cancel.
   * Returning a form with its own Save/Cancel is idiomatic.
   */
  editor: (props: EditorRenderProps) => React.ReactNode;
  /** Additional classes on the row container. */
  className?: string;
  /** When true, disables click-to-edit entirely (for read-only rows). */
  readOnly?: boolean;
  /** Optional explicit trigger — defaults to the whole row being clickable. */
  alignRight?: boolean;
};

type EditableRowProps =
  | (CommonProps & { mode: "popover"; sheetTitle?: never; sheetSide?: never })
  | (CommonProps & { mode: "sheet"; sheetTitle: string; sheetSide?: "left" | "right" | "top" | "bottom" });

/**
 * Dense, read-first row. Hover underlines the value; click opens a popover or sheet
 * for editing. Editor content is responsible for its own Save/Cancel via the `close` prop.
 */
export function EditableRow(props: EditableRowProps) {
  const { label, value, editor, className, readOnly, alignRight } = props;
  const [open, setOpen] = React.useState(false);
  const close = React.useCallback(() => setOpen(false), []);

  const row = (
    <div
      data-editable-row
      className={cn(
        "group flex items-baseline gap-3 py-2.5 border-b border-border last:border-0",
        !readOnly && "cursor-pointer hover:bg-muted/30 -mx-2 px-2 rounded-sm transition-colors",
        className,
      )}
    >
      <dt className="text-xs uppercase tracking-wide text-muted-foreground w-24 flex-shrink-0">
        {label}
      </dt>
      <dd
        data-editable-value
        className={cn(
          "text-sm text-foreground flex-1 min-w-0",
          alignRight && "text-right",
          !readOnly && "group-hover:underline decoration-muted-foreground/60 underline-offset-4",
        )}
      >
        {value}
      </dd>
    </div>
  );

  if (readOnly) return row;

  if (props.mode === "popover") {
    return (
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger render={<button type="button" className="w-full text-left" />}>
          {row}
        </PopoverTrigger>
        <PopoverContent className="w-80" align="end">
          <div className="mb-1 text-xs uppercase tracking-wide text-muted-foreground">{label}</div>
          {editor({ close })}
        </PopoverContent>
      </Popover>
    );
  }

  // sheet mode
  return (
    <>
      <button
        type="button"
        className="w-full text-left"
        onClick={() => setOpen(true)}
      >
        {row}
      </button>
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent side={props.sheetSide ?? "right"} className="flex flex-col gap-0">
          <SheetHeader>
            <SheetTitle>{props.sheetTitle}</SheetTitle>
          </SheetHeader>
          <div className="flex-1 overflow-y-auto px-4 pb-4">
            {editor({ close })}
          </div>
        </SheetContent>
      </Sheet>
    </>
  );
}

/**
 * Inline Save/Cancel footer matching popover + sheet styling.
 */
export function EditorFooter({
  onSave,
  onCancel,
  saving,
  disabled,
  saveLabel = "Save",
}: {
  onSave: () => void;
  onCancel: () => void;
  saving?: boolean;
  disabled?: boolean;
  saveLabel?: string;
}) {
  return (
    <div className="flex items-center justify-end gap-2 pt-3 border-t border-border mt-3">
      <Button variant="ghost" size="sm" onClick={onCancel} disabled={saving}>
        Cancel
      </Button>
      <Button size="sm" onClick={onSave} disabled={saving || disabled}>
        {saving ? "Saving…" : saveLabel}
      </Button>
    </div>
  );
}

/**
 * Placeholder for empty values. Renders italic muted text.
 */
export function Empty({ children = "Not set" }: { children?: React.ReactNode }) {
  return <span className="italic text-muted-foreground/70">{children}</span>;
}
