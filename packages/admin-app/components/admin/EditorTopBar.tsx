"use client";

import Link from "next/link";
import { Button } from "@/components/ui/Button";
import type { SplitMode } from "@/components/admin/SplitPane";

// ---------------------------------------------------------------------------
// Icons — inline SVGs matching AdminSidebar pattern (24×24, stroke, rounded)
// ---------------------------------------------------------------------------

const icons = {
  arrowLeft: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
    </svg>
  ),
  editorOnly: (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 4h16v16H4z" />
    </svg>
  ),
  splitView: (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 4h16v16H4zM12 4v16" />
    </svg>
  ),
  previewOnly: (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 4h16v16H4z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7z" />
      <circle cx="12" cy="12" r="3" strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} />
    </svg>
  ),
};

// ---------------------------------------------------------------------------
// Status badge colors — matches post detail page
// ---------------------------------------------------------------------------

function getStatusBadgeClass(status: string): string {
  switch (status) {
    case "active":
      return "bg-green-100 text-green-800";
    case "pending_approval":
      return "bg-amber-100 text-amber-800";
    case "rejected":
      return "bg-red-100 text-red-800";
    case "draft":
      return "bg-blue-100 text-blue-800";
    case "archived":
      return "bg-stone-100 text-stone-600";
    default:
      return "bg-stone-100 text-stone-800";
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case "active": return "Published";
    case "pending_approval": return "In Review";
    case "rejected": return "Rejected";
    case "draft": return "Draft";
    case "archived": return "Archived";
    default: return status;
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface EditorTopBarProps {
  title: string;
  status?: string;
  backHref: string;
  backLabel?: string;
  onSave: () => void;
  onPublish?: () => void;
  saving?: boolean;
  dirty?: boolean;
  mode: SplitMode;
  onModeChange: (mode: SplitMode) => void;
}

export function EditorTopBar({
  title,
  status,
  backHref,
  backLabel = "Posts",
  onSave,
  onPublish,
  saving = false,
  dirty = false,
  mode,
  onModeChange,
}: EditorTopBarProps) {
  const modeButtons: { key: SplitMode; icon: React.ReactNode; label: string }[] = [
    { key: "editor", icon: icons.editorOnly, label: "Editor only" },
    { key: "split", icon: icons.splitView, label: "Split view" },
    { key: "preview", icon: icons.previewOnly, label: "Preview only" },
  ];

  return (
    <div className="flex items-center h-14 px-4 border-b border-border bg-surface-raised shrink-0 gap-3">
      {/* Left — Back link */}
      <Link
        href={backHref}
        className="flex items-center gap-1.5 text-sm text-text-muted hover:text-text-primary transition-colors shrink-0"
      >
        {icons.arrowLeft}
        <span className="hidden sm:inline">{backLabel}</span>
      </Link>

      {/* Center — Title + Status */}
      <div className="flex-1 flex items-center justify-center gap-2 min-w-0">
        <span className="text-sm font-medium text-text-primary truncate max-w-md">
          {title || "Untitled"}
        </span>
        {status && (
          <span className={`px-2 py-0.5 text-xs rounded-full font-medium shrink-0 ${getStatusBadgeClass(status)}`}>
            {statusLabel(status)}
          </span>
        )}
        {dirty && (
          <span className="w-2 h-2 rounded-full bg-admin-accent shrink-0" title="Unsaved changes" />
        )}
      </div>

      {/* Right — Mode toggles + Actions */}
      <div className="flex items-center gap-2 shrink-0">
        {/* View mode toggles */}
        <div className="hidden md:flex items-center bg-surface-muted rounded-md p-0.5">
          {modeButtons.map(({ key, icon, label }) => (
            <button
              key={key}
              onClick={() => onModeChange(key)}
              title={label}
              className={`p-1.5 rounded transition-colors ${
                mode === key
                  ? "bg-surface-raised text-text-primary shadow-sm"
                  : "text-text-muted hover:text-text-primary"
              }`}
            >
              {icon}
            </button>
          ))}
        </div>

        {/* Save button */}
        <Button
          variant="ghost"
          size="sm"
          onClick={onSave}
          loading={saving}
          disabled={!dirty || saving}
        >
          Save
        </Button>

        {/* Publish button */}
        {onPublish && (
          <Button
            variant="admin"
            size="sm"
            onClick={onPublish}
            disabled={saving}
          >
            Publish
          </Button>
        )}
      </div>
    </div>
  );
}
