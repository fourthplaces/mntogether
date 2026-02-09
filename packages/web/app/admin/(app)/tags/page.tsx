"use client";

import { Suspense, useState, useMemo } from "react";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type {
  TagKindListResult,
  TagKindResult,
  TagListResult,
  TagResult,
} from "@/lib/restate/types";

const RESOURCE_TYPES = [
  "post",
  "website",
  "provider",
  "container",
  "referral_document",
];

export default function TagsPage() {
  return (
    <Suspense fallback={<AdminLoader label="Loading tags..." />}>
      <TagsContent />
    </Suspense>
  );
}

function TagsContent() {
  const { data: kindsData, isLoading: kindsLoading } =
    useRestate<TagKindListResult>("Tags", "list_kinds", {});
  const { data: tagsData, isLoading: tagsLoading } =
    useRestate<TagListResult>("Tags", "list_tags", {});

  const [showAddKind, setShowAddKind] = useState(false);
  const [expandedKinds, setExpandedKinds] = useState<Set<string>>(new Set());

  const tagsByKind = useMemo(() => {
    const map: Record<string, TagResult[]> = {};
    for (const tag of tagsData?.tags || []) {
      if (!map[tag.kind]) map[tag.kind] = [];
      map[tag.kind].push(tag);
    }
    return map;
  }, [tagsData]);

  const toggleKind = (slug: string) => {
    setExpandedKinds((prev) => {
      const next = new Set(prev);
      if (next.has(slug)) next.delete(slug);
      else next.add(slug);
      return next;
    });
  };

  if (kindsLoading || tagsLoading) {
    return <AdminLoader label="Loading tags..." />;
  }

  const kinds = kindsData?.kinds || [];

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-5xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-3xl font-bold text-stone-900">Tags</h1>
          <button
            onClick={() => setShowAddKind(!showAddKind)}
            className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors"
          >
            + Add Kind
          </button>
        </div>

        {showAddKind && (
          <AddKindForm onClose={() => setShowAddKind(false)} />
        )}

        <div className="space-y-3">
          {kinds.map((kind) => (
            <KindSection
              key={kind.id}
              kind={kind}
              tags={tagsByKind[kind.slug] || []}
              expanded={expandedKinds.has(kind.slug)}
              onToggle={() => toggleKind(kind.slug)}
            />
          ))}
        </div>

        {kinds.length === 0 && (
          <div className="text-stone-500 text-center py-12">
            No tag kinds found
          </div>
        )}
      </div>
    </div>
  );
}

// =============================================================================
// Add Kind Form
// =============================================================================

function AddKindForm({ onClose }: { onClose: () => void }) {
  const [slug, setSlug] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [description, setDescription] = useState("");
  const [resourceTypes, setResourceTypes] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const toggleResource = (rt: string) => {
    setResourceTypes((prev) =>
      prev.includes(rt) ? prev.filter((r) => r !== rt) : [...prev, rt]
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!slug.trim() || !displayName.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await callService("Tags", "create_kind", {
        slug: slug.trim(),
        display_name: displayName.trim(),
        description: description.trim() || null,
        allowed_resource_types: resourceTypes,
      });
      invalidateService("Tags");
      onClose();
    } catch (err: any) {
      setError(err.message || "Failed to create kind");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="bg-white rounded-lg shadow px-4 py-4 mb-6 space-y-3"
    >
      <div className="text-sm font-medium text-stone-700">New Tag Kind</div>
      <div className="grid grid-cols-2 gap-3">
        <input
          type="text"
          value={slug}
          onChange={(e) => setSlug(e.target.value)}
          placeholder="slug (e.g. my_kind)"
          className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
          autoFocus
          disabled={loading}
        />
        <input
          type="text"
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          placeholder="Display Name"
          className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
          disabled={loading}
        />
      </div>
      <input
        type="text"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        placeholder="Description (optional)"
        className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      <div>
        <div className="text-xs text-stone-500 mb-1">
          Allowed resource types:
        </div>
        <div className="flex flex-wrap gap-2">
          {RESOURCE_TYPES.map((rt) => (
            <label
              key={rt}
              className="flex items-center gap-1 text-sm text-stone-700"
            >
              <input
                type="checkbox"
                checked={resourceTypes.includes(rt)}
                onChange={() => toggleResource(rt)}
                disabled={loading}
                className="rounded border-stone-300 text-amber-600 focus:ring-amber-500"
              />
              {rt}
            </label>
          ))}
        </div>
      </div>
      <div className="flex items-center gap-2">
        <button
          type="submit"
          disabled={loading || !slug.trim() || !displayName.trim()}
          className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
        >
          {loading ? "Creating..." : "Create Kind"}
        </button>
        <button
          type="button"
          onClick={onClose}
          className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
        >
          Cancel
        </button>
        {error && <span className="text-red-600 text-sm">{error}</span>}
      </div>
    </form>
  );
}

// =============================================================================
// Kind Section
// =============================================================================

function KindSection({
  kind,
  tags,
  expanded,
  onToggle,
}: {
  kind: TagKindResult;
  tags: TagResult[];
  expanded: boolean;
  onToggle: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [showAddTag, setShowAddTag] = useState(false);

  const handleDeleteKind = async () => {
    try {
      await callService("Tags", "delete_kind", { id: kind.id });
      invalidateService("Tags");
    } catch (err: any) {
      console.error("Failed to delete kind:", err);
    }
  };

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-3 cursor-pointer hover:bg-stone-50"
        onClick={onToggle}
      >
        <div className="flex items-center gap-3">
          <span className="text-stone-400 text-sm">{expanded ? "▼" : "▶"}</span>
          <div>
            <span className="font-medium text-stone-900">
              {kind.display_name}
            </span>
            <span className="text-stone-400 text-sm ml-2">({kind.slug})</span>
          </div>
          <span className="text-xs bg-stone-100 text-stone-600 px-2 py-0.5 rounded-full">
            {kind.tag_count} tags
          </span>
        </div>
        <div className="flex items-center gap-1" onClick={(e) => e.stopPropagation()}>
          <button
            onClick={() => { setEditing(!editing); if (!expanded) onToggle(); }}
            className="px-2 py-1 text-xs text-stone-500 hover:text-amber-700 hover:bg-amber-50 rounded transition-colors"
          >
            Edit
          </button>
          <button
            onClick={handleDeleteKind}
            className="px-2 py-1 text-xs text-stone-500 hover:text-red-700 hover:bg-red-50 rounded transition-colors"
          >
            Delete
          </button>
        </div>
      </div>

      {/* Expanded content */}
      {expanded && (
        <div className="border-t border-stone-100 px-4 py-3">
          {/* Kind edit panel */}
          {editing && (
            <EditKindForm kind={kind} onClose={() => setEditing(false)} />
          )}

          {/* Description */}
          {kind.description && !editing && (
            <p className="text-sm text-stone-500 mb-3">{kind.description}</p>
          )}

          {/* Resource types */}
          {!editing && kind.allowed_resource_types.length > 0 && (
            <div className="flex flex-wrap gap-1 mb-3">
              {kind.allowed_resource_types.map((rt) => (
                <span
                  key={rt}
                  className="text-xs bg-amber-50 text-amber-700 px-2 py-0.5 rounded"
                >
                  {rt}
                </span>
              ))}
            </div>
          )}

          {/* Add tag */}
          <div className="flex items-center gap-2 mb-3">
            <button
              onClick={() => setShowAddTag(!showAddTag)}
              className="text-xs text-amber-600 hover:text-amber-800 font-medium"
            >
              + Add Tag
            </button>
          </div>

          {showAddTag && (
            <AddTagForm
              kindSlug={kind.slug}
              onClose={() => setShowAddTag(false)}
            />
          )}

          {/* Tag rows */}
          {tags.length > 0 ? (
            <div className="space-y-1">
              {tags.map((tag) => (
                <TagRow key={tag.id} tag={tag} />
              ))}
            </div>
          ) : (
            <p className="text-sm text-stone-400 italic">No tags yet</p>
          )}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Edit Kind Form
// =============================================================================

function EditKindForm({
  kind,
  onClose,
}: {
  kind: TagKindResult;
  onClose: () => void;
}) {
  const [displayName, setDisplayName] = useState(kind.display_name);
  const [description, setDescription] = useState(kind.description || "");
  const [resourceTypes, setResourceTypes] = useState<string[]>(
    kind.allowed_resource_types
  );
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const toggleResource = (rt: string) => {
    setResourceTypes((prev) =>
      prev.includes(rt) ? prev.filter((r) => r !== rt) : [...prev, rt]
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await callService("Tags", "update_kind", {
        id: kind.id,
        display_name: displayName.trim(),
        description: description.trim() || null,
        allowed_resource_types: resourceTypes,
      });
      invalidateService("Tags");
      onClose();
    } catch (err: any) {
      setError(err.message || "Failed to update kind");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="bg-stone-50 rounded-lg px-4 py-3 mb-3 space-y-3"
    >
      <div className="grid grid-cols-2 gap-3">
        <input
          type="text"
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          placeholder="Display Name"
          className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
          disabled={loading}
        />
        <input
          type="text"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Description (optional)"
          className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
          disabled={loading}
        />
      </div>
      <div>
        <div className="text-xs text-stone-500 mb-1">
          Allowed resource types:
        </div>
        <div className="flex flex-wrap gap-2">
          {RESOURCE_TYPES.map((rt) => (
            <label
              key={rt}
              className="flex items-center gap-1 text-sm text-stone-700"
            >
              <input
                type="checkbox"
                checked={resourceTypes.includes(rt)}
                onChange={() => toggleResource(rt)}
                disabled={loading}
                className="rounded border-stone-300 text-amber-600 focus:ring-amber-500"
              />
              {rt}
            </label>
          ))}
        </div>
      </div>
      <div className="flex items-center gap-2">
        <button
          type="submit"
          disabled={loading || !displayName.trim()}
          className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
        >
          {loading ? "Saving..." : "Save"}
        </button>
        <button
          type="button"
          onClick={onClose}
          className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
        >
          Cancel
        </button>
        {error && <span className="text-red-600 text-sm">{error}</span>}
      </div>
    </form>
  );
}

// =============================================================================
// Add Tag Form
// =============================================================================

function AddTagForm({
  kindSlug,
  onClose,
}: {
  kindSlug: string;
  onClose: () => void;
}) {
  const [value, setValue] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!value.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await callService("Tags", "create_tag", {
        kind: kindSlug,
        value: value.trim(),
        display_name: displayName.trim() || null,
      });
      invalidateService("Tags");
      setValue("");
      setDisplayName("");
      onClose();
    } catch (err: any) {
      setError(err.message || "Failed to create tag");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="flex items-center gap-2 mb-3 bg-stone-50 rounded-lg px-3 py-2"
    >
      <input
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder="value (e.g. my-tag)"
        className="px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        autoFocus
        disabled={loading}
      />
      <input
        type="text"
        value={displayName}
        onChange={(e) => setDisplayName(e.target.value)}
        placeholder="Display Name (optional)"
        className="px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      <button
        type="submit"
        disabled={loading || !value.trim()}
        className="px-3 py-1.5 bg-amber-600 text-white rounded text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
      >
        {loading ? "..." : "Add"}
      </button>
      <button
        type="button"
        onClick={onClose}
        className="text-stone-400 hover:text-stone-600 text-sm"
      >
        Cancel
      </button>
      {error && <span className="text-red-600 text-xs">{error}</span>}
    </form>
  );
}

// =============================================================================
// Tag Row
// =============================================================================

function TagRow({ tag }: { tag: TagResult }) {
  const [editing, setEditing] = useState(false);
  const [displayName, setDisplayName] = useState(tag.display_name || "");
  const [color, setColor] = useState(tag.color || "");
  const [description, setDescription] = useState(tag.description || "");
  const [emoji, setEmoji] = useState(tag.emoji || "");
  const [loading, setLoading] = useState(false);

  const handleSave = async () => {
    setLoading(true);
    try {
      await callService("Tags", "update_tag", {
        id: tag.id,
        display_name: displayName.trim(),
        color: color.trim() || null,
        description: description.trim() || null,
        emoji: emoji.trim() || null,
      });
      invalidateService("Tags");
      setEditing(false);
    } catch (err: any) {
      console.error("Failed to update tag:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async () => {
    try {
      await callService("Tags", "delete_tag", { id: tag.id });
      invalidateService("Tags");
    } catch (err: any) {
      console.error("Failed to delete tag:", err);
    }
  };

  if (editing) {
    return (
      <div className="border border-stone-200 rounded-lg p-3 space-y-3 bg-stone-50">
        <div className="flex items-center gap-2 mb-1">
          <code className="text-stone-700 bg-stone-100 px-1.5 py-0.5 rounded text-xs">
            {tag.value}
          </code>
        </div>
        <div className="grid grid-cols-4 gap-3">
          <div>
            <label className="block text-xs text-stone-500 mb-1">Display Name</label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
              autoFocus
              disabled={loading}
            />
          </div>
          <div>
            <label className="block text-xs text-stone-500 mb-1">Emoji</label>
            <input
              type="text"
              value={emoji}
              onChange={(e) => setEmoji(e.target.value)}
              placeholder="e.g. 🤲"
              className="w-full px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
              disabled={loading}
              maxLength={4}
            />
          </div>
          <div>
            <label className="block text-xs text-stone-500 mb-1">Description</label>
            <input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Brief description..."
              className="w-full px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
              disabled={loading}
            />
          </div>
          <div>
            <label className="block text-xs text-stone-500 mb-1">Color</label>
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={color || "#a8a29e"}
                onChange={(e) => setColor(e.target.value)}
                className="w-8 h-8 rounded border border-stone-300 cursor-pointer p-0"
                disabled={loading}
              />
              <input
                type="text"
                value={color}
                onChange={(e) => setColor(e.target.value)}
                placeholder="#3b82f6"
                className="flex-1 px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                disabled={loading}
              />
              {color && (
                <button
                  onClick={() => setColor("")}
                  className="text-xs text-stone-400 hover:text-stone-600"
                >
                  Clear
                </button>
              )}
            </div>
          </div>
        </div>
        {color && (
          <span
            className="inline-block px-3 py-1 text-sm rounded-full font-medium"
            style={{ backgroundColor: color + "20", color: color }}
          >
            {displayName || tag.value}
          </span>
        )}
        <div className="flex items-center gap-2">
          <button
            onClick={handleSave}
            disabled={loading}
            className="px-3 py-1.5 bg-amber-600 text-white rounded text-xs font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
          >
            {loading ? "Saving..." : "Save"}
          </button>
          <button
            onClick={() => {
              setEditing(false);
              setDisplayName(tag.display_name || "");
              setColor(tag.color || "");
              setDescription(tag.description || "");
              setEmoji(tag.emoji || "");
            }}
            className="text-xs text-stone-400 hover:text-stone-600"
          >
            Cancel
          </button>
          <div className="flex-1" />
          <button
            onClick={handleDelete}
            className="px-2 py-1 text-xs text-red-400 hover:text-red-700 hover:bg-red-50 rounded transition-colors"
          >
            Delete
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      className="flex items-center justify-between py-2 px-2 rounded hover:bg-stone-50 cursor-pointer"
      onClick={() => setEditing(true)}
    >
      <div className="flex items-center gap-3 text-sm">
        <code className="text-stone-700 bg-stone-100 px-1.5 py-0.5 rounded text-xs">
          {tag.value}
        </code>
        {tag.emoji && <span>{tag.emoji}</span>}
        {tag.color ? (
          <span
            className="px-2 py-0.5 text-xs rounded-full font-medium"
            style={{ backgroundColor: tag.color + "20", color: tag.color }}
          >
            {tag.display_name || tag.value}
          </span>
        ) : (
          <span className="text-stone-500">
            {tag.display_name || <span className="italic text-stone-300">no display name</span>}
          </span>
        )}
        {tag.description && (
          <span className="text-xs text-stone-400">{tag.description}</span>
        )}
      </div>
      <span className="text-xs text-stone-300">click to edit</span>
    </div>
  );
}
