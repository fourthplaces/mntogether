"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { EditorTopBar } from "@/components/admin/EditorTopBar";
import { SplitPane, type SplitMode } from "@/components/admin/SplitPane";
import { PostEditorForm } from "@/components/admin/PostEditorForm";
import { MarkdownPreview } from "@/components/admin/MarkdownPreview";
import { PostDetailQuery, UpdatePostMutation, ApprovePostMutation } from "@/lib/graphql/posts";
import {
  type PostFormValues,
  DEFAULT_VALUES,
  validatePostForm,
} from "@/lib/post-form-constants";

const mutationContext = {
  additionalTypenames: ["Post", "PostConnection", "PostStats"],
};

export default function EditPostPage() {
  const params = useParams();
  const router = useRouter();
  const postId = params.id as string;

  // Load post data
  const [{ data, fetching, error }] = useQuery({
    query: PostDetailQuery,
    variables: { id: postId },
  });
  const post = data?.post;

  // Form state
  const [values, setValues] = useState<PostFormValues>(DEFAULT_VALUES);
  const [dirty, setDirty] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [mode, setMode] = useState<SplitMode>("split");
  const initialized = useRef(false);

  // Initialize form values from loaded post
  useEffect(() => {
    if (post && !initialized.current) {
      initialized.current = true;
      setValues({
        ...DEFAULT_VALUES,
        title: post.title || "",
        descriptionMarkdown: post.descriptionMarkdown || post.description || "",
        summary: post.summary || "",
      });
    }
  }, [post]);

  // Mutations
  const [{ fetching: saving }, updatePost] = useMutation(UpdatePostMutation);
  const [, approvePost] = useMutation(ApprovePostMutation);

  const handleChange = useCallback((newValues: PostFormValues) => {
    setValues(newValues);
    setDirty(true);
    // Clear field errors as user types
    setErrors({});
  }, []);

  const handleSave = useCallback(async () => {
    const validationErrors = validatePostForm(values);
    if (Object.keys(validationErrors).length > 0) {
      setErrors(validationErrors);
      return;
    }

    const result = await updatePost(
      {
        id: postId,
        input: {
          title: values.title.trim(),
          descriptionMarkdown: values.descriptionMarkdown.trim(),
          summary: values.summary.trim() || undefined,
        },
      },
      mutationContext
    );

    if (!result.error) {
      setDirty(false);
    }
  }, [values, postId, updatePost]);

  const handlePublish = useCallback(async () => {
    // Save first, then approve
    await handleSave();
    await approvePost({ id: postId }, mutationContext);
    router.push(`/admin/posts/${postId}`);
  }, [handleSave, approvePost, postId, router]);

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

  // Force editor-only on small screens
  useEffect(() => {
    const mq = window.matchMedia("(max-width: 768px)");
    const handler = (e: MediaQueryListEvent) => {
      if (e.matches) setMode("editor");
    };
    if (mq.matches) setMode("editor");
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  if (fetching) {
    return (
      <div className="h-screen flex items-center justify-center">
        <AdminLoader label="Loading post..." />
      </div>
    );
  }

  if (error || !post) {
    return (
      <div className="h-screen flex items-center justify-center">
        <div className="text-center">
          <h1 className="text-xl font-semibold text-text-primary mb-2">
            {error ? "Error loading post" : "Post not found"}
          </h1>
          <p className="text-sm text-text-muted mb-4">
            {error?.message || "The post you're looking for doesn't exist."}
          </p>
          <a href={`/admin/posts`} className="text-admin-accent hover:underline text-sm">
            Back to Posts
          </a>
        </div>
      </div>
    );
  }

  const showPublish = post.status === "draft";

  return (
    <>
      <EditorTopBar
        title={values.title}
        status={post.status}
        backHref={`/admin/posts/${postId}`}
        backLabel="Back to post"
        onSave={handleSave}
        onPublish={showPublish ? handlePublish : undefined}
        saving={saving}
        dirty={dirty}
        mode={mode}
        onModeChange={setMode}
      />
      <SplitPane
        mode={mode}
        left={
          <PostEditorForm
            values={values}
            onChange={handleChange}
            errors={errors}
            disabled={saving}
          />
        }
        right={
          <MarkdownPreview
            markdown={values.descriptionMarkdown}
            title={values.title}
          />
        }
      />
    </>
  );
}
