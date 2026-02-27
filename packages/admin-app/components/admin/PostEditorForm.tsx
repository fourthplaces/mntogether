"use client";

import { useQuery } from "urql";
import { Input, Textarea } from "@/components/ui/Input";
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

  const selectClasses =
    "w-full px-3 py-2 text-sm bg-surface-subtle border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-focus-ring focus:border-transparent transition-all duration-150";

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
          className={`w-full text-2xl font-semibold text-text-primary bg-transparent border-0 border-b-2 px-0 py-2 focus:outline-none transition-colors placeholder:text-text-faint ${
            errors.title
              ? "border-danger"
              : "border-transparent focus:border-admin-accent"
          }`}
        />
        {errors.title && (
          <p className="text-xs text-danger-text mt-1">{errors.title}</p>
        )}
      </div>

      {/* Metadata section */}
      <hr className="border-border-subtle my-5" />

      <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-1">
        <FieldWrapper label="Post Type" className="mb-0">
          <select
            value={values.postType}
            onChange={(e) => update("postType", e.target.value)}
            className={selectClasses}
            disabled={disabled}
          >
            {POST_TYPES.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
        </FieldWrapper>

        <FieldWrapper label="Weight" className="mb-0">
          <select
            value={values.weight}
            onChange={(e) => update("weight", e.target.value)}
            className={selectClasses}
            disabled={disabled}
          >
            {WEIGHTS.map((w) => (
              <option key={w.value} value={w.value}>
                {w.label}
              </option>
            ))}
          </select>
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
          <select
            value={values.urgency}
            onChange={(e) => update("urgency", e.target.value)}
            className={selectClasses}
            disabled={disabled}
          >
            {URGENCIES.map((u) => (
              <option key={u.value} value={u.value}>
                {u.label}
              </option>
            ))}
          </select>
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
          <select
            value={values.organizationId}
            onChange={(e) => update("organizationId", e.target.value)}
            className={selectClasses}
            disabled={disabled}
          >
            <option value="">None</option>
            {organizations.map((org) => (
              <option key={org.id} value={org.id}>
                {org.name}
              </option>
            ))}
          </select>
        </FieldWrapper>
      </div>

      {/* Summary */}
      <hr className="border-border-subtle my-5" />

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
      <hr className="border-border-subtle my-5" />

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
