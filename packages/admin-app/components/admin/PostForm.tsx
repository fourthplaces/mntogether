"use client";

import { useState } from "react";
import { useQuery } from "urql";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
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
  const [bodyRaw, setBodyRaw] = useState(
    initialValues?.bodyRaw ?? ""
  );
  const [postType, setPostType] = useState(
    initialValues?.postType ?? "notice"
  );
  const [weight, setWeight] = useState(initialValues?.weight ?? "medium");
  const [priority, setPriority] = useState(initialValues?.priority ?? 0);
  const [isUrgent, setIsUrgent] = useState(initialValues?.isUrgent ?? false);
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
    if (!bodyRaw.trim())
      newErrors.bodyRaw = "Content is required";
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!validate()) return;
    await onSubmit({
      title: title.trim(),
      bodyRaw: bodyRaw.trim(),
      postType,
      weight,
      priority,
      isUrgent,
      location: location.trim(),
      organizationId,
    });
  }

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
        error={errors.bodyRaw}
        hint="Markdown supported. A rich editor is coming soon."
      >
        <Textarea
          value={bodyRaw}
          onChange={(e) => setBodyRaw(e.target.value)}
          placeholder="Write your content in Markdown..."
          rows={12}
          disabled={loading}
          className="font-mono text-sm"
        />
      </FieldWrapper>

      <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6">
        <FieldWrapper label="Post Type" className="mb-0">
          <Select value={postType} onValueChange={(val) => val !== null && setPostType(val)} disabled={loading}>
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
          <Select value={weight} onValueChange={(val) => val !== null && setWeight(val)} disabled={loading}>
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
            value={priority}
            onChange={(e) => setPriority(Number(e.target.value))}
            disabled={loading}
          />
        </FieldWrapper>
      </div>

      <div className="grid grid-cols-2 gap-4 mb-6">
        <FieldWrapper label="Urgent" className="mb-0">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={isUrgent}
              onChange={(e) => setIsUrgent(e.target.checked)}
              disabled={loading}
              className="rounded border-border"
            />
            <span className="text-sm">Flag as urgent</span>
          </label>
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
        <Select
          value={organizationId || "__none__"}
          onValueChange={(v) => v !== null && setOrganizationId(v === "__none__" ? "" : v)}
          disabled={loading}
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

      <div className="flex items-center gap-3 pt-4 border-t border-border">
        <Button type="submit" loading={loading}>
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
