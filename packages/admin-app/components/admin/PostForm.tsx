"use client";

import { useState } from "react";
import { useQuery } from "urql";
import { Button } from "@/components/ui/Button";
import { Input, Textarea } from "@/components/ui/Input";
import { FieldWrapper } from "@/components/admin/FieldWrapper";
import { OrganizationsListQuery } from "@/lib/graphql/organizations";
import {
  POST_TYPES,
  WEIGHTS,
  URGENCIES,
  type PostFormValues,
} from "@/lib/post-form-constants";

export type { PostFormValues };

interface PostFormProps {
  initialValues?: Partial<PostFormValues>;
  onSubmit: (values: PostFormValues) => Promise<void>;
  onCancel?: () => void;
  loading?: boolean;
}

export function PostForm({
  initialValues,
  onSubmit,
  onCancel,
  loading,
}: PostFormProps) {
  const [title, setTitle] = useState(initialValues?.title ?? "");
  const [descriptionMarkdown, setDescriptionMarkdown] = useState(
    initialValues?.descriptionMarkdown ?? ""
  );
  const [summary, setSummary] = useState(initialValues?.summary ?? "");
  const [postType, setPostType] = useState(
    initialValues?.postType ?? "notice"
  );
  const [weight, setWeight] = useState(initialValues?.weight ?? "medium");
  const [priority, setPriority] = useState(initialValues?.priority ?? 0);
  const [urgency, setUrgency] = useState(initialValues?.urgency ?? "");
  const [location, setLocation] = useState(initialValues?.location ?? "");
  const [organizationId, setOrganizationId] = useState(
    initialValues?.organizationId ?? ""
  );
  const [errors, setErrors] = useState<Record<string, string>>({});

  const [{ data: orgsData }] = useQuery({ query: OrganizationsListQuery });
  const organizations = orgsData?.organizations ?? [];

  function validate(): boolean {
    const newErrors: Record<string, string> = {};
    if (!title.trim()) newErrors.title = "Title is required";
    if (!descriptionMarkdown.trim())
      newErrors.descriptionMarkdown = "Content is required";
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!validate()) return;
    await onSubmit({
      title: title.trim(),
      descriptionMarkdown: descriptionMarkdown.trim(),
      summary: summary.trim(),
      postType,
      weight,
      priority,
      urgency,
      location: location.trim(),
      organizationId,
    });
  }

  const selectClasses =
    "w-full px-4 py-2.5 text-sm bg-surface-subtle border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-focus-ring focus:border-transparent transition-all duration-150";

  return (
    <form onSubmit={handleSubmit}>
      <FieldWrapper label="Title" required error={errors.title}>
        <Input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="Post title"
          disabled={loading}
        />
      </FieldWrapper>

      <FieldWrapper
        label="Content"
        required
        error={errors.descriptionMarkdown}
        hint="Markdown supported. A rich editor is coming soon."
      >
        <Textarea
          value={descriptionMarkdown}
          onChange={(e) => setDescriptionMarkdown(e.target.value)}
          placeholder="Write your content in Markdown..."
          rows={12}
          disabled={loading}
          className="font-mono text-sm"
        />
      </FieldWrapper>

      <FieldWrapper
        label="Summary"
        hint="Brief plain-text summary for cards and previews"
      >
        <Textarea
          value={summary}
          onChange={(e) => setSummary(e.target.value)}
          placeholder="Optional summary..."
          rows={3}
          disabled={loading}
        />
      </FieldWrapper>

      <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6">
        <FieldWrapper label="Post Type" className="mb-0">
          <select
            value={postType}
            onChange={(e) => setPostType(e.target.value)}
            className={selectClasses}
            disabled={loading}
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
            value={weight}
            onChange={(e) => setWeight(e.target.value)}
            className={selectClasses}
            disabled={loading}
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
            value={priority}
            onChange={(e) => setPriority(Number(e.target.value))}
            disabled={loading}
          />
        </FieldWrapper>
      </div>

      <div className="grid grid-cols-2 gap-4 mb-6">
        <FieldWrapper label="Urgency" className="mb-0">
          <select
            value={urgency}
            onChange={(e) => setUrgency(e.target.value)}
            className={selectClasses}
            disabled={loading}
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
            value={location}
            onChange={(e) => setLocation(e.target.value)}
            placeholder="e.g. Minneapolis, MN"
            disabled={loading}
          />
        </FieldWrapper>
      </div>

      <FieldWrapper label="Organization">
        <select
          value={organizationId}
          onChange={(e) => setOrganizationId(e.target.value)}
          className={selectClasses}
          disabled={loading}
        >
          <option value="">None</option>
          {organizations.map((org) => (
            <option key={org.id} value={org.id}>
              {org.name}
            </option>
          ))}
        </select>
      </FieldWrapper>

      <div className="flex items-center gap-3 pt-4 border-t border-border">
        <Button type="submit" variant="primary" loading={loading}>
          {initialValues ? "Save Changes" : "Create Draft"}
        </Button>
        {onCancel && (
          <Button
            type="button"
            variant="ghost"
            onClick={onCancel}
            disabled={loading}
          >
            Cancel
          </Button>
        )}
      </div>
    </form>
  );
}
