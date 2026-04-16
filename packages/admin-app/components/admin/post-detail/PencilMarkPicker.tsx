"use client";

import * as React from "react";
import Image from "next/image";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { cn } from "@/lib/utils";

type PencilMark = "star" | "heart" | "smile" | "circle" | null;

const MARKS: Array<{ value: Exclude<PencilMark, null>; label: string; src: string }> = [
  { value: "star", label: "Star", src: "/pencils/pencil-star.svg" },
  { value: "heart", label: "Heart", src: "/pencils/pencil-heart.svg" },
  { value: "smile", label: "Smile", src: "/pencils/pencil-smile.svg" },
  { value: "circle", label: "Circle", src: "/pencils/pencil-circle.svg" },
];

export function PencilMarkPicker({
  value,
  onChange,
}: {
  value: PencilMark;
  onChange: (next: PencilMark) => void;
}) {
  const [open, setOpen] = React.useState(false);
  const current = MARKS.find((m) => m.value === value);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={
          <button
            type="button"
            title={current ? `Pencil mark: ${current.label}` : "No pencil mark"}
            className="inline-flex items-center justify-center h-9 w-9 rounded-md border border-border bg-card hover:bg-accent transition-colors"
          />
        }
      >
        {current ? (
          <Image src={current.src} alt={current.label} width={22} height={22} />
        ) : (
          <span className="block h-5 w-5 rounded-full border border-dashed border-muted-foreground/50" />
        )}
      </PopoverTrigger>
      <PopoverContent className="w-auto p-2" align="end">
        <div className="mb-1 px-1 text-xs uppercase tracking-wide text-muted-foreground">
          Pencil mark
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => {
              onChange(null);
              setOpen(false);
            }}
            className={cn(
              "inline-flex h-10 w-10 items-center justify-center rounded-md border border-border text-muted-foreground hover:bg-accent",
              value === null && "ring-2 ring-primary/50",
            )}
            title="None"
          >
            <span className="block h-5 w-5 rounded-full border border-dashed border-muted-foreground/50" />
          </button>
          {MARKS.map((m) => (
            <button
              key={m.value}
              type="button"
              onClick={() => {
                onChange(m.value);
                setOpen(false);
              }}
              className={cn(
                "inline-flex h-10 w-10 items-center justify-center rounded-md border border-border hover:bg-accent",
                value === m.value && "ring-2 ring-primary/50",
              )}
              title={m.label}
            >
              <Image src={m.src} alt={m.label} width={26} height={26} />
            </button>
          ))}
        </div>
      </PopoverContent>
    </Popover>
  );
}
