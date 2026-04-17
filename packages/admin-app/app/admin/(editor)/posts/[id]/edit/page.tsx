"use client";

import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import type { Value } from "platejs";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { EditorTopBar } from "@/components/admin/EditorTopBar";
import { PlateEditor } from "@/components/admin/PlateEditor";
import "@/app/editor-broadsheet.css";
import {
  PostDetailQuery,
  UpdatePostMutation,
  ApprovePostMutation,
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
// InlineField — text input styled as broadsheet content
// ---------------------------------------------------------------------------

function InlineField({
  className = "",
  value,
  onChange,
  placeholder,
}: {
  className?: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = "auto";
      el.style.height = el.scrollHeight + "px";
    }
  }, [value]);

  return (
    <div className={`editable-region ${className}`}>
      <textarea
        ref={textareaRef}
        className="inline-field"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        rows={1}
        style={{ minHeight: "1.4em" }}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// PersonQuoteField — editable pull quote for spotlights
// ---------------------------------------------------------------------------

function PersonQuoteField({
  name,
  role,
  quote,
  onNameChange,
  onRoleChange,
  onQuoteChange,
}: {
  name: string;
  role: string;
  quote: string;
  onNameChange: (v: string) => void;
  onRoleChange: (v: string) => void;
  onQuoteChange: (v: string) => void;
}) {
  const quoteRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const el = quoteRef.current;
    if (el) {
      el.style.height = "auto";
      el.style.height = el.scrollHeight + "px";
    }
  }, [quote]);

  return (
    <div className="person-quote editable-region">
      <div className="person-quote__text">
        <textarea
          ref={quoteRef}
          className="inline-field"
          value={quote}
          onChange={(e) => onQuoteChange(e.target.value)}
          placeholder="A memorable quote..."
          rows={2}
          style={{
            font: "inherit",
            minHeight: "2.6em",
          }}
        />
      </div>
      <div className="person-quote__attribution">
        <input
          type="text"
          className="inline-field"
          value={name}
          onChange={(e) => onNameChange(e.target.value)}
          placeholder="Name"
          style={{ display: "inline", width: "auto", maxWidth: "200px" }}
        />
        {(name || role) && <span style={{ margin: "0 0.4em" }}>·</span>}
        <input
          type="text"
          className="inline-field"
          value={role}
          onChange={(e) => onRoleChange(e.target.value)}
          placeholder="Role"
          style={{ display: "inline", width: "auto", maxWidth: "200px" }}
        />
      </div>
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

  // Form state
  const [values, setValues] = useState<PostFormValues>(DEFAULT_VALUES);
  const [fieldGroups, setFieldGroups] = useState<FieldGroupState>(DEFAULT_FIELD_GROUPS);
  const [bodyAst, setBodyAst] = useState<Value | null>(null);
  const [dirty, setDirty] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const initialized = useRef(false);

  // Parse body_ast synchronously from GraphQL (comes as JSON string).
  // If no body_ast exists, fall back to description text as paragraphs.
  const parsedInitialAst = useMemo(() => {
    if (post?.bodyAst) {
      try { return JSON.parse(post.bodyAst) as Value; }
      catch { /* fall through */ }
    }
    // Convert plain text description to Plate paragraph nodes
    const text = post?.bodyRaw;
    if (text) {
      return text.split(/\n\n+/).filter(Boolean).map((para) => ({
        type: "p" as const,
        children: [{ text: para.trim() }],
      })) as Value;
    }
    return null;
  }, [post?.bodyAst, post?.bodyRaw]);

  // Initialize form values from loaded post
  useEffect(() => {
    if (post && !initialized.current) {
      initialized.current = true;
      setValues({
        ...DEFAULT_VALUES,
        title: post.title || "",
        bodyRaw: post.bodyRaw || "",
      });
      setFieldGroups({
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
  const [, upsertMeta] = useMutation(UpsertPostMetaMutation);
  const [, upsertPerson] = useMutation(UpsertPostPersonMutation);

  const handleBodyAstChange = useCallback((value: Value) => {
    setBodyAst(value);
    setDirty(true);
  }, []);

  const handleTitleChange = useCallback((v: string) => {
    setValues((prev) => ({ ...prev, title: v }));
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

    // Save base fields + body AST
    const result = await updatePost(
      {
        id: postId,
        input: {
          title: values.title.trim(),
          bodyRaw: values.bodyRaw.trim() || undefined,
          bodyAst: bodyAst
            ? JSON.stringify(bodyAst)
            : (parsedInitialAst ? JSON.stringify(parsedInitialAst) : undefined),
        },
      },
      mutationContext
    );

    // Save field groups (parallel)
    const fg = fieldGroups;
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
  }, [values, fieldGroups, bodyAst, parsedInitialAst, postId, updatePost, upsertMeta, upsertPerson]);

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
          <a href="/admin/posts" className="text-admin-accent hover:underline text-sm">
            Back to Posts
          </a>
        </div>
      </div>
    );
  }

  const showPublish = post.status === "draft";
  // "spotlight" was renamed to "person" in the 9-type enum (migration 216).
  const isSpotlight = post.postType === "person";
  const postType = post.postType || "story";
  const titleSizeClass = `title-a--${postType}`;

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
        previewUrl={`http://localhost:3001/posts/${postId}`}
      />

      <div className="editor-broadsheet flex-1 overflow-y-auto">
        <div className="editor-article-column">
          {/* Kicker */}
          <InlineField
            className="kicker-a"
            value={fieldGroups.kicker}
            onChange={(v) => updateFieldGroup("kicker", v)}
            placeholder="Section kicker"
          />

          {/* Title */}
          <InlineField
            className={`title-a ${titleSizeClass}`}
            value={values.title}
            onChange={handleTitleChange}
            placeholder="Headline"
          />

          {/* Deck */}
          <InlineField
            className="title-a__deck"
            value={fieldGroups.deck}
            onChange={(v) => updateFieldGroup("deck", v)}
            placeholder="Subtitle or subheadline"
          />

          {/* Article meta (byline) */}
          <div className="article-meta">
            <InlineField
              value={fieldGroups.byline}
              onChange={(v) => updateFieldGroup("byline", v)}
              placeholder="Byline"
            />
          </div>

          {/* Body — Plate.js WYSIWYG with broadsheet styling */}
          <PlateEditor
            key={postId}
            initialValue={parsedInitialAst}
            onChange={handleBodyAstChange}
            placeholder="Write your story..."
            disabled={saving}
          />

          {/* Person pull quote (spotlight only) */}
          {isSpotlight && (
            <PersonQuoteField
              name={fieldGroups.personName}
              role={fieldGroups.personRole}
              quote={fieldGroups.personQuote}
              onNameChange={(v) => updateFieldGroup("personName", v)}
              onRoleChange={(v) => updateFieldGroup("personRole", v)}
              onQuoteChange={(v) => updateFieldGroup("personQuote", v)}
            />
          )}
        </div>
      </div>
    </>
  );
}
