// Shared constants and types for post forms (PostForm + PostEditorForm)

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

export const URGENCIES = [
  { value: "", label: "None" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "urgent", label: "Urgent" },
] as const;

export interface PostFormValues {
  title: string;
  descriptionMarkdown: string;
  summary: string;
  postType: string;
  weight: string;
  priority: number;
  urgency: string;
  location: string;
  organizationId: string;
}

export const DEFAULT_VALUES: PostFormValues = {
  title: "",
  descriptionMarkdown: "",
  summary: "",
  postType: "notice",
  weight: "medium",
  priority: 0,
  urgency: "",
  location: "",
  organizationId: "",
};

export function validatePostForm(values: PostFormValues): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!values.title.trim()) errors.title = "Title is required";
  if (!values.descriptionMarkdown.trim())
    errors.descriptionMarkdown = "Content is required";
  return errors;
}
