"use client";

import { Textarea } from "@/components/ui/textarea";
import { Separator } from "@/components/ui/separator";
import { FieldWrapper } from "@/components/admin/FieldWrapper";
import type { PostFormValues } from "@/lib/post-form-constants";

interface PostEditorFormProps {
  values: PostFormValues;
  onChange: (values: PostFormValues) => void;
  errors: Record<string, string>;
  disabled?: boolean;
}

export function PostEditorForm({
  values,
  onChange,
  errors,
  disabled = false,
}: PostEditorFormProps) {
  // Helper to update a single field
  function update<K extends keyof PostFormValues>(field: K, value: PostFormValues[K]) {
    onChange({ ...values, [field]: value });
  }

  return (
    <div className="p-6 space-y-0">
      {/* Title — prominent, no label wrapper */}
      <div className="mb-2">
        <input
          type="text"
          value={values.title}
          onChange={(e) => update("title", e.target.value)}
          placeholder="Post title..."
          disabled={disabled}
          className={`w-full text-2xl font-semibold text-foreground bg-transparent border-0 border-b-2 px-0 py-2 focus:outline-none transition-colors placeholder:text-muted-foreground ${
            errors.title
              ? "border-destructive"
              : "border-transparent focus:border-admin-accent"
          }`}
        />
        {errors.title && (
          <p className="text-xs text-destructive mt-1">{errors.title}</p>
        )}
      </div>

      {/* Content — markdown textarea (future Plate.js swap point) */}
      <Separator className="my-5" />

      <FieldWrapper
        label="Content"
        required
        error={errors.bodyRaw}
        hint="Markdown supported"
        className="mb-0"
      >
        <Textarea
          value={values.bodyRaw}
          onChange={(e) => update("bodyRaw", e.target.value)}
          placeholder="Write your content in Markdown..."
          rows={20}
          disabled={disabled}
          className="font-mono text-sm leading-relaxed"
        />
      </FieldWrapper>
    </div>
  );
}
