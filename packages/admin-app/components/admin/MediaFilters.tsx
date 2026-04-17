"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Search, EyeOff } from "lucide-react";
import { cn } from "@/lib/utils";

export type MediaSort = "newest" | "oldest" | "most_used" | "largest";

export type MediaFilterState = {
  search: string;
  unusedOnly: boolean;
  sort: MediaSort;
};

export function MediaFilters({
  value,
  onChange,
}: {
  value: MediaFilterState;
  onChange: (next: MediaFilterState) => void;
}) {
  return (
    <div className="flex items-center gap-2 flex-wrap">
      <div className="relative flex-1 min-w-[240px]">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
        <Input
          value={value.search}
          onChange={(e) => onChange({ ...value, search: e.target.value })}
          placeholder="Search filename or alt text…"
          className="pl-8 text-sm"
        />
      </div>

      <Button
        variant={value.unusedOnly ? "default" : "outline"}
        size="sm"
        onClick={() => onChange({ ...value, unusedOnly: !value.unusedOnly })}
        title="Show only media with zero references — great for cleanup."
      >
        <EyeOff className="size-3.5 mr-1.5" />
        Unused only
      </Button>

      <select
        value={value.sort}
        onChange={(e) => onChange({ ...value, sort: e.target.value as MediaSort })}
        className={cn(
          "rounded-md border border-border bg-background px-2.5 py-1.5 text-sm",
          "focus:outline-none focus:ring-2 focus:ring-primary/40",
        )}
      >
        <option value="newest">Newest</option>
        <option value="oldest">Oldest</option>
        <option value="most_used">Most used</option>
        <option value="largest">Largest</option>
      </select>
    </div>
  );
}
