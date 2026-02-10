"use client";

import Link from "next/link";
import { useState, useRef, useEffect } from "react";
import { useParams, useRouter } from "next/navigation";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type {
  OrganizationResult,
  SourceListResult,
  NoteListResult,
  NoteResult,
} from "@/lib/restate/types";

const PLATFORMS = ["instagram", "facebook", "tiktok"];

const SOURCE_TYPE_LABELS: Record<string, string> = {
  website: "Website",
  instagram: "Instagram",
  facebook: "Facebook",
  tiktok: "TikTok",
};

export default function OrganizationDetailPage() {
  const params = useParams();
  const router = useRouter();
  const orgId = params.id as string;

  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [editLoading, setEditLoading] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [regenerating, setRegenerating] = useState(false);
  const [generatingNotes, setGeneratingNotes] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const {
    data: org,
    isLoading: orgLoading,
    error: orgError,
    mutate: refetchOrg,
  } = useRestate<OrganizationResult>("Organizations", "get", { id: orgId }, {
    revalidateOnFocus: false,
  });

  const { data: sourcesData, mutate: refetchSources } =
    useRestate<SourceListResult>("Sources", "list_by_organization", {
      organization_id: orgId,
    }, { revalidateOnFocus: false });

  const { data: notesData, mutate: refetchNotes } =
    useRestate<NoteListResult>("Notes", "list_for_entity", {
      noteable_type: "organization",
      noteable_id: orgId,
    }, { revalidateOnFocus: false });

  const sources = sourcesData?.sources || [];
  const notes = notesData?.notes || [];

  const startEditing = () => {
    if (!org) return;
    setEditName(org.name);
    setEditDescription(org.description || "");
    setEditing(true);
    setEditError(null);
  };

  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editName.trim()) return;

    setEditLoading(true);
    setEditError(null);
    try {
      await callService("Organizations", "update", {
        id: orgId,
        name: editName.trim(),
        description: editDescription.trim() || null,
      });
      invalidateService("Organizations");
      refetchOrg();
      setEditing(false);
    } catch (err: any) {
      setEditError(err.message || "Failed to update organization");
    } finally {
      setEditLoading(false);
    }
  };

  const handleDelete = async () => {
    if (!confirm("Are you sure you want to delete this organization?")) return;
    try {
      await callService("Organizations", "delete", { id: orgId });
      invalidateService("Organizations");
      router.push("/admin/organizations");
    } catch (err: any) {
      alert(err.message || "Failed to delete organization");
    }
  };

  const handleRegenerate = async () => {
    setMenuOpen(false);
    setRegenerating(true);
    try {
      const result = await callService<{ organization_id: string | null; status: string }>(
        "Organizations", "regenerate", { id: orgId }
      );
      invalidateService("Organizations");
      invalidateService("Sources");
      if (result.organization_id && result.organization_id !== orgId) {
        router.push(`/admin/organizations/${result.organization_id}`);
      } else {
        refetchOrg();
        refetchSources();
      }
    } catch (err: any) {
      console.error("Failed to regenerate:", err);
      alert(err.message || "Failed to regenerate organization");
    } finally {
      setRegenerating(false);
    }
  };

  const handleGenerateNotes = async () => {
    setMenuOpen(false);
    setGeneratingNotes(true);
    try {
      const result = await callService<{ notes_created: number; sources_scanned: number }>(
        "Notes", "generate_notes", { organization_id: orgId }
      );
      invalidateService("Notes");
      refetchNotes();
      alert(`Generated ${result.notes_created} notes from ${result.sources_scanned} sources.`);
    } catch (err: any) {
      console.error("Failed to generate notes:", err);
      alert(err.message || "Failed to generate notes");
    } finally {
      setGeneratingNotes(false);
    }
  };

  if (orgLoading) {
    return <AdminLoader label="Loading organization..." />;
  }

  if (orgError) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-5xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-red-600 mb-4">Error</h1>
          <p className="text-stone-600 mb-4">{orgError.message}</p>
          <Link href="/admin/organizations" className="text-amber-600 hover:text-amber-800">
            Back to Organizations
          </Link>
        </div>
      </div>
    );
  }

  if (!org) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-5xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-stone-900 mb-4">Not Found</h1>
          <Link href="/admin/organizations" className="text-amber-600 hover:text-amber-800">
            Back to Organizations
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-5xl mx-auto">
        <Link
          href="/admin/organizations"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          {"\u2190"} Back to Organizations
        </Link>

        {/* Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          {editing ? (
            <form onSubmit={handleUpdate} className="space-y-3">
              <input
                type="text"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                className="w-full px-3 py-2 border border-stone-300 rounded-lg text-lg font-bold focus:outline-none focus:ring-2 focus:ring-amber-500"
                autoFocus
                disabled={editLoading}
              />
              <textarea
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
                placeholder="Description (optional)"
                rows={3}
                className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                disabled={editLoading}
              />
              <div className="flex items-center gap-2">
                <button
                  type="submit"
                  disabled={editLoading || !editName.trim()}
                  className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
                >
                  {editLoading ? "Saving..." : "Save"}
                </button>
                <button
                  type="button"
                  onClick={() => setEditing(false)}
                  className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
                >
                  Cancel
                </button>
                {editError && <span className="text-red-600 text-sm">{editError}</span>}
              </div>
            </form>
          ) : (
            <div className="flex justify-between items-start">
              <div>
                <h1 className="text-2xl font-bold text-stone-900 mb-1">{org.name}</h1>
                {org.description && (
                  <p className="text-stone-600">{org.description}</p>
                )}
              </div>
              <div className="flex gap-2">
                <button
                  onClick={startEditing}
                  className="px-3 py-1.5 rounded-lg text-sm font-medium bg-stone-100 text-stone-700 hover:bg-stone-200 transition-colors"
                >
                  Edit
                </button>
                <div className="relative" ref={menuRef}>
                  <button
                    onClick={() => setMenuOpen(!menuOpen)}
                    disabled={regenerating || generatingNotes}
                    className="px-3 py-1.5 bg-stone-100 text-stone-700 rounded-lg hover:bg-stone-200 disabled:opacity-50 text-sm"
                  >
                    {regenerating || generatingNotes ? "..." : "\u22EF"}
                  </button>
                  {menuOpen && (
                    <div className="absolute right-0 mt-2 w-56 bg-white rounded-lg shadow-lg border border-stone-200 py-1 z-10">
                      <button
                        onClick={handleRegenerate}
                        disabled={regenerating}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        Regenerate with AI
                      </button>
                      <button
                        onClick={handleGenerateNotes}
                        disabled={generatingNotes}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {generatingNotes ? "Generating Notes..." : "Generate Notes"}
                      </button>
                      <div className="border-t border-stone-100 my-1" />
                      <button
                        onClick={() => { setMenuOpen(false); handleDelete(); }}
                        className="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50"
                      >
                        Delete Organization
                      </button>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {regenerating && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Regenerating organization from website content...
              </span>
            </div>
          )}

          {generatingNotes && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Generating notes from crawled content...
              </span>
            </div>
          )}

          <div className="grid grid-cols-3 gap-4 pt-4 mt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Websites</span>
              <p className="text-lg font-semibold text-stone-900">{org.website_count}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Social</span>
              <p className="text-lg font-semibold text-stone-900">{org.social_profile_count}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">
                {new Date(org.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>
        </div>

        {/* Sources */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">Sources</h2>
          {sources.length === 0 ? (
            <p className="text-stone-500 text-sm">
              No sources linked. Assign this organization from a source's detail page, or add a social profile below.
            </p>
          ) : (
            <div className="space-y-2">
              {sources.map((source) => (
                <Link
                  key={source.id}
                  href={`/admin/sources/${source.id}`}
                  className="flex items-center justify-between p-3 rounded-lg border border-stone-200 hover:bg-stone-50"
                >
                  <div className="flex items-center gap-3">
                    <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                      source.source_type === "website" ? "bg-blue-100 text-blue-800" :
                      source.source_type === "instagram" ? "bg-purple-100 text-purple-800" :
                      source.source_type === "facebook" ? "bg-indigo-100 text-indigo-800" :
                      "bg-stone-100 text-stone-800"
                    }`}>
                      {SOURCE_TYPE_LABELS[source.source_type] || source.source_type}
                    </span>
                    <span className="font-medium text-stone-900">{source.identifier}</span>
                    <span
                      className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                        source.status === "approved"
                          ? "bg-green-100 text-green-800"
                          : source.status === "pending_review"
                            ? "bg-yellow-100 text-yellow-800"
                            : "bg-stone-100 text-stone-600"
                      }`}
                    >
                      {source.status.replace(/_/g, " ")}
                    </span>
                  </div>
                  <span className="text-sm text-stone-500">
                    {source.post_count || 0} posts
                  </span>
                </Link>
              ))}
            </div>
          )}

          <div className="mt-4 pt-4 border-t border-stone-200">
            <h3 className="text-sm font-medium text-stone-700 mb-2">Add Social Profile</h3>
            <AddSocialProfileForm orgId={orgId} onAdded={() => {
              refetchSources();
              refetchOrg();
            }} />
          </div>
        </div>

        {/* Notes */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">Notes</h2>

          <AddNoteForm
            noteableType="organization"
            noteableId={orgId}
            onAdded={() => refetchNotes()}
          />

          {notes.length === 0 ? (
            <p className="text-stone-500 text-sm mt-4">No notes yet.</p>
          ) : (
            <div className="space-y-2 mt-4">
              {notes.map((note) => (
                <NoteRow
                  key={note.id}
                  note={note}
                  noteableType="organization"
                  noteableId={orgId}
                  onChanged={() => refetchNotes()}
                />
              ))}
            </div>
          )}
        </div>

      </div>
    </div>
  );
}

function AddSocialProfileForm({
  orgId,
  onAdded,
}: {
  orgId: string;
  onAdded: () => void;
}) {
  const [platform, setPlatform] = useState("instagram");
  const [handle, setHandle] = useState("");
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!handle.trim()) return;

    setLoading(true);
    setError(null);
    try {
      await callService("Sources", "create_social", {
        organization_id: orgId,
        source_type: platform,
        handle: handle.trim(),
        url: url.trim() || null,
      });
      invalidateService("Sources");
      setHandle("");
      setUrl("");
      onAdded();
    } catch (err: any) {
      setError(err.message || "Failed to add profile");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="flex items-center gap-2 bg-stone-50 rounded-lg px-3 py-2"
    >
      <select
        value={platform}
        onChange={(e) => setPlatform(e.target.value)}
        className="px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 bg-white"
        disabled={loading}
      >
        {PLATFORMS.map((p) => (
          <option key={p} value={p}>
            {p.charAt(0).toUpperCase() + p.slice(1)}
          </option>
        ))}
      </select>
      <input
        type="text"
        value={handle}
        onChange={(e) => setHandle(e.target.value)}
        placeholder="Handle (e.g. @example)"
        className="flex-1 px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      <input
        type="text"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="URL (optional)"
        className="flex-1 px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      <button
        type="submit"
        disabled={loading || !handle.trim()}
        className="px-3 py-1.5 bg-amber-600 text-white rounded text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
      >
        {loading ? "..." : "Add"}
      </button>
      {error && <span className="text-red-600 text-xs">{error}</span>}
    </form>
  );
}

function AddNoteForm({
  noteableType,
  noteableId,
  onAdded,
}: {
  noteableType: string;
  noteableId: string;
  onAdded: () => void;
}) {
  const [content, setContent] = useState("");
  const [severity, setSeverity] = useState("info");
  const [isPublic, setIsPublic] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim()) return;

    setLoading(true);
    setError(null);
    try {
      await callService("Notes", "create", {
        content: content.trim(),
        severity,
        is_public: isPublic,
        noteable_type: noteableType,
        noteable_id: noteableId,
      });
      invalidateService("Notes");
      setContent("");
      setSeverity("info");
      setIsPublic(false);
      onAdded();
    } catch (err: any) {
      setError(err.message || "Failed to add note");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-2 bg-stone-50 rounded-lg px-3 py-2">
      <div className="flex items-center gap-2">
        <select
          value={severity}
          onChange={(e) => setSeverity(e.target.value)}
          className="px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 bg-white"
          disabled={loading}
        >
          <option value="info">Info</option>
          <option value="notice">Notice</option>
          <option value="warn">Warn</option>
        </select>
        <input
          type="text"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="Add a note..."
          className="flex-1 px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
          disabled={loading}
        />
        <label className="flex items-center gap-1 text-xs text-stone-500 cursor-pointer">
          <input
            type="checkbox"
            checked={isPublic}
            onChange={(e) => setIsPublic(e.target.checked)}
            className="rounded border-stone-300"
            disabled={loading}
          />
          Public
        </label>
        <button
          type="submit"
          disabled={loading || !content.trim()}
          className="px-3 py-1.5 bg-amber-600 text-white rounded text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
        >
          {loading ? "..." : "Add"}
        </button>
      </div>
      {error && <span className="text-red-600 text-xs">{error}</span>}
    </form>
  );
}

const SEVERITY_STYLES: Record<string, string> = {
  warn: "bg-red-100 text-red-800",
  notice: "bg-yellow-100 text-yellow-800",
  info: "bg-blue-100 text-blue-800",
};

function NoteRow({
  note,
  noteableType,
  noteableId,
  onChanged,
}: {
  note: NoteResult;
  noteableType: string;
  noteableId: string;
  onChanged: () => void;
}) {
  const isExpired = !!note.expired_at;

  const handleDelete = async () => {
    try {
      await callService("Notes", "delete", { id: note.id });
      invalidateService("Notes");
      onChanged();
    } catch (err: any) {
      console.error("Failed to delete note:", err);
    }
  };

  const handleUnlink = async () => {
    try {
      await callService("Notes", "unlink", {
        note_id: note.id,
        noteable_type: noteableType,
        noteable_id: noteableId,
      });
      invalidateService("Notes");
      onChanged();
    } catch (err: any) {
      console.error("Failed to unlink note:", err);
    }
  };

  return (
    <div
      className={`flex items-start justify-between p-3 rounded-lg border ${
        isExpired ? "border-stone-200 bg-stone-50 opacity-60" : "border-stone-200"
      }`}
    >
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span
            className={`px-2 py-0.5 text-xs rounded-full font-medium ${
              SEVERITY_STYLES[note.severity] || SEVERITY_STYLES.info
            }`}
          >
            {note.severity}
          </span>
          {note.is_public && (
            <span className="px-2 py-0.5 text-xs rounded-full font-medium bg-green-100 text-green-800">
              public
            </span>
          )}
          {isExpired && (
            <span className="px-2 py-0.5 text-xs rounded-full font-medium bg-stone-200 text-stone-600">
              expired
            </span>
          )}
          {note.source_type && (
            <span className="text-xs text-stone-400">
              via {note.source_type}
            </span>
          )}
          <span className="text-xs text-stone-400">
            {note.created_by} &middot; {new Date(note.created_at).toLocaleDateString()}
          </span>
        </div>
        <p className="text-sm text-stone-700">{note.content}</p>
        {note.source_url && (
          <a
            href={note.source_url}
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs text-blue-600 hover:text-blue-800 mt-1 inline-block"
          >
            Source {"\u2197"}
          </a>
        )}
      </div>
      <div className="flex gap-1 ml-2 shrink-0">
        <button
          onClick={handleUnlink}
          className="px-2 py-1 text-xs text-stone-500 hover:text-amber-700 hover:bg-amber-50 rounded transition-colors"
          title="Unlink from this entity"
        >
          Unlink
        </button>
        <button
          onClick={handleDelete}
          className="px-2 py-1 text-xs text-stone-500 hover:text-red-700 hover:bg-red-50 rounded transition-colors"
          title="Delete note entirely"
        >
          Delete
        </button>
      </div>
    </div>
  );
}

