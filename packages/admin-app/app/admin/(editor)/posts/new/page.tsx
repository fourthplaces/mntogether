"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { useMutation } from "urql";
import { EditorTopBar } from "@/components/admin/EditorTopBar";
import { PostEditorForm } from "@/components/admin/PostEditorForm";
import { CreatePostMutation } from "@/lib/graphql/posts";
import {
  type PostFormValues,
  DEFAULT_VALUES,
  validatePostForm,
} from "@/lib/post-form-constants";

const mutationContext = {
  additionalTypenames: ["Post", "PostConnection", "PostStats"],
};

export default function NewPostPage() {
  const router = useRouter();

  // Form state
  const [values, setValues] = useState<PostFormValues>(DEFAULT_VALUES);
  const [dirty, setDirty] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Mutation
  const [{ fetching: saving }, createPost] = useMutation(CreatePostMutation);

  const handleChange = useCallback((newValues: PostFormValues) => {
    setValues(newValues);
    setDirty(true);
    setErrors({});
  }, []);

  const handleSave = useCallback(async () => {
    const validationErrors = validatePostForm(values);
    if (Object.keys(validationErrors).length > 0) {
      setErrors(validationErrors);
      return;
    }

    const result = await createPost(
      {
        input: {
          title: values.title.trim(),
          bodyRaw: values.bodyRaw.trim(),
          postType: values.postType,
          weight: values.weight,
          priority: values.priority,
          urgency: values.urgency || undefined,
          location: values.location.trim() || undefined,
          organizationId: values.organizationId || undefined,
        },
      },
      mutationContext
    );

    if (result.data?.createPost?.id) {
      setDirty(false);
      router.push(`/admin/posts/${result.data.createPost.id}`);
    }
  }, [values, createPost, router]);

  // Warn on navigation with unsaved changes
  useEffect(() => {
    const handler = (e: BeforeUnloadEvent) => {
      if (dirty) {
        e.preventDefault();
      }
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [dirty]);

  return (
    <>
      <EditorTopBar
        title={values.title || "New Post"}
        backHref="/admin/posts"
        backLabel="Posts"
        onSave={handleSave}
        saving={saving}
        dirty={dirty}
      />
      <div className="flex-1 overflow-y-auto">
        <PostEditorForm
          values={values}
          onChange={handleChange}
          errors={errors}
          disabled={saving}
        />
      </div>
    </>
  );
}
