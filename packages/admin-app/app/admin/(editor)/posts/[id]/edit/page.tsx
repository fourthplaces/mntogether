"use client";

import { useState, useEffect, useCallback, useRef } from "react";
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
// InlineField — text input styled as broadsheet content
// ---------------------------------------------------------------------------

function InlineField({
  className = "",
  value,
  onChange,
  placeholder,
  multiline = false,
}: {
  className?: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  multiline?: boolean;
}) {
  if (multiline) {
    return (
      <div className={`editable-region ${className}`}>
        <textarea
          className="inline-field"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          rows={1}
          style={{ minHeight: "1.4em" }}
          onInput={(e) => {
            // Auto-resize textarea
            const el = e.currentTarget;
            el.style.height = "auto";
            el.style.height = el.scrollHeight + "px";
          }}
        />
      </div>
    );
  }
  return (
    <div className={`editable-region ${className}`}>
      <input
        type="text"
        className="inline-field"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// HeroPhotoField — image with inline-editable caption/credit
// ---------------------------------------------------------------------------

function HeroPhotoField({
  imageUrl,
  caption,
  credit,
  onImageUrlChange,
  onCaptionChange,
  onCreditChange,
}: {
  imageUrl: string;
  caption: string;
  credit: string;
  onImageUrlChange: (v: string) => void;
  onCaptionChange: (v: string) => void;
  onCreditChange: (v: string) => void;
}) {
  return (
    <figure className="photo-a editable-region">
      {imageUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={imageUrl} alt={caption || ""} />
      ) : (
        <div
          style={{
            width: "100%",
            height: "200px",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            background: "rgba(0,0,0,0.04)",
            border: "1px dashed rgba(0,0,0,0.15)",
            color: "var(--pebble)",
            fontFamily: "var(--font-mono)",
            fontSize: "0.82rem",
          }}
        >
          No hero image
        </div>
      )}
      <figcaption className="photo-a__caption">
        <input
          type="text"
          className="inline-field photo-a__caption-text"
          value={caption}
          onChange={(e) => onCaptionChange(e.target.value)}
          placeholder="Caption"
        />
        <input
          type="text"
          className="inline-field photo-a__credit"
          value={credit}
          onChange={(e) => onCreditChange(e.target.value)}
          placeholder="Credit"
          style={{ textAlign: "right", maxWidth: "200px" }}
        />
      </figcaption>
      <input
        type="text"
        className="inline-field"
        value={imageUrl}
        onChange={(e) => onImageUrlChange(e.target.value)}
        placeholder="Image URL"
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: "0.72rem",
          color: "var(--pebble)",
          marginTop: "4px",
          borderBottom: "1px dashed rgba(0,0,0,0.15)",
          paddingBottom: "4px",
        }}
      />
    </figure>
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
  return (
    <div className="person-quote editable-region">
      <div className="person-quote__text">
        <textarea
          className="inline-field"
          value={quote}
          onChange={(e) => onQuoteChange(e.target.value)}
          placeholder="A memorable quote..."
          rows={2}
          style={{
            font: "inherit",
            minHeight: "2.6em",
          }}
          onInput={(e) => {
            const el = e.currentTarget;
            el.style.height = "auto";
            el.style.height = el.scrollHeight + "px";
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

  // Parsed body_ast from GraphQL (comes as JSON string)
  const [parsedInitialAst, setParsedInitialAst] = useState<Value | null>(null);
  const [initialMarkdown, setInitialMarkdown] = useState("");

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

      // Parse body AST from GraphQL (stored as JSON string)
      if (post.bodyAst) {
        try {
          const parsed = JSON.parse(post.bodyAst);
          setParsedInitialAst(parsed);
        } catch {
          console.warn("Failed to parse bodyAst JSON");
        }
      }
      setInitialMarkdown(post.descriptionMarkdown || post.description || "");
    }
  }, [post]);

  // Mutations
  const [{ fetching: saving }, updatePost] = useMutation(UpdatePostMutation);
  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, upsertMedia] = useMutation(UpsertPostMediaMutation);
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
          descriptionMarkdown: values.descriptionMarkdown.trim() || undefined,
          bodyAst: bodyAst ? JSON.stringify(bodyAst) : undefined,
          summary: values.summary.trim() || undefined,
        },
      },
      mutationContext
    );

    // Save field groups (parallel)
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
  }, [values, fieldGroups, bodyAst, postId, updatePost, upsertMedia, upsertMeta, upsertPerson]);

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
  const isSpotlight = post.postType === "spotlight";
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
            multiline
          />

          {/* Article meta (byline) */}
          <div className="article-meta">
            <InlineField
              value={fieldGroups.byline}
              onChange={(v) => updateFieldGroup("byline", v)}
              placeholder="Byline"
            />
          </div>

          {/* Hero photo */}
          <HeroPhotoField
            imageUrl={fieldGroups.imageUrl}
            caption={fieldGroups.caption}
            credit={fieldGroups.credit}
            onImageUrlChange={(v) => updateFieldGroup("imageUrl", v)}
            onCaptionChange={(v) => updateFieldGroup("caption", v)}
            onCreditChange={(v) => updateFieldGroup("credit", v)}
          />

          {/* Body — Plate.js WYSIWYG with broadsheet styling */}
          <PlateEditor
            initialValue={parsedInitialAst}
            initialMarkdown={initialMarkdown}
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
