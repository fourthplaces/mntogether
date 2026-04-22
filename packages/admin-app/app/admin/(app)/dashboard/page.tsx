"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { useMutation, useQuery } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  CircleDashed,
  FileText,
  MessageSquare,
  Upload,
} from "lucide-react";
import {
  CountyDashboardQuery,
  BatchGenerateEditionsMutation,
  PublishEditionMutation,
  CreateEditionMutation,
  GenerateEditionMutation,
} from "@/lib/graphql/editions";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/**
 * Returns the Monday of the current week (America/Chicago local time)
 * and the following Sunday. All editions are keyed on Mon–Sun periods.
 */
function currentWeek(): { monday: Date; sunday: Date; label: string; mondayIso: string; sundayIso: string } {
  const now = new Date();
  // getDay(): Sunday=0 ... Saturday=6. Shift so Monday=0 ... Sunday=6.
  const dayMondayFirst = (now.getDay() + 6) % 7;
  const monday = new Date(now);
  monday.setDate(now.getDate() - dayMondayFirst);
  monday.setHours(0, 0, 0, 0);
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);

  const iso = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

  const fmt = (d: Date) =>
    d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  const year = monday.getFullYear();
  const label = `${fmt(monday)} – ${fmt(sunday)}, ${year}`;

  return { monday, sunday, label, mondayIso: iso(monday), sundayIso: iso(sunday) };
}

type CountyRow = {
  county: { id: string; name: string; isPseudo: boolean };
  currentEdition: {
    id: string;
    periodStart: string;
    periodEnd: string;
    status: string;
    publishedAt: string | null;
    rowCount: number;
  } | null;
  isStale: boolean;
};

type Bucket = "published" | "approved" | "in_review" | "draft" | "missing";

/**
 * Bucket a county row by what it needs next. Editions whose currentEdition
 * belongs to a prior week count as "missing" — the county has nothing for
 * this week, regardless of its last edition's status.
 */
function bucketFor(row: CountyRow, currentMondayIso: string): Bucket {
  const e = row.currentEdition;
  if (!e || e.periodStart !== currentMondayIso) return "missing";
  if (e.status === "published") return "published";
  if (e.status === "approved") return "approved";
  if (e.status === "in_review") return "in_review";
  return "draft";
}

// ─── Component ───────────────────────────────────────────────────────────────

export default function DashboardPage() {
  const week = useMemo(() => currentWeek(), []);
  const [{ data, fetching }, refetch] = useQuery({ query: CountyDashboardQuery });
  const [batchState, batchGenerate] = useMutation(BatchGenerateEditionsMutation);
  const [, publishEdition] = useMutation(PublishEditionMutation);
  const [, createEdition] = useMutation(CreateEditionMutation);
  const [, generateEdition] = useMutation(GenerateEditionMutation);
  const [flashMessage, setFlashMessage] = useState<string | null>(null);
  const [flashError, setFlashError] = useState<string | null>(null);
  const [publishing, setPublishing] = useState(false);
  const [statewideBusy, setStatewideBusy] = useState<
    null | "generate" | "publish"
  >(null);

  const buckets = useMemo(() => {
    const allRows = (data?.countyDashboard ?? []) as CountyRow[];
    // Pseudo counties (Statewide) are first-class in the picker but
    // don't belong in "N of 87 counties published" roll-ups. Bucket
    // them separately so the dashboard keeps its coverage signal
    // clean while still surfacing the statewide edition's status.
    const realRows = allRows.filter((r) => !r.county.isPseudo);
    const pseudoRows = allRows.filter((r) => r.county.isPseudo);
    const counts: Record<Bucket, number> = {
      published: 0,
      approved: 0,
      in_review: 0,
      draft: 0,
      missing: 0,
    };
    for (const r of realRows) counts[bucketFor(r, week.mondayIso)]++;
    const statewide =
      pseudoRows.length > 0
        ? { row: pseudoRows[0], bucket: bucketFor(pseudoRows[0], week.mondayIso) }
        : null;
    return {
      counts,
      total: realRows.length,
      statewide,
    };
  }, [data, week.mondayIso]);

  /**
   * Bulk-publish every approved edition for the current week. The dashboard
   * already has the per-county edition state from countyDashboard; filter to
   * this-week + status=approved and fire publishEdition for each.
   *
   * Sequential (not Promise.all) so a single failure doesn't abort the
   * rest and so we can report fine-grained results. For 87 counties this
   * is a couple of seconds of UI progress; fine for admin UX. If we ever
   * want a one-shot batch on the server, add `/Editions/publish_batch` —
   * until then this loop owns the flow.
   */
  async function handlePublishAllApproved() {
    setFlashError(null);
    setFlashMessage(null);
    const rows = (data?.countyDashboard ?? []) as CountyRow[];
    const targets = rows
      .filter(
        (r) =>
          !r.county.isPseudo &&
          r.currentEdition?.periodStart === week.mondayIso &&
          r.currentEdition.status === "approved"
      )
      .map((r) => r.currentEdition!.id);
    if (targets.length === 0) return;
    const ok = window.confirm(
      `Publish ${targets.length} approved edition${
        targets.length === 1 ? "" : "s"
      } to the public site?`
    );
    if (!ok) return;
    setPublishing(true);
    let succeeded = 0;
    const failures: Array<{ id: string; message: string }> = [];
    for (const id of targets) {
      const r = await publishEdition({ id });
      if (r.error) failures.push({ id, message: r.error.message });
      else succeeded++;
    }
    setPublishing(false);
    refetch({ requestPolicy: "network-only" });
    if (failures.length === 0) {
      setFlashMessage(
        `Published ${succeeded} edition${succeeded === 1 ? "" : "s"}.`
      );
    } else {
      setFlashError(
        `Published ${succeeded} · failed ${failures.length}. First error: ${failures[0].message}`
      );
    }
  }

  /**
   * Create + generate the Statewide edition for this week. Used by the
   * StatewideCallout when status=missing. The batch-generate flow would
   * also produce it (alongside 87 real-county editions) but editors may
   * want to set up Statewide on its own before publishing county
   * editions. This handler runs createEdition → generateEdition against
   * just the pseudo county.
   */
  async function handleGenerateStatewide(countyId: string) {
    setFlashError(null);
    setFlashMessage(null);
    setStatewideBusy("generate");
    try {
      const created = await createEdition({
        countyId,
        periodStart: week.mondayIso,
        periodEnd: week.sundayIso,
      });
      if (created.error) {
        setFlashError(`Create failed: ${created.error.message}`);
        return;
      }
      const editionId = created.data?.createEdition?.id;
      if (!editionId) {
        setFlashError("Create succeeded but returned no edition id.");
        return;
      }
      const generated = await generateEdition({ id: editionId });
      if (generated.error) {
        setFlashError(`Layout generation failed: ${generated.error.message}`);
        return;
      }
      setFlashMessage("Statewide edition generated.");
      refetch({ requestPolicy: "network-only" });
    } finally {
      setStatewideBusy(null);
    }
  }

  /**
   * Publish a single edition (used by StatewideCallout when its edition
   * is approved). Thin wrapper around publishEdition + refetch.
   */
  async function handlePublishStatewide(editionId: string) {
    setFlashError(null);
    setFlashMessage(null);
    setStatewideBusy("publish");
    try {
      const result = await publishEdition({ id: editionId });
      if (result.error) {
        setFlashError(`Publish failed: ${result.error.message}`);
        return;
      }
      setFlashMessage("Statewide edition published.");
      refetch({ requestPolicy: "network-only" });
    } finally {
      setStatewideBusy(null);
    }
  }

  async function handleBatchGenerate() {
    setFlashError(null);
    setFlashMessage(null);
    const result = await batchGenerate({
      periodStart: week.mondayIso,
      periodEnd: week.sundayIso,
    });
    if (result.error) {
      setFlashError(result.error.message);
      return;
    }
    const r = result.data?.batchGenerateEditions;
    if (r) {
      setFlashMessage(
        `Generated ${r.created} new · regenerated ${r.regenerated} · skipped ${r.skipped} · failed ${r.failed} (of ${r.totalCounties} counties).`
      );
      refetch({ requestPolicy: "network-only" });
    }
  }

  if (fetching && !data) {
    return <AdminLoader label="Loading dashboard..." />;
  }

  const primary = pickPrimaryAction(buckets.counts);

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-3xl font-bold text-foreground mb-1">Dashboard</h1>
          <p className="text-muted-foreground">
            This week:{" "}
            <span className="font-medium text-foreground">{week.label}</span>
            <span className="mx-2">·</span>
            <span className="font-medium text-foreground">
              {buckets.counts.published}
            </span>{" "}
            of {buckets.total || 87} counties published
          </p>
        </div>

        {/* Flash messages from batch generate. Success banner shrinks
         * to content so it reads as a toast; error banner keeps full
         * width in case the message is long. */}
        {flashMessage && (
          <Alert variant="success" className="mb-6 !w-fit">
            <AlertDescription>{flashMessage}</AlertDescription>
          </Alert>
        )}
        {flashError && (
          <Alert variant="error" className="mb-6">
            <AlertDescription>{flashError}</AlertDescription>
          </Alert>
        )}

        {/* Statewide edition — surfaced separately because it doesn't
         * roll into the "N of 87" county coverage. When Statewide is
         * missing/draft/in-review/approved the editor probably wants
         * quick access; when it's published we confirm success. */}
        {buckets.statewide && (
          <StatewideCallout
            bucket={buckets.statewide.bucket}
            countyId={buckets.statewide.row.county.id}
            editionId={buckets.statewide.row.currentEdition?.id ?? null}
            busy={statewideBusy}
            onGenerate={() =>
              handleGenerateStatewide(buckets.statewide!.row.county.id)
            }
            onPublish={() =>
              buckets.statewide!.row.currentEdition &&
              handlePublishStatewide(buckets.statewide!.row.currentEdition.id)
            }
          />
        )}

        {/* Primary action — adapts to current state */}
        <PrimaryActionCard
          action={primary}
          onBatchGenerate={handleBatchGenerate}
          onPublishAllApproved={handlePublishAllApproved}
          generating={batchState.fetching}
          publishing={publishing}
        />

        {/* Status breakdown */}
        <div className="grid grid-cols-2 md:grid-cols-5 gap-3 mb-8">
          <StatusCard
            value={buckets.counts.missing}
            label="Missing"
            accent="bg-zinc-300 text-zinc-700 border-zinc-300"
            tooltip="No edition has been generated for this week yet."
            href={`/admin/editions?periodStart=${week.mondayIso}`}
          />
          <StatusCard
            value={buckets.counts.draft}
            label="Draft"
            accent="bg-yellow-100 text-yellow-800 border-yellow-200"
            tooltip="Generated but not yet moved into review."
            href={`/admin/editions?status=draft&periodStart=${week.mondayIso}`}
          />
          <StatusCard
            value={buckets.counts.in_review}
            label="In review"
            accent="bg-amber-100 text-amber-800 border-amber-200"
            tooltip="Editor is walking the draft; approve or send back."
            href={`/admin/editions?status=in_review&periodStart=${week.mondayIso}`}
          />
          <StatusCard
            value={buckets.counts.approved}
            label="Approved"
            accent="bg-emerald-100 text-emerald-800 border-emerald-200"
            tooltip="Ready to publish — one click away."
            href={`/admin/editions?status=approved&periodStart=${week.mondayIso}`}
          />
          <StatusCard
            value={buckets.counts.published}
            label="Published"
            accent="bg-green-100 text-green-800 border-green-200"
            tooltip="Live on the public site."
            href={`/admin/editions?status=published&periodStart=${week.mondayIso}`}
          />
        </div>

        {/* How it works — a small explainer */}
        <HowItWorks />

        {/* Root Signal ingestion — placeholder */}
        <div className="bg-card rounded-lg border border-border my-8">
          <div className="px-6 py-4 border-b border-border">
            <h2 className="text-lg font-semibold text-foreground">
              Root Signal ingestion
            </h2>
          </div>
          <div className="px-6 py-10 flex flex-col items-center text-center">
            <div className="w-12 h-12 rounded-full bg-muted flex items-center justify-center mb-3">
              <Upload className="w-5 h-5 text-muted-foreground" />
            </div>
            <h3 className="text-base font-medium text-foreground mb-1">
              No signals this week
            </h3>
            <p className="text-sm text-muted-foreground max-w-sm mb-4">
              When Root Signal delivers new stories and topics, they'll land
              here for triage before edition drafts are generated.
            </p>
            <div className="flex items-center gap-3">
              <span className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-muted border border-border text-xs text-muted-foreground">
                <span className="w-1.5 h-1.5 rounded-full bg-muted-foreground/40" />
                Waiting for data
              </span>
            </div>
          </div>
        </div>

        {/* Quick actions */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Link
            href="/admin/workflow"
            className="bg-amber-600 hover:bg-amber-700 text-white rounded-lg p-5 transition-colors"
          >
            <div className="text-lg font-semibold mb-1">Review Board</div>
            <p className="text-amber-100 text-sm">
              Drag editions through the approval pipeline
            </p>
          </Link>
          <Link
            href="/admin/editions"
            className="bg-primary hover:bg-primary/90 text-primary-foreground rounded-lg p-5 transition-colors"
          >
            <div className="text-lg font-semibold mb-1">All Editions</div>
            <p className="text-primary-foreground/70 text-sm">
              Browse and filter all county editions
            </p>
          </Link>
        </div>
      </div>
    </div>
  );
}

// ─── PrimaryActionCard — adapts to the current state ────────────────────────

type Action =
  | { kind: "generate"; missing: number }
  | { kind: "publish"; approved: number }
  | { kind: "review"; inReview: number }
  | { kind: "nudge-drafts"; draft: number }
  | { kind: "all-set" };

function pickPrimaryAction(c: Record<Bucket, number>): Action {
  if (c.missing > 0) return { kind: "generate", missing: c.missing };
  if (c.approved > 0) return { kind: "publish", approved: c.approved };
  if (c.in_review > 0) return { kind: "review", inReview: c.in_review };
  if (c.draft > 0) return { kind: "nudge-drafts", draft: c.draft };
  return { kind: "all-set" };
}

function PrimaryActionCard({
  action,
  onBatchGenerate,
  onPublishAllApproved,
  generating,
  publishing,
}: {
  action: Action;
  onBatchGenerate: () => void;
  onPublishAllApproved: () => void;
  generating: boolean;
  publishing: boolean;
}) {
  let Icon = CircleDashed;
  let title = "";
  let description = "";
  let cta: React.ReactNode = null;
  let tone = "bg-muted border-border";
  // Saturated icon tint drawn from the same family as the card
  // background so the icon reads as part of the state signal rather
  // than a greyed-out afterthought on a tinted card.
  let iconTint = "text-foreground";

  switch (action.kind) {
    case "generate":
      Icon = AlertTriangle;
      title = `${action.missing} ${
        action.missing === 1 ? "county has" : "counties have"
      } no edition for this week`;
      description =
        "Generate fills empty weeks from eligible posts + widgets per county. You can regenerate any county individually later.";
      tone = "bg-amber-50 border-amber-200";
      iconTint = "text-amber-700";
      cta = (
        <Button onClick={onBatchGenerate} disabled={generating} size="sm">
          {generating ? "Generating..." : "Generate this week's editions"}
          {!generating && <ArrowRight className="size-3.5" />}
        </Button>
      );
      break;
    case "publish":
      Icon = CheckCircle2;
      title = `${action.approved} approved ${
        action.approved === 1 ? "edition is" : "editions are"
      } ready to publish`;
      description =
        "Published editions appear on the public site immediately. Use the list view if you want to spot-check before publishing.";
      tone = "bg-emerald-50 border-emerald-200";
      iconTint = "text-emerald-700";
      cta = (
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            onClick={onPublishAllApproved}
            disabled={publishing}
          >
            {publishing ? "Publishing..." : `Publish all ${action.approved}`}
            {!publishing && <ArrowRight className="size-3.5" />}
          </Button>
          <Button
            size="sm"
            variant="outline"
            render={<Link href={`/admin/editions?status=approved`} />}
          >
            Review list
          </Button>
        </div>
      );
      break;
    case "review":
      Icon = MessageSquare;
      title = `${action.inReview} ${
        action.inReview === 1 ? "edition needs" : "editions need"
      } editorial review`;
      description =
        "Walk the broadsheet, adjust slots and widgets, then approve or send back.";
      tone = "bg-amber-50 border-amber-200";
      iconTint = "text-amber-700";
      cta = (
        <Button size="sm" render={<Link href="/admin/workflow" />}>
          Open review board <ArrowRight className="size-3.5" />
        </Button>
      );
      break;
    case "nudge-drafts":
      Icon = FileText;
      title = `${action.draft} ${
        action.draft === 1 ? "draft is" : "drafts are"
      } waiting to start review`;
      description =
        "Drafts are fresh layouts the engine produced. Move them into review to start the editorial pass.";
      tone = "bg-yellow-50 border-yellow-200";
      iconTint = "text-yellow-700";
      cta = (
        <Button
          size="sm"
          render={<Link href={`/admin/editions?status=draft`} />}
        >
          Open drafts <ArrowRight className="size-3.5" />
        </Button>
      );
      break;
    case "all-set":
      Icon = CheckCircle2;
      title = "All caught up";
      description =
        "Every county has a published edition for this week. Next week's generation happens automatically when the period rolls over.";
      tone = "bg-green-50 border-green-200";
      iconTint = "text-green-700";
      cta = null;
      break;
  }

  return (
    <div className={`rounded-lg border ${tone} p-5 mb-8 flex items-start gap-4`}>
      <div className="shrink-0 mt-0.5">
        <Icon className={`size-5 ${iconTint}`} />
      </div>
      <div className="flex-1 min-w-0">
        <h2 className="text-base font-semibold text-foreground mb-1">
          {title}
        </h2>
        <p className="text-sm text-muted-foreground mb-3 max-w-2xl">
          {description}
        </p>
        {cta}
      </div>
    </div>
  );
}

// ─── StatewideCallout — status of the Statewide pseudo-county ──────────────

function StatewideCallout({
  bucket,
  editionId,
  busy,
  onGenerate,
  onPublish,
}: {
  bucket: Bucket;
  countyId: string;
  editionId: string | null;
  busy: null | "generate" | "publish";
  onGenerate: () => void;
  onPublish: () => void;
}) {
  // Each state surfaces (a) a one-liner title, (b) the most obvious
  // next action as an actual Button (not just a link to a filter page),
  // and (c) a Review-link secondary action so editors can spot-check
  // before doing anything destructive.
  const tone: Record<Bucket, string> = {
    missing: "bg-amber-50 border-amber-200",
    draft: "bg-yellow-50 border-yellow-200",
    in_review: "bg-amber-50 border-amber-200",
    approved: "bg-emerald-50 border-emerald-200",
    published: "bg-green-50 border-green-200",
  };
  // Lucide icon per state — no unicode glyphs. Icon color picks a
  // saturated tone from the same family as the background tint so the
  // icon reads as part of the state signal, not a grey afterthought.
  const StateIcon: Record<Bucket, typeof CircleDashed> = {
    missing: AlertTriangle,
    draft: FileText,
    in_review: MessageSquare,
    approved: CircleDashed,
    published: CheckCircle2,
  };
  const iconColor: Record<Bucket, string> = {
    missing: "text-amber-700",
    draft: "text-yellow-700",
    in_review: "text-amber-700",
    approved: "text-emerald-700",
    published: "text-green-700",
  };
  const Icon = StateIcon[bucket];
  const title: Record<Bucket, string> = {
    missing: "Statewide edition: not generated for this week",
    draft: "Statewide edition: draft",
    in_review: "Statewide edition: in review",
    approved: "Statewide edition: approved, ready to publish",
    published: "Statewide edition: published",
  };

  const reviewHref = editionId
    ? `/admin/editions/${editionId}`
    : "/admin/editions";

  let primary: React.ReactNode = null;
  switch (bucket) {
    case "missing":
      primary = (
        <Button
          size="sm"
          onClick={onGenerate}
          disabled={busy !== null}
        >
          {busy === "generate" ? "Generating…" : "Generate now"}
          {busy !== "generate" && <ArrowRight className="size-3.5" />}
        </Button>
      );
      break;
    case "draft":
    case "in_review":
      primary = (
        <Button size="sm" render={<Link href={reviewHref} />}>
          {bucket === "draft" ? "Open draft" : "Continue review"}
          <ArrowRight className="size-3.5" />
        </Button>
      );
      break;
    case "approved":
      primary = (
        <Button
          size="sm"
          onClick={onPublish}
          disabled={busy !== null}
        >
          {busy === "publish" ? "Publishing…" : "Publish now"}
          {busy !== "publish" && <ArrowRight className="size-3.5" />}
        </Button>
      );
      break;
    case "published":
      primary = (
        <Button size="sm" variant="outline" render={<Link href={reviewHref} />}>
          View edition
        </Button>
      );
      break;
  }

  return (
    <div
      className={`rounded-lg border ${tone[bucket]} px-4 py-3 mb-4 flex items-center justify-between gap-4`}
    >
      <span className="flex items-center gap-3 text-sm font-medium text-foreground">
        <Icon className={`size-4 ${iconColor[bucket]}`} aria-hidden />
        {title[bucket]}
      </span>
      <div className="flex items-center gap-2">
        {bucket !== "missing" && bucket !== "published" && (
          <Button
            size="sm"
            variant="outline"
            render={<Link href={reviewHref} />}
          >
            Review
          </Button>
        )}
        {primary}
      </div>
    </div>
  );
}

// ─── StatusCard — one per status bucket ─────────────────────────────────────

function StatusCard({
  value,
  label,
  accent,
  tooltip,
  href,
}: {
  value: number;
  label: string;
  accent: string;
  tooltip: string;
  href: string;
}) {
  return (
    <Link
      href={href}
      className="bg-card rounded-lg border border-border p-4 hover:border-border/80 hover:shadow-sm transition-all block"
      title={tooltip}
    >
      <div className="flex items-center justify-between mb-1">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
          {label}
        </span>
        <span className={`text-[10px] font-medium uppercase tracking-wide px-1.5 py-0.5 rounded border ${accent}`}>
          {label}
        </span>
      </div>
      <div className="text-2xl font-bold text-foreground leading-none">
        {value}
      </div>
    </Link>
  );
}

// ─── HowItWorks — compact explainer ─────────────────────────────────────────

function HowItWorks() {
  const steps: Array<{ num: string; title: string; body: string }> = [
    {
      num: "1",
      title: "Ingest",
      body: "Root Signal delivers posts + widgets keyed to county + week, or editors add them by hand.",
    },
    {
      num: "2",
      title: "Generate",
      body: "The layout engine drafts a broadsheet per county: rows, slots, sections, widget placements.",
    },
    {
      num: "3",
      title: "Review",
      body: "Editors walk the draft, adjust slots, rewrite copy, and swap templates. Status moves to Approved when ready.",
    },
    {
      num: "4",
      title: "Publish",
      body: "Approved editions go live on the public site. Published content stays up until the next week's edition lands.",
    },
  ];
  return (
    <section className="bg-card rounded-lg border border-border p-5">
      <h2 className="text-base font-semibold text-foreground mb-3">
        How the editorial flow works
      </h2>
      <ol className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {steps.map((s) => (
          <li key={s.num} className="flex gap-3">
            <span className="shrink-0 size-6 rounded-full bg-muted text-foreground text-xs font-semibold flex items-center justify-center">
              {s.num}
            </span>
            <div className="min-w-0">
              <div className="text-sm font-semibold text-foreground mb-0.5">
                {s.title}
              </div>
              <p className="text-xs text-muted-foreground leading-relaxed">
                {s.body}
              </p>
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}
