#!/usr/bin/env node

// seed-signal-inbox.mjs — Seeds mock `status = 'in_review'` posts for the
// Signal Inbox admin UI (/admin/signal-inbox). Emits idempotent SQL to stdout.
//
// Usage:
//   node data/seed-signal-inbox.mjs | docker compose exec -T postgres psql -U postgres -d rooteditorial
//   # or via Make:
//   make seed-signal-inbox
//
// Input: data/signal_inbox_mock.json
//
// Each mock entry becomes a post row with columns chosen to trigger
// specific review-flag derivations in Post::find_in_review_with_flags:
//   - extractionConfidence < 60          → `low_confidence`
//   - weight='heavy' + no post_meta.deck → `deck_missing_on_heavy`
//   - _duplicateOfSlug present           → `possible_duplicate`
//
// Entries marked `"status": "active"` + `_canonicalSlug` seed the canonical
// posts that the duplicate flag points at. They don't appear in the inbox
// themselves.
//
// Zero external dependencies (Node stdlib only).

import { readFileSync } from "fs";
import { createHash } from "crypto";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const entries = JSON.parse(
  readFileSync(join(__dirname, "signal_inbox_mock.json"), "utf8")
).filter((e) => !e._comment);

// Deterministic UUIDv5-ish from a slug so reruns hit the same rows.
const UUID_NS = "root-editorial/signal-inbox-mock/v1";
function slugUuid(slug) {
  const h = createHash("sha1").update(`${UUID_NS}|${slug}`).digest("hex");
  const p1 = h.slice(0, 8);
  const p2 = h.slice(8, 12);
  const p3 = "5" + h.slice(13, 16);
  const variant = (parseInt(h.slice(16, 18), 16) & 0x3f) | 0x80;
  const p4 = variant.toString(16).padStart(2, "0") + h.slice(18, 20);
  const p5 = h.slice(20, 32);
  return `${p1}-${p2}-${p3}-${p4}-${p5}`;
}

const esc = (v) => {
  if (v === null || v === undefined) return "NULL";
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  return `'${String(v).replace(/'/g, "''")}'`;
};

const out = (s) => process.stdout.write(s + "\n");

// Assign stable IDs. Every inbox post gets a slug from its title; canonical
// anchors use their explicit `_canonicalSlug`.
const idBySlug = new Map();
for (const e of entries) {
  const slug =
    e._canonicalSlug ||
    e.title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "")
      .slice(0, 60);
  e._id = slugUuid(slug);
  if (e._canonicalSlug) idBySlug.set(e._canonicalSlug, e._id);
}

// Resolve duplicate_of_id from slug references, AFTER every row has an id.
for (const e of entries) {
  if (e._duplicateOfSlug) {
    const target = idBySlug.get(e._duplicateOfSlug);
    if (!target) {
      throw new Error(
        `Mock post "${e.title}" references unknown canonical slug "${e._duplicateOfSlug}".`
      );
    }
    e._duplicateOfId = target;
  }
}

// ---------------------------------------------------------------------------
// SQL emission
// ---------------------------------------------------------------------------

out("-- signal-inbox mock seed (idempotent)");
out("-- regenerate with: node data/seed-signal-inbox.mjs");
out("BEGIN;");
out("");

for (const e of entries) {
  const status = e.status || "in_review";
  const submissionType = e.submissionType || "ingested";
  const extractionConfidence =
    e.extractionConfidence === undefined ? "NULL" : e.extractionConfidence;
  const duplicateOfId = e._duplicateOfId ? esc(e._duplicateOfId) : "NULL";

  // 1. Insert the post row with a deterministic id. Uses UPSERT on id so
  //    reruns refresh content without duplicating. is_seed=true keeps it
  //    distinguishable from real ingested data.
  out(`-- ${e.title}`);
  out(`INSERT INTO posts (`);
  out(`    id, title, body_raw, body_light, post_type, weight, priority,`);
  out(`    status, submission_type, is_urgent, is_evergreen, is_seed,`);
  out(`    location, extraction_confidence, duplicate_of_id, published_at`);
  out(`) VALUES (`);
  out(`    ${esc(e._id)},`);
  out(`    ${esc(e.title)},`);
  out(`    ${esc(e.bodyRaw)},`);
  out(`    ${esc(e.bodyLight || null)},`);
  out(`    ${esc(e.postType)},`);
  out(`    ${esc(e.weight || "medium")},`);
  out(`    0,`);
  out(`    ${esc(status)},`);
  out(`    ${esc(submissionType)},`);
  out(`    false,`);
  out(`    false,`);
  out(`    true,`);
  out(`    ${esc(e.location || null)},`);
  out(`    ${extractionConfidence},`);
  out(`    ${duplicateOfId},`);
  out(`    NOW() - INTERVAL '${Math.floor(Math.random() * 72)} hours'`);
  out(`) ON CONFLICT (id) DO UPDATE SET`);
  out(`    title = EXCLUDED.title,`);
  out(`    body_raw = EXCLUDED.body_raw,`);
  out(`    body_light = EXCLUDED.body_light,`);
  out(`    post_type = EXCLUDED.post_type,`);
  out(`    weight = EXCLUDED.weight,`);
  out(`    status = EXCLUDED.status,`);
  out(`    location = EXCLUDED.location,`);
  out(`    extraction_confidence = EXCLUDED.extraction_confidence,`);
  out(`    duplicate_of_id = EXCLUDED.duplicate_of_id,`);
  out(`    updated_at = NOW();`);
  out("");

  // 2. Optional post_meta. Skipped on `deck_missing_on_heavy` intent so the
  //    flag derivation fires; emitted when caller explicitly supplies meta.
  if (e.meta) {
    out(
      `INSERT INTO post_meta (post_id, kicker, byline, deck, updated) VALUES (`
    );
    out(`    ${esc(e._id)}, ${esc(e.meta.kicker || null)}, ${esc(e.meta.byline || null)},`);
    out(`    ${esc(e.meta.deck || null)}, ${esc(e.meta.updated || null)}`);
    out(`) ON CONFLICT (post_id) DO UPDATE SET`);
    out(`    kicker = EXCLUDED.kicker,`);
    out(`    byline = EXCLUDED.byline,`);
    out(`    deck = EXCLUDED.deck,`);
    out(`    updated = EXCLUDED.updated;`);
    out("");
  }

  // 3. Tag resolution. Expects tags to already exist (seeded via data/tags.json)
  //    — unknown slugs are skipped silently rather than created, to avoid
  //    polluting the real tag vocabulary.
  if (e.tags) {
    for (const t of e.tags) {
      out(`INSERT INTO taggables (tag_id, taggable_type, taggable_id)`);
      out(`SELECT id, 'post', ${esc(e._id)} FROM tags`);
      out(`WHERE kind = ${esc(t.kind)} AND value = ${esc(t.value)}`);
      out(`ON CONFLICT DO NOTHING;`);
    }
    out("");
  }
}

out("COMMIT;");
out("");
out("-- Done. Query the queue with:");
out("--   SELECT id, title, weight, extraction_confidence, duplicate_of_id");
out("--   FROM posts WHERE status = 'in_review' AND is_seed = true;");
