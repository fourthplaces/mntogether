#!/usr/bin/env node

/**
 * audit-seed.mjs — Reports per-post gaps against the Root Signal data contract.
 *
 * Reads: data/posts.json
 * Writes: stdout (human-readable summary) + data/audit-seed.out.json
 *         (machine-readable per-post report for the enrichment plan to consume).
 *
 * The thresholds here match §4.1 and §7 of
 * docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md. Keep them in sync when the
 * contract moves.
 *
 * Flags:
 *   --check     Exit 1 if any gap category regressed vs data/audit-seed.baseline.json.
 *               Used by `make audit-seed` in CI/hook contexts.
 *   --rebaseline
 *               Overwrite data/audit-seed.baseline.json with the current run.
 *               Use after finishing a pass to lock in the new floor.
 *
 * Usage:
 *   node data/audit-seed.mjs              # print + write out.json
 *   node data/audit-seed.mjs --check      # above, plus exit 1 if regressed
 *   node data/audit-seed.mjs --rebaseline # above, plus update baseline
 */

import { readFileSync, writeFileSync, existsSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const flags = new Set(process.argv.slice(2));

const rawPosts = JSON.parse(readFileSync(join(__dirname, "posts.json"), "utf8"));
const posts = rawPosts.filter((p) => !p._comment);

// ─── Contract thresholds (match ROOT_SIGNAL_DATA_CONTRACT.md §4.1) ────────────
const BODY_RAW_MIN = 250;
const BODY_HEAVY_MIN = 600;
const BODY_MEDIUM_MIN = 150;
const BODY_LIGHT_MIN = 30;
const BODY_LIGHT_MAX = 200;

// ─── Per-post-type required field groups (§7) ────────────────────────────────
// We report absence as a gap. "Soft" requirements (e.g., deck on heavy stories)
// are not enforced as hard failures but are tracked.
const REQUIRED_GROUPS_BY_TYPE = {
  story:     { required: [],                              recommended: ["media"], softHeavy: ["deck"] },
  update:    { required: ["actOrLink"],                   recommended: ["sourceAttribution"] },
  action:    { required: ["link"],                        recommended: ["contacts"] },
  event:     { required: ["datetime", "actOrLink"],       recommended: ["contacts"] },
  need:      { required: ["items", "contacts", "status"], recommended: ["scheduleEntries"] },
  aid:       { required: ["items", "contacts", "status"], recommended: ["scheduleEntries"] },
  person:    { required: ["person", "media"],             recommended: ["sourceAttribution"] },
  business:  { required: ["contacts", "scheduleEntries"], recommended: ["media", "link"], evergreen: true },
  reference: { required: ["items", "contacts"],           recommended: ["scheduleEntries"], evergreen: true },
};

// ─── Check helpers ────────────────────────────────────────────────────────────
function bodyRawOf(p) {
  // Matches the fallback in data/seed.mjs line 231.
  return p.bodyRaw || p.bodyHeavy || p.bodyMedium || p.bodyLight || "";
}

function hasActOrLink(p) {
  // "Reader needs a way to act" — link OR any contact
  return Boolean(p.link) || Boolean(p.contacts && p.contacts.length > 0);
}

function auditPost(p) {
  const gaps = [];

  // §3 core
  if (!p.title) gaps.push("core:title_missing");
  if (!p.postType) gaps.push("core:post_type_missing");
  if (!p.weight) gaps.push("core:weight_missing");
  if (p.priority == null) gaps.push("core:priority_missing");

  // §3.2 body tiers
  const bodyRaw = bodyRawOf(p);
  if (bodyRaw.length < BODY_RAW_MIN) {
    gaps.push(`body:raw_below_${BODY_RAW_MIN}_chars (got ${bodyRaw.length})`);
  }
  if (p.weight === "heavy" && (!p.bodyHeavy || p.bodyHeavy.length < BODY_HEAVY_MIN)) {
    gaps.push(`body:heavy_missing_or_below_${BODY_HEAVY_MIN} (got ${p.bodyHeavy?.length ?? 0})`);
  }
  if (["heavy", "medium"].includes(p.weight) && (!p.bodyMedium || p.bodyMedium.length < BODY_MEDIUM_MIN)) {
    gaps.push(`body:medium_missing_or_below_${BODY_MEDIUM_MIN} (got ${p.bodyMedium?.length ?? 0})`);
  }
  if (!p.bodyLight || p.bodyLight.length < BODY_LIGHT_MIN) {
    gaps.push(`body:light_missing_or_below_${BODY_LIGHT_MIN} (got ${p.bodyLight?.length ?? 0})`);
  }
  if (p.bodyLight && p.bodyLight.length > BODY_LIGHT_MAX) {
    gaps.push(`body:light_above_${BODY_LIGHT_MAX} (got ${p.bodyLight.length})`);
  }

  // §3.5 taxonomy
  if (!p.tags?.topic || p.tags.topic.length === 0) gaps.push("tags:topic_missing");
  if (!p.tags?.serviceArea || p.tags.serviceArea.length === 0) gaps.push("tags:service_area_missing");

  // §3.6 source (seed represents as sourceAttribution today)
  if (!p.sourceAttribution) gaps.push("source:attribution_missing");

  // §3.7 editorial metadata
  if (!p.meta) {
    gaps.push("meta:block_missing");
  } else {
    if (!p.meta.kicker) gaps.push("meta:kicker_missing");
    if (!p.meta.byline) gaps.push("meta:byline_missing");
    if (p.weight === "heavy" && !p.meta.deck) gaps.push("meta:deck_missing_on_heavy");
  }

  // §7 field-group requirements by post_type
  const rules = REQUIRED_GROUPS_BY_TYPE[p.postType];
  if (rules) {
    for (const req of rules.required) {
      if (req === "actOrLink") {
        if (!hasActOrLink(p)) gaps.push(`type_group:${p.postType}_needs_contacts_or_link`);
        continue;
      }
      const val = p[req];
      const empty = val == null || (Array.isArray(val) && val.length === 0);
      if (empty) gaps.push(`type_group:${p.postType}_missing_${req}`);
    }
    if (rules.evergreen && p.isEvergreen !== true) {
      // `isEvergreen` is defaulted true for reference/business in seed.mjs, but
      // if someone explicitly sets false we flag it.
      if (p.isEvergreen === false) gaps.push(`type_group:${p.postType}_should_be_evergreen`);
    }
  } else if (p.postType) {
    gaps.push(`type_group:unknown_post_type_${p.postType}`);
  }

  // §3.8 media (recommended for heavy weights)
  if (p.weight === "heavy" && (!p.media || p.media.length === 0)) {
    gaps.push("media:no_hero_on_heavy");
  }

  return gaps;
}

// ─── Run audit ────────────────────────────────────────────────────────────────
const report = {
  total: posts.length,
  perPost: [],
  gapCounts: {},
  perType: {},
  perWeight: {},
  perfectPosts: 0,
};

for (const p of posts) {
  const gaps = auditPost(p);
  report.perPost.push({
    title: p.title?.slice(0, 80),
    postType: p.postType,
    weight: p.weight,
    bodyRawLen: bodyRawOf(p).length,
    gaps,
  });

  if (gaps.length === 0) report.perfectPosts++;

  for (const g of gaps) {
    const key = g.split(" (")[0]; // strip the "(got N)" detail for counting
    report.gapCounts[key] = (report.gapCounts[key] ?? 0) + 1;
  }

  const t = p.postType ?? "untyped";
  report.perType[t] = report.perType[t] ?? { total: 0, gapsTotal: 0 };
  report.perType[t].total++;
  report.perType[t].gapsTotal += gaps.length;

  const w = p.weight ?? "untyped";
  report.perWeight[w] = report.perWeight[w] ?? { total: 0, gapsTotal: 0 };
  report.perWeight[w].total++;
  report.perWeight[w].gapsTotal += gaps.length;
}

// ─── Human-readable output ────────────────────────────────────────────────────
console.log(`# Seed data audit vs. Root Signal data contract`);
console.log(`# Generated: ${new Date().toISOString()}`);
console.log(``);
console.log(`Total posts: ${report.total}`);
console.log(`Perfect (zero gaps): ${report.perfectPosts} (${((report.perfectPosts / report.total) * 100).toFixed(1)}%)`);
console.log(``);

console.log(`## Gap frequency (top offenders)`);
const sortedGaps = Object.entries(report.gapCounts).sort((a, b) => b[1] - a[1]);
for (const [gap, count] of sortedGaps) {
  console.log(`  ${String(count).padStart(4)}  ${gap}`);
}
console.log(``);

console.log(`## Per-type average gaps`);
for (const [t, stats] of Object.entries(report.perType).sort((a, b) => (b[1].gapsTotal / b[1].total) - (a[1].gapsTotal / a[1].total))) {
  const avg = (stats.gapsTotal / stats.total).toFixed(1);
  console.log(`  ${String(stats.total).padStart(4)} ${t.padEnd(12)}  avg ${avg} gaps/post`);
}
console.log(``);

console.log(`## Per-weight average gaps`);
for (const [w, stats] of Object.entries(report.perWeight).sort((a, b) => (b[1].gapsTotal / b[1].total) - (a[1].gapsTotal / a[1].total))) {
  const avg = (stats.gapsTotal / stats.total).toFixed(1);
  console.log(`  ${String(stats.total).padStart(4)} ${w.padEnd(12)}  avg ${avg} gaps/post`);
}
console.log(``);

console.log(`## Worst 10 offenders`);
const worst = [...report.perPost].sort((a, b) => b.gaps.length - a.gaps.length).slice(0, 10);
for (const p of worst) {
  console.log(`  (${String(p.gaps.length).padStart(2)} gaps) [${p.weight}/${p.postType}] ${p.title}`);
}

// ─── Machine-readable output ──────────────────────────────────────────────────
writeFileSync(
  join(__dirname, "audit-seed.out.json"),
  JSON.stringify(report, null, 2)
);
console.log(``);
console.log(`Machine-readable report: data/audit-seed.out.json`);

// ─── Regression check / rebaseline ────────────────────────────────────────────
const baselinePath = join(__dirname, "audit-seed.baseline.json");

if (flags.has("--rebaseline")) {
  writeFileSync(baselinePath, JSON.stringify(report, null, 2));
  console.log(``);
  console.log(`Baseline updated: ${baselinePath}`);
}

if (flags.has("--check")) {
  if (!existsSync(baselinePath)) {
    console.error(`\nNo baseline at ${baselinePath}. Run with --rebaseline first.`);
    process.exit(2);
  }
  const baseline = JSON.parse(readFileSync(baselinePath, "utf8"));
  const regressions = [];
  const allKeys = new Set([
    ...Object.keys(report.gapCounts),
    ...Object.keys(baseline.gapCounts),
  ]);
  for (const key of allKeys) {
    const now = report.gapCounts[key] ?? 0;
    const then = baseline.gapCounts[key] ?? 0;
    if (now > then) regressions.push({ key, then, now, delta: now - then });
  }
  if (regressions.length > 0) {
    console.error(``);
    console.error(`REGRESSION: gap counts increased vs baseline`);
    for (const r of regressions) {
      console.error(`  +${r.delta}  ${r.key}  (was ${r.then}, now ${r.now})`);
    }
    console.error(``);
    console.error(`If this is intentional progress (e.g. you added more posts and`);
    console.error(`some new gaps are expected), re-run with --rebaseline to lock`);
    console.error(`in the new floor.`);
    process.exit(1);
  } else {
    console.log(``);
    console.log(`Check: no regressions vs baseline ✓`);
  }
}
