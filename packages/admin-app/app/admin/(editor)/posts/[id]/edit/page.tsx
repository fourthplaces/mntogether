"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { EditorTopBar } from "@/components/admin/EditorTopBar";
import { SplitPane, type SplitMode } from "@/components/admin/SplitPane";
import { PlateEditor } from "@/components/admin/PlateEditor";
import { ArticlePreview } from "@/components/admin/ArticlePreview";
import {
  PostDetailQuery,
  UpdatePostMutation,
  ApprovePostMutation,
  UpsertPostMediaMutation,
  UpsertPostMetaMutation,
  UpsertPostPersonMutation,
} from "@/lib/graphql/posts";
import {
  type PostFormValues,
  DEFAULT_VALUES,
  validatePostForm,
} from "@/lib/post-form-constants";

const mutationContext = {
  additionalTypenames: ["Post", "PostConnection", "PostStats"],
};

// ---------------------------------------------------------------------------
// Field group state
// ---------------------------------------------------------------------------

interface FieldGroupState {
  // Media
  imageUrl: string;
  caption: string;
  credit: string;
  // Meta
  kicker: string;
  byline: string;
  deck: string;
  // Person (spotlights)
  personName: string;
  personRole: string;
  personBio: string;
  personPhotoUrl: string;
  personQuote: string;
}

const DEFAULT_FIELD_GROUPS: FieldGroupState = {
  imageUrl: "",
  caption: "",
  credit: "",
  kicker: "",
  byline: "",
  deck: "",
  personName: "",
  personRole: "",
  personBio: "",
  personPhotoUrl: "",
  personQuote: "",
};

// ---------------------------------------------------------------------------
// Collapsible field group panel
// ---------------------------------------------------------------------------

function FieldGroupPanel({
  title,
  defaultOpen = false,
  children,
}: {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="border border-border rounded-md overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="w-full flex items-center justify-between px-3 py-2 bg-surface-raised/50 text-sm font-medium text-text-primary hover:bg-surface-muted transition-colors"
      >
        {title}
        <span className="text-text-muted text-xs">{open ? "▾" : "▸"}</span>
      </button>
      {open && <div className="px-3 py-3 space-y-3">{children}</div>}
    </div>
  );
}

function FieldInput({
  label,
  value,
  onChange,
  placeholder,
  multiline = false,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  multiline?: boolean;
}) {
  return (
    <div>
      <label className="block text-xs text-text-muted uppercase tracking-wide mb-1">
        {label}
      </label>
      {multiline ? (
        <textarea
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          rows={2}
          className="w-full rounded border border-border bg-white px-2 py-1.5 text-sm text-text-primary placeholder:text-text-muted/50 focus:outline-none focus:ring-1 focus:ring-admin-accent resize-y"
        />
      ) : (
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="w-full rounded border border-border bg-white px-2 py-1.5 text-sm text-text-primary placeholder:text-text-muted/50 focus:outline-none focus:ring-1 focus:ring-admin-accent"
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

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

  // Form state — base fields
  const [values, setValues] = useState<PostFormValues>(DEFAULT_VALUES);
  const [fieldGroups, setFieldGroups] = useState<FieldGroupState>(DEFAULT_FIELD_GROUPS);
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
      setFieldGroups({
        imageUrl: post.media?.[0]?.imageUrl || "",
        caption: post.media?.[0]?.caption || "",
        credit: post.media?.[0]?.credit || "",
        kicker: post.meta?.kicker || "",
        byline: post.meta?.byline || "",
        deck: post.meta?.deck || "",
        personName: post.person?.name || "",
        personRole: post.person?.role || "",
        personBio: post.person?.bio || "",
        personPhotoUrl: post.person?.photoUrl || "",
        personQuote: post.person?.quote || "",
      });
    }
  }, [post]);

  // Mutations
  const [{ fetching: saving }, updatePost] = useMutation(UpdatePostMutation);
  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, upsertMedia] = useMutation(UpsertPostMediaMutation);
  const [, upsertMeta] = useMutation(UpsertPostMetaMutation);
  const [, upsertPerson] = useMutation(UpsertPostPersonMutation);

  const handleMarkdownChange = useCallback((markdown: string) => {
    setValues((prev) => ({ ...prev, descriptionMarkdown: markdown }));
    setDirty(true);
  }, []);

  const handleTitleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setValues((prev) => ({ ...prev, title: e.target.value }));
    setDirty(true);
  }, []);

  const updateFieldGroup = useCallback((key: keyof FieldGroupState, value: string) => {
    setFieldGroups((prev) => ({ ...prev, [key]: value }));
    setDirty(true);
  }, []);

  const handleSave = useCallback(async () => {
    const validationErrors = validatePostForm(values);
    if (Object.keys(validationErrors).length > 0) {
      setErrors(validationErrors);
      return;
    }

    // Save base fields
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

    // Save field groups (fire and forget — parallel)
    const fg = fieldGroups;
    if (fg.imageUrl || fg.caption || fg.credit) {
      upsertMedia({
        postId,
        imageUrl: fg.imageUrl || undefined,
        caption: fg.caption || undefined,
        credit: fg.credit || undefined,
      });
    }
    if (fg.kicker || fg.byline || fg.deck) {
      upsertMeta({
        postId,
        kicker: fg.kicker || undefined,
        byline: fg.byline || undefined,
        deck: fg.deck || undefined,
      });
    }
    if (fg.personName || fg.personRole || fg.personBio || fg.personPhotoUrl || fg.personQuote) {
      upsertPerson({
        postId,
        name: fg.personName || undefined,
        role: fg.personRole || undefined,
        bio: fg.personBio || undefined,
        photoUrl: fg.personPhotoUrl || undefined,
        quote: fg.personQuote || undefined,
      });
    }

    if (!result.error) {
      setDirty(false);
    }
  }, [values, fieldGroups, postId, updatePost, upsertMedia, upsertMeta, upsertPerson]);

  const handlePublish = useCallback(async () => {
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
  const isSpotlight = post.postType === "spotlight";

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
        previewUrl={`http://localhost:3001/posts/${postId}`}
      />
      <SplitPane
        mode={mode}
        left={
          <div className="p-4 space-y-4 overflow-y-auto h-full">
            {/* Title */}
            <input
              type="text"
              value={values.title}
              onChange={handleTitleChange}
              placeholder="Post title"
              className="w-full text-2xl font-semibold text-text-primary bg-transparent border-b border-border pb-2 focus:outline-none focus:border-admin-accent placeholder:text-text-muted/40"
            />

            {/* WYSIWYG Editor */}
            <PlateEditor
              initialMarkdown={post.descriptionMarkdown || post.description || ""}
              onChange={handleMarkdownChange}
              placeholder="Write your story..."
              disabled={saving}
            />

            {/* Field group panels — content visible in article main column */}
            <div className="space-y-2 pt-2">
              <FieldGroupPanel title="Meta (Kicker, Byline, Deck)" defaultOpen={!!fieldGroups.kicker || !!fieldGroups.byline}>
                <FieldInput label="Kicker" value={fieldGroups.kicker} onChange={(v) => updateFieldGroup("kicker", v)} placeholder="Housing" />
                <FieldInput label="Byline" value={fieldGroups.byline} onChange={(v) => updateFieldGroup("byline", v)} placeholder="Root Editorial Staff" />
                <FieldInput label="Deck" value={fieldGroups.deck} onChange={(v) => updateFieldGroup("deck", v)} placeholder="Subtitle or subheadline" multiline />
              </FieldGroupPanel>

              <FieldGroupPanel title="Media (Image)" defaultOpen={!!fieldGroups.imageUrl}>
                <FieldInput label="Image URL" value={fieldGroups.imageUrl} onChange={(v) => updateFieldGroup("imageUrl", v)} placeholder="https://..." />
                <FieldInput label="Caption" value={fieldGroups.caption} onChange={(v) => updateFieldGroup("caption", v)} placeholder="Photo caption" multiline />
                <FieldInput label="Credit" value={fieldGroups.credit} onChange={(v) => updateFieldGroup("credit", v)} placeholder="Photo credit" />
              </FieldGroupPanel>

              {isSpotlight && (
                <FieldGroupPanel title="Person Profile" defaultOpen={!!fieldGroups.personName}>
                  <FieldInput label="Name" value={fieldGroups.personName} onChange={(v) => updateFieldGroup("personName", v)} placeholder="Full name" />
                  <FieldInput label="Role" value={fieldGroups.personRole} onChange={(v) => updateFieldGroup("personRole", v)} placeholder="Community Organizer" />
                  <FieldInput label="Bio" value={fieldGroups.personBio} onChange={(v) => updateFieldGroup("personBio", v)} placeholder="Short biography" multiline />
                  <FieldInput label="Photo URL" value={fieldGroups.personPhotoUrl} onChange={(v) => updateFieldGroup("personPhotoUrl", v)} placeholder="https://..." />
                  <FieldInput label="Quote" value={fieldGroups.personQuote} onChange={(v) => updateFieldGroup("personQuote", v)} placeholder="A memorable quote" multiline />
                </FieldGroupPanel>
              )}
            </div>
          </div>
        }
        right={
          <div className="overflow-y-auto h-full">
            <ArticlePreview
              title={values.title}
              markdown={values.descriptionMarkdown}
              postType={post.postType ?? undefined}
              kicker={fieldGroups.kicker}
              byline={fieldGroups.byline}
              deck={fieldGroups.deck}
              imageUrl={fieldGroups.imageUrl}
              caption={fieldGroups.caption}
              credit={fieldGroups.credit}
              personName={fieldGroups.personName}
              personRole={fieldGroups.personRole}
              personQuote={fieldGroups.personQuote}
            />
          </div>
        }
      />
    </>
  );
}
