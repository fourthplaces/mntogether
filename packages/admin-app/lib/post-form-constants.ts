// Shared constants and types for post forms (PostForm + PostEditorForm)
// POST_TYPES and WEIGHTS are also used on the detail page for inline dropdowns.

export const POST_TYPES = [
  { value: "story", label: "Story" },
  { value: "notice", label: "Notice" },
  { value: "exchange", label: "Exchange" },
  { value: "event", label: "Event" },
  { value: "spotlight", label: "Spotlight" },
  { value: "reference", label: "Reference" },
] as const;

export const WEIGHTS = [
  { value: "heavy", label: "Heavy" },
  { value: "medium", label: "Medium" },
  { value: "light", label: "Light" },
] as const;

export interface PostFormValues {
  title: string;
  descriptionMarkdown: string;
  summary: string;
}

export const DEFAULT_VALUES: PostFormValues = {
  title: "",
  descriptionMarkdown: "",
  summary: "",
};

export function validatePostForm(values: PostFormValues): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!values.title.trim()) errors.title = "Title is required";
  if (!values.descriptionMarkdown.trim())
    errors.descriptionMarkdown = "Content is required";
  return errors;
}
