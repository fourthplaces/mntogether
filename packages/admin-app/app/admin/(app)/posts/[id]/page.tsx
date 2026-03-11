"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { TagsSection } from "@/components/admin/TagsSection";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { ArrowLeft, CalendarIcon, ChevronDownIcon, Clock, ExternalLink, Plus, X } from "lucide-react";
import { format } from "date-fns";
import { Calendar } from "@/components/ui/calendar";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { useQuery, useMutation } from "urql";
import { useState, useCallback, useMemo, useRef } from "react";
import {
  PostDetailFullQuery,
  UpdatePostMutation,
  ApprovePostMutation,
  RejectPostMutation,
  ArchivePostMutation,
  DeletePostMutation,
  ReactivatePostMutation,
  AddPostTagMutation,
  RemovePostTagMutation,
  AddPostContactMutation,
  RemovePostContactMutation,
  AddPostScheduleMutation,
  DeletePostScheduleMutation,
} from "@/lib/graphql/posts";
import { OrganizationsListQuery } from "@/lib/graphql/organizations";
import { TagKindsQuery, TagsQuery } from "@/lib/graphql/tags";
import { markdownComponents } from "@/lib/markdown-components";
import { POST_TYPES, WEIGHTS, URGENCIES, CATEGORIES } from "@/lib/post-form-constants";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

function formatTime12h(time24: string): string {
  const [h, m] = time24.split(":").map(Number);
  const suffix = h >= 12 ? "PM" : "AM";
  const h12 = h % 12 || 12;
  return `${h12}:${m.toString().padStart(2, "0")} ${suffix}`;
}

interface ScheduleItem {
  id: string;
  dayOfWeek?: number | null;
  opensAt?: string | null;
  closesAt?: string | null;
  timezone: string;
  notes?: string | null;
  rrule?: string | null;
  dtstart?: string | null;
  dtend?: string | null;
  isAllDay: boolean;
  durationMinutes?: number | null;
}

function isScheduleExpired(s: ScheduleItem): boolean {
  if (s.dtend && !s.rrule) return new Date(s.dtend) < new Date();
  if (s.dtstart && !s.rrule && !s.dtend) return new Date(s.dtstart) < new Date();
  return false;
}

function formatSchedule(s: ScheduleItem): string {
  if (s.dtstart && s.dayOfWeek == null) {
    const date = new Date(s.dtstart);
    const dateStr = date.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
    const timeStr = s.opensAt && s.closesAt
      ? `${formatTime12h(s.opensAt)} \u2013 ${formatTime12h(s.closesAt)}`
      : s.opensAt ? formatTime12h(s.opensAt) : "";
    return [dateStr, timeStr].filter(Boolean).join("  ");
  }
  const dayName = s.dayOfWeek != null ? DAY_NAMES[s.dayOfWeek] : "";
  const timeStr = s.opensAt && s.closesAt
    ? `${formatTime12h(s.opensAt)} \u2013 ${formatTime12h(s.closesAt)}`
    : s.opensAt ? formatTime12h(s.opensAt) : "";
  let suffix = "";
  if (s.rrule?.includes("INTERVAL=2")) suffix = " (every other week)";
  if (s.rrule?.includes("FREQ=MONTHLY")) suffix = " (monthly)";
  return [dayName, timeStr, suffix].filter(Boolean).join("  ");
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString();
}

function statusBadgeVariant(status: string): "success" | "warning" | "danger" | "info" | "secondary" {
  switch (status) {
    case "active": return "success";
    case "pending_approval": return "warning"; // legacy
    case "draft": return "info";
    case "rejected": return "danger";
    case "draft": return "info";
    case "archived": return "secondary";
    default: return "secondary";
  }
}

// ---------------------------------------------------------------------------
// Section label component
// ---------------------------------------------------------------------------

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-3">
      {children}
    </h3>
  );
}


// ---------------------------------------------------------------------------
// Body text preview (read-only, from Root Signal)
// ---------------------------------------------------------------------------

function BodyPreview({ label, text }: { label: string; text?: string | null }) {
  return (
    <div className="border-t border-border pt-4">
      <SectionLabel>{label}</SectionLabel>
      {text ? (
        <p className="text-sm text-text-body leading-relaxed whitespace-pre-wrap">{text}</p>
      ) : (
        <p className="text-sm text-text-faint italic">Not yet generated</p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Inline field — text input that saves on blur
// ---------------------------------------------------------------------------

function InlineTextField({
  label,
  value,
  placeholder,
  onSave,
  className,
  missing,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onSave: (v: string) => void;
  className?: string;
  missing?: boolean;
}) {
  const [localValue, setLocalValue] = useState(value);
  const prevValue = useRef(value);

  // Sync if external value changes (e.g. after mutation response)
  if (value !== prevValue.current) {
    prevValue.current = value;
    setLocalValue(value);
  }

  const handleBlur = () => {
    const trimmed = localValue.trim();
    if (trimmed !== value) {
      onSave(trimmed);
    }
  };

  return (
    <div className={className}>
      <label className="block text-xs text-muted-foreground uppercase mb-1">{label}</label>
      <Input
        value={localValue}
        onChange={(e) => setLocalValue(e.target.value)}
        onBlur={handleBlur}
        placeholder={placeholder}
        className={`h-9 text-sm ${missing ? "border-2 border-amber-400" : ""}`}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Contacts inline CRUD
// ---------------------------------------------------------------------------

const CONTACT_TYPES = [
  { value: "phone", label: "Phone" },
  { value: "email", label: "Email" },
  { value: "website", label: "Website" },
  { value: "address", label: "Address" },
  { value: "booking_url", label: "Booking URL" },
  { value: "social", label: "Social" },
];

function ContactsSection({
  contacts,
  postId,
  addPostContact,
  removePostContact,
  mutationContext,
}: {
  contacts: Array<{ id: string; contactType: string; contactValue: string; contactLabel?: string | null }>;
  postId: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  addPostContact: (vars: any, ctx?: any) => Promise<any>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  removePostContact: (vars: any, ctx?: any) => Promise<any>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mutationContext: any;
}) {
  const [newType, setNewType] = useState("phone");
  const [newValue, setNewValue] = useState("");
  const [busy, setBusy] = useState(false);

  const handleAdd = async () => {
    const trimmed = newValue.trim();
    if (!trimmed) return;
    setBusy(true);
    try {
      await addPostContact({ postId, contactType: newType, contactValue: trimmed }, mutationContext);
      setNewValue("");
    } catch (err) {
      console.error("Failed to add contact:", err);
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async (contactId: string) => {
    setBusy(true);
    try {
      await removePostContact({ postId, contactId }, mutationContext);
    } catch (err) {
      console.error("Failed to remove contact:", err);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="border-t border-border pt-4">
      <SectionLabel>Contacts</SectionLabel>

      {/* Existing contacts */}
      {contacts.length > 0 ? (
        <div className="space-y-1.5 mb-3">
          {contacts.map((c) => (
            <div key={c.id} className="flex items-center gap-2 group">
              <Badge variant="secondary" className="text-[10px] uppercase flex-shrink-0 w-16 justify-center">
                {c.contactType.replace("_", " ")}
              </Badge>
              <span className="text-sm text-text-body break-all flex-1 min-w-0 truncate">
                {c.contactType === "email" ? (
                  <a href={`mailto:${c.contactValue}`} className="text-link hover:text-link-hover">{c.contactValue}</a>
                ) : c.contactType === "phone" ? (
                  <a href={`tel:${c.contactValue}`} className="text-link hover:text-link-hover">{c.contactValue}</a>
                ) : c.contactType === "website" || c.contactType === "booking_url" || c.contactType === "social" ? (
                  <a href={c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`} target="_blank" rel="noopener noreferrer" className="text-link hover:text-link-hover">{c.contactValue}</a>
                ) : (
                  c.contactValue
                )}
              </span>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => handleRemove(c.id)}
                disabled={busy}
                className="h-6 w-6 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-danger-text transition-opacity"
                title="Remove contact"
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-text-faint italic mb-3">No contacts</p>
      )}

      {/* Add contact form */}
      <div className="flex items-center gap-1.5">
        <Select value={newType} onValueChange={(val) => val !== null && setNewType(val)}>
          <SelectTrigger className="text-xs w-24 flex-shrink-0">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {CONTACT_TYPES.map((t) => (
              <SelectItem key={t.value} value={t.value}>{t.label}</SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Input
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="Value..."
          className="text-xs flex-1 min-w-0"
          disabled={busy}
        />
        <Button
          variant="outline"
          size="icon"
          onClick={handleAdd}
          disabled={busy || !newValue.trim()}
          className="w-8 flex-shrink-0"
        >
          <Plus className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Styled time input — hides native browser chrome per shadcn Time Picker
// ---------------------------------------------------------------------------

const timeInputStyles =
  "appearance-none bg-card [&::-webkit-calendar-picker-indicator]:hidden [&::-webkit-calendar-picker-indicator]:appearance-none";

// ---------------------------------------------------------------------------
// Schedules inline CRUD
// ---------------------------------------------------------------------------

const DAY_OPTIONS = DAY_NAMES.map((name, i) => ({ value: i, label: name }));

type ScheduleMode = "hours" | "event";

function SchedulesSection({
  schedules,
  postId,
  addPostSchedule,
  deletePostSchedule,
  mutationContext,
}: {
  schedules: ScheduleItem[];
  postId: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  addPostSchedule: (vars: any, ctx?: any) => Promise<any>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  deletePostSchedule: (vars: any, ctx?: any) => Promise<any>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mutationContext: any;
}) {
  const [mode, setMode] = useState<ScheduleMode>("hours");
  const [busy, setBusy] = useState(false);

  // Operating hours state
  const [newDay, setNewDay] = useState<string>("1"); // Monday
  const [newOpens, setNewOpens] = useState("09:00");
  const [newCloses, setNewCloses] = useState("17:00");

  // Event state
  const [eventDate, setEventDate] = useState<Date | undefined>(undefined);
  const [eventTime, setEventTime] = useState("12:00");
  const [calendarOpen, setCalendarOpen] = useState(false);

  const handleAddHours = async () => {
    setBusy(true);
    try {
      await addPostSchedule(
        {
          postId,
          input: {
            dayOfWeek: parseInt(newDay, 10),
            opensAt: newOpens,
            closesAt: newCloses,
            timezone: "America/Chicago",
          },
        },
        mutationContext
      );
    } catch (err) {
      console.error("Failed to add schedule:", err);
    } finally {
      setBusy(false);
    }
  };

  const handleAddEvent = async () => {
    if (!eventDate) return;
    setBusy(true);
    try {
      const [h, m] = eventTime.split(":").map(Number);
      const dt = new Date(eventDate);
      dt.setHours(h, m, 0, 0);
      await addPostSchedule(
        {
          postId,
          input: {
            dtstart: dt.toISOString(),
            opensAt: eventTime,
            timezone: "America/Chicago",
          },
        },
        mutationContext
      );
      setEventDate(undefined);
      setEventTime("12:00");
    } catch (err) {
      console.error("Failed to add event:", err);
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async (scheduleId: string) => {
    setBusy(true);
    try {
      await deletePostSchedule({ postId, scheduleId }, mutationContext);
    } catch (err) {
      console.error("Failed to delete schedule:", err);
    } finally {
      setBusy(false);
    }
  };

  const oneOffSchedules = schedules.filter((s) => !s.rrule);
  const allOneOffsExpired = oneOffSchedules.length > 0 && oneOffSchedules.every(isScheduleExpired);

  return (
    <div className="border-t border-border pt-4">
      <SectionLabel>Schedule</SectionLabel>

      {allOneOffsExpired && (
        <p className="text-xs text-amber-600 font-medium mb-2">This event has passed</p>
      )}

      {/* Existing schedules */}
      {schedules.length > 0 ? (
        <div className="space-y-1.5 mb-4">
          {schedules.map((s) => (
            <div key={s.id} className={`flex items-center gap-2 group ${isScheduleExpired(s) ? "opacity-60" : ""}`}>
              <Clock className="w-4 h-4 flex-shrink-0 text-text-faint" />
              <span className="text-sm text-text-body flex-1 min-w-0">{formatSchedule(s)}</span>
              {s.notes && <span className="text-xs text-text-faint italic truncate max-w-[100px]">{s.notes}</span>}
              <Button
                variant="ghost"
                size="icon"
                onClick={() => handleRemove(s.id)}
                disabled={busy}
                className="h-6 w-6 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-danger-text transition-opacity"
                title="Remove schedule"
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-text-faint italic mb-4">No schedules</p>
      )}

      {/* Add form */}
      <Tabs value={mode} onValueChange={(v) => setMode(v as ScheduleMode)}>
        <TabsList className="mb-3">
          <TabsTrigger value="hours">Operating Hours</TabsTrigger>
          <TabsTrigger value="event">One-off Event</TabsTrigger>
        </TabsList>

        <TabsContent value="hours">
          <FieldGroup className="flex-row items-end gap-2">
            <Field className="w-auto">
              <FieldLabel className="text-xs">Day</FieldLabel>
              <Select value={newDay} onValueChange={(val) => val !== null && setNewDay(val)}>
                <SelectTrigger className="text-sm w-28">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {DAY_OPTIONS.map((d) => (
                    <SelectItem key={d.value} value={String(d.value)}>{d.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field className="w-auto">
              <FieldLabel className="text-xs">Opens</FieldLabel>
              <Input
                type="time"
                value={newOpens}
                onChange={(e) => setNewOpens(e.target.value)}
                className={`text-sm w-[110px] ${timeInputStyles}`}
                disabled={busy}
              />
            </Field>
            <Field className="w-auto">
              <FieldLabel className="text-xs">Closes</FieldLabel>
              <Input
                type="time"
                value={newCloses}
                onChange={(e) => setNewCloses(e.target.value)}
                className={`text-sm w-[110px] ${timeInputStyles}`}
                disabled={busy}
              />
            </Field>
            <Button
              variant="outline"
              size="sm"
              onClick={handleAddHours}
              disabled={busy}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add
            </Button>
          </FieldGroup>
        </TabsContent>

        <TabsContent value="event">
          <FieldGroup className="flex-row items-end gap-2">
            <Field className="w-auto">
              <FieldLabel className="text-xs">Date</FieldLabel>
              <Popover open={calendarOpen} onOpenChange={setCalendarOpen}>
                <PopoverTrigger render={<Button variant="outline" className="w-40 justify-between text-sm font-normal" />}>
                    {eventDate ? format(eventDate, "MMM d, yyyy") : "Select date"}
                    <ChevronDownIcon className="h-4 w-4 opacity-50" />
                </PopoverTrigger>
                <PopoverContent className="w-auto overflow-hidden p-0" align="start">
                  <Calendar
                    mode="single"
                    selected={eventDate}
                    captionLayout="dropdown"
                    defaultMonth={eventDate}
                    onSelect={(date) => {
                      setEventDate(date);
                      setCalendarOpen(false);
                    }}
                  />
                </PopoverContent>
              </Popover>
            </Field>
            <Field className="w-auto">
              <FieldLabel className="text-xs">Time</FieldLabel>
              <Input
                type="time"
                value={eventTime}
                onChange={(e) => setEventTime(e.target.value)}
                className={`text-sm w-[110px] ${timeInputStyles}`}
                disabled={busy}
              />
            </Field>
            <Button
              variant="outline"
              size="sm"
              onClick={handleAddEvent}
              disabled={busy || !eventDate}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add
            </Button>
          </FieldGroup>
        </TabsContent>
      </Tabs>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export default function PostDetailPage() {
  const params = useParams();
  const router = useRouter();
  const postId = params.id as string;
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);

  // GraphQL: fetch post detail + notes
  const [{ data: postData, fetching: isLoading, error }] = useQuery({
    query: PostDetailFullQuery,
    variables: { id: postId },
  });
  const post = postData?.post;
  const notes = postData?.entityNotes || [];

  // Organizations for inline dropdown
  const [{ data: orgsData }] = useQuery({ query: OrganizationsListQuery });
  const organizations = orgsData?.organizations ?? [];

  // GraphQL mutations
  const [, updatePost] = useMutation(UpdatePostMutation);
  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, rejectPost] = useMutation(RejectPostMutation);
  const [, archivePost] = useMutation(ArchivePostMutation);
  const [, deletePost] = useMutation(DeletePostMutation);
  const [, reactivatePost] = useMutation(ReactivatePostMutation);
  const [, addPostTag] = useMutation(AddPostTagMutation);
  const [, removePostTag] = useMutation(RemovePostTagMutation);
  const [, addPostContact] = useMutation(AddPostContactMutation);
  const [, removePostContact] = useMutation(RemovePostContactMutation);
  const [, addPostSchedule] = useMutation(AddPostScheduleMutation);
  const [, deletePostSchedule] = useMutation(DeletePostScheduleMutation);

  // Tag data: kinds + all tag values
  const [{ data: kindsData }] = useQuery({ query: TagKindsQuery });
  const [{ data: allTagsData }] = useQuery({ query: TagsQuery });

  const postTagKinds = useMemo(
    () => (kindsData?.tagKinds || [])
      .filter((k) => k.allowedResourceTypes.includes("post"))
      .map((k) => ({ slug: k.slug, displayName: k.displayName, locked: k.locked })),
    [kindsData]
  );

  const allTagsByKind = useMemo(() => {
    const map: Record<string, Array<{ id: string; value: string; displayName?: string | null; color?: string | null }>> = {};
    for (const tag of allTagsData?.tags || []) {
      if (!map[tag.kind]) map[tag.kind] = [];
      map[tag.kind].push(tag);
    }
    return map;
  }, [allTagsData]);

  const tags = post?.tags || [];

  const mutationContext = { additionalTypenames: ["Post", "PostConnection", "PostStats"] };

  // ---------------------------------------------------------------------------
  // Inline update helper — fires updatePost for a single field change
  // ---------------------------------------------------------------------------
  const inlineUpdate = useCallback(
    async (input: Record<string, unknown>) => {
      await updatePost({ id: postId, input }, mutationContext);
    },
    [postId, updatePost]
  );

  // ---------------------------------------------------------------------------
  // Action handlers (same as before)
  // ---------------------------------------------------------------------------

  const handleAddTags = useCallback(
    async (kindSlug: string, newTags: Array<{ value: string; displayName: string }>) => {
      setIsUpdating(true);
      try {
        await Promise.all(
          newTags.map((t) =>
            addPostTag({ postId, tagKind: kindSlug, tagValue: t.value, displayName: t.displayName }, mutationContext)
          )
        );
      } catch (err) {
        console.error("Failed to add tags:", err);
      } finally {
        setIsUpdating(false);
      }
    },
    [postId, addPostTag]
  );

  const handleRemoveTag = useCallback(
    async (tagId: string) => {
      if (!postId) return;
      setIsUpdating(true);
      try {
        await removePostTag({ postId, tagId }, mutationContext);
      } catch (err) {
        console.error("Failed to remove tag:", err);
      } finally {
        setIsUpdating(false);
      }
    },
    [postId, removePostTag]
  );

  const withAction = (name: string, fn: () => Promise<unknown>) => async () => {
    setActionInProgress(name);
    try { await fn(); } catch (err) { console.error(`Failed to ${name}:`, err); } finally { setActionInProgress(null); }
  };

  const handleArchive = withAction("archive", () => archivePost({ id: postId }, mutationContext));
  const handleReactivate = withAction("reactivate", () => reactivatePost({ id: postId }, mutationContext));
  const handleApprove = withAction("approve", () => approvePost({ id: postId }, mutationContext));
  const handleReject = withAction("reject", () => rejectPost({ id: postId, reason: "Rejected by admin" }, mutationContext));
  const handleDelete = withAction("delete", async () => {
    await deletePost({ id: postId }, mutationContext);
    router.push("/admin/posts");
  });

  // ---------------------------------------------------------------------------
  // Loading / error states
  // ---------------------------------------------------------------------------

  if (isLoading) return <AdminLoader label="Loading post..." />;

  if (error) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-danger-text mb-4">Error Loading Post</h1>
          <p className="text-muted-foreground mb-4">{error.message}</p>
          <Link href="/admin/posts" className="text-link hover:text-link-hover">Back to Posts</Link>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-foreground mb-4">Post Not Found</h1>
          <Link href="/admin/posts" className="text-link hover:text-link-hover">Back to Posts</Link>
        </div>
      </div>
    );
  }

  const urgencyValue = post.urgency || "";

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div className="min-h-screen bg-background px-4 py-4">
      <div className="max-w-7xl mx-auto">

        {/* ── Header bar ─────────────────────────────────────────────── */}
        <div className="flex items-center justify-between mb-4">
          <Link
            href="/admin/posts"
            className="inline-flex items-center text-muted-foreground hover:text-foreground text-sm"
          >
            <ArrowLeft className="w-4 h-4 mr-1" /> Back to Posts
          </Link>

          <div className="flex items-center gap-2">
            <Button render={<Link href={`/admin/posts/${postId}/edit`} />} variant="outline" size="sm">
              Edit
            </Button>

            <Select
              value={post.status}
              disabled={actionInProgress !== null}
              onValueChange={(val) => {
                if (!val || val === post.status) return;
                const newStatus = val as string;
                if (newStatus === "active") handleApprove();
                else if (newStatus === "rejected") handleReject();
                else if (newStatus === "archived") handleArchive();
                else if (newStatus === "draft" || newStatus === "pending_approval") handleReactivate();
              }}
            >
              <SelectTrigger className="h-7 w-auto min-w-0 gap-1 rounded-full px-2.5 text-xs font-medium">
                <Badge variant={statusBadgeVariant(post.status)} className="pointer-events-none">
                  <SelectValue />
                </Badge>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="draft">Draft</SelectItem>
                <SelectItem value="active">Active</SelectItem>
                <SelectItem value="rejected">Rejected</SelectItem>
                <SelectItem value="archived">Archived</SelectItem>
              </SelectContent>
            </Select>

            {post.status === "active" && (
              <Link
                href={`/posts/${postId}`}
                className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
                title="View public page"
              >
                <ExternalLink className="w-4 h-4" />
              </Link>
            )}

            {post.sourceUrl && (
              <a
                href={post.sourceUrl.startsWith("http") ? post.sourceUrl : `https://${post.sourceUrl}`}
                target="_blank"
                rel="noopener noreferrer"
                className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
                title="View source page"
              >
                {"\u{1F517}"}
              </a>
            )}

            <Button
              variant="destructive"
              size="sm"
              onClick={handleDelete}
              disabled={actionInProgress !== null}
            >
              {actionInProgress === "delete" ? "Deleting..." : "Delete"}
            </Button>
          </div>
        </div>

        {/* ── Two-column layout ──────────────────────────────────────── */}
        <div className="grid grid-cols-1 lg:grid-cols-[6fr_4fr] gap-6">

          {/* ── LEFT COLUMN (70%) ──────────────────────────────────── */}
          <div className="space-y-6">

            {/* Title */}
            <h1 className="text-2xl font-bold text-foreground">{post.title}</h1>

            {/* ── Broadsheet Display ─────────────────────────────── */}
            <div className="space-y-3 border-t border-border pt-4">
              <SectionLabel>Broadsheet Display</SectionLabel>

              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                {/* Post Type */}
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Type</label>
                  <Select
                    value={post.postType || "notice"}
                    onValueChange={(v) => inlineUpdate({ postType: v })}
                  >
                    <SelectTrigger className="text-sm w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {POST_TYPES.map((t) => (
                        <SelectItem key={t.value} value={t.value}>{t.label}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                {/* Weight */}
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Weight</label>
                  <Select
                    value={post.weight || "medium"}
                    onValueChange={(v) => inlineUpdate({ weight: v })}
                  >
                    <SelectTrigger className="text-sm w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {WEIGHTS.map((w) => (
                        <SelectItem key={w.value} value={w.value}>{w.label}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                {/* Priority */}
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Priority</label>
                  <Input
                    type="number"
                    defaultValue={post.priority ?? 0}
                    className="text-sm"
                    onBlur={(e) => {
                      const val = Number(e.target.value);
                      if (val !== (post.priority ?? 0)) {
                        inlineUpdate({ priority: val });
                      }
                    }}
                  />
                </div>

                {/* Urgency */}
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Urgency</label>
                  <Select
                    value={urgencyValue || "__none__"}
                    onValueChange={(v) => inlineUpdate({ urgency: v === "__none__" ? "" : v })}
                  >
                    <SelectTrigger className="text-sm w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {URGENCIES.map((u) => (
                        <SelectItem key={u.value || "__none__"} value={u.value || "__none__"}>
                          {u.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>

              {urgencyValue === "urgent" && (
                <p className="text-xs text-red-600">Flagged as urgent — will display an Urgent label on the broadsheet.</p>
              )}
            </div>

            {/* ── Content Details ─────────────────────────────────── */}
            <div className="space-y-3 border-t border-border pt-4">
              <SectionLabel>Content Details</SectionLabel>

              {/* Organization */}
              <div>
                <label className="block text-xs text-muted-foreground uppercase mb-1">Organization</label>
                <Select
                  value={post.organizationId || "__none__"}
                  onValueChange={(v) => inlineUpdate({ organizationId: v === "__none__" ? null : v })}
                >
                  <SelectTrigger className="text-sm w-full max-w-sm">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="__none__">None</SelectItem>
                    {organizations.map((org) => (
                      <SelectItem key={org.id} value={org.id}>{org.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* Category */}
              <div>
                <label className="block text-xs text-muted-foreground uppercase mb-1">Category</label>
                <Select
                  value={post.category || "other"}
                  onValueChange={(v) => inlineUpdate({ category: v })}
                >
                  <SelectTrigger className="text-sm w-full max-w-sm">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {CATEGORIES.map((c) => (
                      <SelectItem key={c.value} value={c.value}>{c.label}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* Source URL */}
              <InlineTextField
                label="Source URL"
                value={post.sourceUrl || ""}
                placeholder="https://..."
                onSave={(v) => inlineUpdate({ sourceUrl: v || null })}
                missing={!post.sourceUrl}
              />

              {/* Location + Zip Code */}
              <div className="grid grid-cols-2 gap-3">
                <InlineTextField
                  label="Location"
                  value={post.location || ""}
                  placeholder="e.g. Minneapolis, MN"
                  onSave={(v) => inlineUpdate({ location: v || null })}
                  missing={!post.location}
                />
                <InlineTextField
                  label="Zip Code"
                  value={post.zipCode || ""}
                  placeholder="e.g. 55401"
                  onSave={(v) => inlineUpdate({ zipCode: v || null })}
                  missing={!post.zipCode}
                />
              </div>

              {/* Lat/Lng (display-only, from geocoding) */}
              {(post.latitude != null || post.longitude != null) && (
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Coordinates</label>
                  <span className="text-sm text-text-body font-mono">
                    {post.latitude?.toFixed(4)}, {post.longitude?.toFixed(4)}
                  </span>
                </div>
              )}

              {/* Submission type */}
              {post.submissionType && (
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Submission Type</label>
                  <Badge variant="secondary" className="text-xs">{post.submissionType}</Badge>
                </div>
              )}
            </div>

            {/* Contacts */}
            <ContactsSection
              contacts={post.contacts || []}
              postId={postId}
              addPostContact={addPostContact}
              removePostContact={removePostContact}
              mutationContext={mutationContext}
            />

            {/* Schedule */}
            <SchedulesSection
              schedules={post.schedules || []}
              postId={postId}
              addPostSchedule={addPostSchedule}
              deletePostSchedule={deletePostSchedule}
              mutationContext={mutationContext}
            />

            {/* Full text */}
            <div className="border-t border-border pt-4">
              <SectionLabel>Full Text</SectionLabel>
              <div className="prose prose-stone max-w-none text-sm">
                <ReactMarkdown components={markdownComponents}>
                  {post.descriptionMarkdown || post.description || ""}
                </ReactMarkdown>
              </div>
            </div>

            {/* Body previews — heavy / medium / light */}
            <BodyPreview label="Heavy" text={post.bodyHeavy} />
            <BodyPreview label="Medium" text={post.bodyMedium} />
            <BodyPreview label="Light" text={post.bodyLight} />
          </div>

          {/* ── RIGHT COLUMN (30%) ─────────────────────────────────── */}
          <div className="space-y-6">

            {/* Tags */}
            <TagsSection
              tags={tags}
              applicableKinds={postTagKinds}
              allTagsByKind={allTagsByKind}
              onRemoveTag={handleRemoveTag}
              onAddTags={handleAddTags}
              disabled={isUpdating}
            />

            {/* Notes */}
            {notes.length > 0 && (
              <div className="border-t border-border pt-4">
                <SectionLabel>Notes ({notes.length})</SectionLabel>
                <div className="space-y-2">
                  {notes.map((note) => {
                    const isExpired = !!note.expiredAt;
                    const severityVariant: "danger" | "warning" | "info" =
                      note.severity === "urgent" ? "danger" :
                      note.severity === "notice" ? "warning" : "info";
                    return (
                      <div
                        key={note.id}
                        className={`p-3 rounded-lg border ${isExpired ? "border-border bg-secondary opacity-60" : "border-border bg-card"}`}
                      >
                        <div className="flex items-center gap-2 mb-1">
                          <Badge variant={severityVariant}>{note.severity}</Badge>
                          {note.isPublic && <Badge variant="success">public</Badge>}
                          {isExpired && <Badge variant="secondary">expired</Badge>}
                          <span className="text-xs text-text-faint">
                            {note.createdBy} &middot; {new Date(note.createdAt).toLocaleDateString()}
                          </span>
                        </div>
                        <p className="text-sm text-text-body">{note.content}</p>
                        {note.sourceUrl && (
                          <a href={note.sourceUrl} target="_blank" rel="noopener noreferrer" className="text-xs text-link hover:text-link-hover mt-1 inline-block">
                            Source <ExternalLink className="inline w-3 h-3 ml-0.5" />
                          </a>
                        )}
                        {note.linkedPosts && note.linkedPosts.filter(p => p.id !== postId).length > 0 && (
                          <div className="flex flex-wrap items-center gap-1 mt-1.5">
                            <span className="text-xs text-text-faint">Also on:</span>
                            {note.linkedPosts.filter(p => p.id !== postId).map((p) => (
                              <Link
                                key={p.id}
                                href={`/admin/posts/${p.id}`}
                                className="text-xs px-1.5 py-0.5 bg-secondary text-secondary-foreground rounded hover:bg-accent hover:text-accent-foreground transition-colors truncate max-w-[200px]"
                                title={p.title}
                              >
                                {p.title}
                              </Link>
                            ))}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* System info */}
            <div className="border-t border-border pt-4">
              <SectionLabel>System</SectionLabel>
              <div className="space-y-1.5 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Language</span>
                  <Badge variant="secondary" className="text-xs uppercase">{post.sourceLanguage}</Badge>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Submitted by</span>
                  <span className="text-foreground font-medium">
                    {post.submittedBy?.submitterType === "member" ? "Member" : <span className="text-text-faint">Unknown</span>}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Ingested</span>
                  <span className="text-foreground">{formatDate(post.createdAt)}</span>
                </div>
                {post.publishedAt && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Published</span>
                    <span className="text-foreground">{formatDate(post.publishedAt)}</span>
                  </div>
                )}
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Last edited</span>
                  <span className="text-foreground">{formatDate(post.updatedAt)}</span>
                </div>
                {post.revisionOfPostId && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Revision of</span>
                    <Link href={`/admin/posts/${post.revisionOfPostId}`} className="text-link hover:text-link-hover text-xs font-mono truncate max-w-[140px]">
                      {post.revisionOfPostId}
                    </Link>
                  </div>
                )}
                {post.translationOfId && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Translation of</span>
                    <Link href={`/admin/posts/${post.translationOfId}`} className="text-link hover:text-link-hover text-xs font-mono truncate max-w-[140px]">
                      {post.translationOfId}
                    </Link>
                  </div>
                )}
                {post.duplicateOfId && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Duplicate of</span>
                    <Link href={`/admin/posts/${post.duplicateOfId}`} className="text-link hover:text-link-hover text-xs font-mono truncate max-w-[140px]">
                      {post.duplicateOfId}
                    </Link>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

    </div>
  );
}
