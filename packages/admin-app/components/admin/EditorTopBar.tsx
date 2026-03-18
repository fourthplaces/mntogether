"use client";

import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
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
// Status badge variant mapping — uses shadcn Badge semantic variants
// ---------------------------------------------------------------------------

function statusBadgeVariant(status: string): "success" | "warning" | "danger" | "info" | "secondary" {
  switch (status) {
    case "active": return "success";
    case "pending_approval": return "warning"; // legacy
    case "rejected": return "danger";
    case "draft": return "info";
    case "archived": return "secondary";
    default: return "secondary";
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
  /** URL to open full preview in web-app (new tab) */
  previewUrl?: string;
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
  previewUrl,
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
          <Badge variant={statusBadgeVariant(status)} className="shrink-0">
            {statusLabel(status)}
          </Badge>
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
            <Button
              key={key}
              variant="ghost"
              size="icon-xs"
              onClick={() => onModeChange(key)}
              title={label}
              className={
                mode === key
                  ? "bg-surface-raised text-text-primary shadow-sm"
                  : "text-text-muted hover:text-text-primary"
              }
            >
              {icon}
            </Button>
          ))}
        </div>

        {/* Open in web-app preview */}
        {previewUrl && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => window.open(previewUrl, "_blank")}
            title="Open full preview in web app"
          >
            <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
            </svg>
            Preview
          </Button>
        )}

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
