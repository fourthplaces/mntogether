"use client";

import { useQuery } from "urql";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { FieldWrapper } from "@/components/admin/FieldWrapper";
import { OrganizationsListQuery } from "@/lib/graphql/organizations";
import {
  POST_TYPES,
  WEIGHTS,
  URGENCIES,
  type PostFormValues,
} from "@/lib/post-form-constants";

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
  const [{ data: orgsData }] = useQuery({ query: OrganizationsListQuery });
  const organizations = orgsData?.organizations ?? [];

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

      {/* Metadata section */}
      <Separator className="my-5" />

      <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-1">
        <FieldWrapper label="Post Type" className="mb-0">
          <Select
            value={values.postType}
            onValueChange={(v) => update("postType", v)}
            disabled={disabled}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {POST_TYPES.map((t) => (
                <SelectItem key={t.value} value={t.value}>
                  {t.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FieldWrapper>

        <FieldWrapper label="Weight" className="mb-0">
          <Select
            value={values.weight}
            onValueChange={(v) => update("weight", v)}
            disabled={disabled}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {WEIGHTS.map((w) => (
                <SelectItem key={w.value} value={w.value}>
                  {w.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FieldWrapper>

        <FieldWrapper label="Priority" className="mb-0">
          <Input
            type="number"
            value={values.priority}
            onChange={(e) => update("priority", Number(e.target.value))}
            disabled={disabled}
          />
        </FieldWrapper>
      </div>

      <div className="grid grid-cols-2 gap-4 mt-4 mb-1">
        <FieldWrapper label="Urgency" className="mb-0">
          <Select
            value={values.urgency || "__none__"}
            onValueChange={(v) => update("urgency", v === "__none__" ? "" : v)}
            disabled={disabled}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {URGENCIES.map((u) => (
                <SelectItem key={u.value || "__none__"} value={u.value || "__none__"}>
                  {u.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FieldWrapper>

        <FieldWrapper label="Location" className="mb-0">
          <Input
            value={values.location}
            onChange={(e) => update("location", e.target.value)}
            placeholder="e.g. Minneapolis, MN"
            disabled={disabled}
          />
        </FieldWrapper>
      </div>

      <div className="mt-4">
        <FieldWrapper label="Organization" className="mb-0">
          <Select
            value={values.organizationId || "__none__"}
            onValueChange={(v) => update("organizationId", v === "__none__" ? "" : v)}
            disabled={disabled}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__none__">None</SelectItem>
              {organizations.map((org) => (
                <SelectItem key={org.id} value={org.id}>
                  {org.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FieldWrapper>
      </div>

      {/* Summary */}
      <Separator className="my-5" />

      <FieldWrapper
        label="Summary"
        hint="Brief plain-text summary for cards and previews"
        className="mb-0"
      >
        <Textarea
          value={values.summary}
          onChange={(e) => update("summary", e.target.value)}
          placeholder="Optional summary..."
          rows={2}
          disabled={disabled}
        />
      </FieldWrapper>

      {/* Content — markdown textarea (future Plate.js swap point) */}
      <Separator className="my-5" />

      <FieldWrapper
        label="Content"
        required
        error={errors.descriptionMarkdown}
        hint="Markdown supported"
        className="mb-0"
      >
        <Textarea
          value={values.descriptionMarkdown}
          onChange={(e) => update("descriptionMarkdown", e.target.value)}
          placeholder="Write your content in Markdown..."
          rows={20}
          disabled={disabled}
          className="font-mono text-sm leading-relaxed"
        />
      </FieldWrapper>
    </div>
  );
}
