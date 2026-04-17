// Shared constants and types for post forms (PostForm + PostEditorForm)
// POST_TYPES and WEIGHTS are also used on the detail page for inline dropdowns.

// Kept in sync with the 9-type enum from migration 216. Order reflects
// rough "weight" in the newsroom: stories lead, references tail.
export const POST_TYPES = [
  { value: "story", label: "Story" },
  { value: "update", label: "Update" },
  { value: "action", label: "Action" },
  { value: "event", label: "Event" },
  { value: "need", label: "Need" },
  { value: "aid", label: "Aid" },
  { value: "person", label: "Person" },
  { value: "business", label: "Business" },
  { value: "reference", label: "Reference" },
] as const;

export const WEIGHTS = [
  { value: "heavy", label: "Heavy" },
  { value: "medium", label: "Medium" },
  { value: "light", label: "Light" },
] as const;

export interface PostFormValues {
  title: string;
  bodyRaw: string;
  postType: string;
  weight: string;
  priority: number;
  isUrgent: boolean;
  location: string;
  organizationId: string;
}

export const DEFAULT_VALUES: PostFormValues = {
  title: "",
  bodyRaw: "",
  postType: "update",
  weight: "medium",
  priority: 0,
  isUrgent: false,
  location: "",
  organizationId: "",
};

export function validatePostForm(values: PostFormValues): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!values.title.trim()) errors.title = "Title is required";
  if (!values.bodyRaw.trim())
    errors.bodyRaw = "Content is required";
  return errors;
}
