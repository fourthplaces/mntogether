# Taxonomy Expansion Brief

**Audience:** Root Signal engineering.
**Relationship to the API request:** this brief is the *why* to the request doc's *what*. The request doc's contract assumes Root Signal's taxonomy will grow to cover Profile, LocalBusiness, and Opportunity; this brief is the argument for each, with scouting strategy, exclusion criteria, field shapes, and Editorial envelope mapping.

**How to read alongside the request doc:** the request doc's §15.1 gives the one-line Root-Signal→Editorial `post_type` mapping. This brief expands each row with the full justification plus the specific envelope fields Editorial populates from each proposed signal type.

---

# Expanding Root Signal's Surface Area

**To:** Craig
**Context:** Root Editorial integration — the downstream CMS needs coverage across five content areas that the current Scout + Signal pipeline undercovers. This doc argues that the cleanest path forward is **expanding the signal taxonomy beyond the current six types**, rather than stretching them to absorb everything.

---

## 1. The core argument: loosen the six-type constraint

The current six types — Gathering, Resource, HelpRequest, Announcement, Concern, Condition — reflect a specific framing: *community signals around response and repair.* Something is wrong (Concern), something persists (Condition), someone needs help (HelpRequest), resources exist for those in need (Resource), events bring people together around these dynamics (Gathering), authorities respond (Announcement). This is a **mutual-aid and civic-response lens**, and it's good for that purpose.

It's incomplete for a weekly community broadsheet. A real local information ecosystem also carries:

- **People** — the neighbor running the after-school program, the nonprofit director, the feature-profile subject
- **Local commerce** — the independent restaurants, shops, and service providers that are the economic texture of place
- **Civic access** — deadline-driven actions (apply for MNsure, file your M1PR, testify at the council) that are neither mutual aid nor advisory
- **Jobs** — paid employment at local employers
- **Culture** — art fairs, festivals, poetry readings, powwows

Trying to force this content into the six existing types degrades the system in four ways:

**1. Prompts get fuzzy.** If "Resource" has to mean *mutual aid AND government programs AND local businesses*, the LLM extraction loses precision. The current prompt literally says *"A business offering paid services is NOT a Resource"* — rewriting it to un-exclude paid local biz while still excluding chains, and to include gov benefit programs while still excluding federal informational content, produces a definition that's long, hedged, and ambiguous.

**2. Scouting strategies diverge per content kind.** A scout looking for food shelves crawls mutual-aid networks. A scout looking for independent restaurants crawls chamber directories. A scout looking for MNsure enrollment crawls .gov domains. A scout looking for staff profiles crawls `/team` pages. These are genuinely different gathering strategies with different seed sets, different rate profiles, and different extraction patterns. Collapsing them under one type means one scout with one confused mandate.

**3. Discriminator fields become a second taxonomy.** Adding `cost`, `kind`, `ownership_type`, `compensation`, `deadline` fields to Resource and HelpRequest to disambiguate what got lumped together is just smuggling a type system inside a type. Downstream consumers end up switching on discriminators anyway. Better to promote the discriminators to first-class types when they drive different scouting, different sources, and different consumer mappings — which is exactly the situation here.

**4. Editorial already picked 9 types.** Root Editorial landed on story, update, action, event, need, aid, person, business, reference. That wasn't arbitrary — it maps to real differences in layout, reader expectation, byline treatment, and weight. If Signal collapses those into 6 upstream and Editorial has to re-infer downstream, we've built a lossy pipe. Every editorial decision (how to weight, which template, which deck, what kicker) becomes a guess against under-typed data.

**The cost of adding a type is bounded.** One struct, one NodeType variant, one GraphQL union variant, one prompt section, one projector arm, tests. A few hundred lines, most of it mechanical. The cost of stretching a type is *recurring* — every extraction, every consumer mapping, every filter query, every downstream integration carries the ambiguity forward.

**The six types were a V1 decision.** Early taxonomies ossify easily. Revisiting them as the product matures is healthy, not heretical. The fact that Editorial — a downstream consumer designed independently — landed on 9 differentiated types is a signal the upstream taxonomy is under-specified.

---

## 2. Proposed expansion: four new signal types

### 2.1 `Profile` (new) → Editorial `person`

**Captures:** A person or group doing community work — nonprofit staff, volunteer-of-the-month, newspaper feature subject, organizer bio, board members, interviewed community members.

**Why not stretch ActorNode:** `ActorNode` is attribution infrastructure (who authored this, who is mentioned here, canonical_key, discovery_depth). A Profile signal is editorial content — intentional, selected, often rich with quote + photo + backstory. Conflating them means either bloating ActorNode with display-layer fields, or putting display data in the graph's attribution layer and hoping consumers sort it out.

**Why Profile and Recognition are one type:** The data shape is identical (name, role, org, photo, quote, bio, external_url). The scouting strategy is the same (scan org pages + newspaper features). The difference is depth of record, not kind of record. Editorial can route a thin record to a directory layout and a rich record to a spotlight feature — that's a presentation decision, not a type decision.

**Distinct scouting:** nonprofit `/staff`, `/team`, `/board`, `/about-us` pages; local newspaper feature archives; community radio interview pages.

**Proposed Profile fields (Signal-side):** `name`, `role` (tagline), `org`, `bio`, `quote`, `photo_url`, `external_url`, plus NodeMeta.

**Editorial envelope mapping:**

| Profile field | Editorial envelope | Notes |
|---|---|---|
| `name` | `field_groups.person.name` | required |
| `role` | `field_groups.person.role` | required |
| `org` | resolves to `source.organization` (if the profile subject is an employee of an org) or `meta.byline` context | not a dedicated envelope field |
| `bio` | `field_groups.person.bio` | required |
| `quote` | `field_groups.person.quote` | optional but strongly recommended |
| `photo_url` | `field_groups.person.photo_url` **and** `field_groups.media[0]` | hero image for the `person` post_type |
| `external_url` | `field_groups.link.url` with `label: "Learn more"` | optional |

Profile → `post_type: "person"`, default `weight: "medium"` per §6 of the request. See §16.6 of the request for a complete worked example.

### 2.2 `LocalBusiness` (new) → Editorial `business`

**Captures:** Independent, locally-owned restaurants, shops, service providers, tradespeople, co-ops, small manufacturers. Explicitly excludes national/multi-state chains and franchise locations that funnel revenue upstream (McDonald's, Subway, etc., even when solo-operated).

**Why not stretch Resource:** Resource's mutual-aid framing is load-bearing. Rewriting the prompt to admit paid commercial entities means the definition becomes "anything available to the community" — which drifts toward meaninglessness. And the scouting strategy is fundamentally different: chamber directories and shop-local aggregators vs. mutual-aid networks.

**Distinct scouting:** chamber of commerce directories, neighborhood biz associations, shop-local aggregators, independent review sites, neighborhood newspaper business listings.

**Proposed LocalBusiness fields (Signal-side):** `name`, `category`, `hours` (schedule), `location`, `contact`, `action_url`, `ownership_type` (independent / co-op / nonprofit-affiliated — explicit exclusion of chain/franchise), plus NodeMeta.

**Editorial envelope mapping:**

| LocalBusiness field | Editorial envelope | Notes |
|---|---|---|
| `name` | `source.organization.name` **and** `title` (often identical) | resolves via org dedup (§7.1) |
| `category` | `tags.topic[]` — pick nearest from controlled vocabulary (`food`, `culture`, `community`, etc.) | one or more topic tags |
| `hours` (Schedule RRULE) | `field_groups.schedule[]` expanded by day-of-week | per §15.8 |
| `location` | `source.organization.address`, plus `location`, `zip_code`, `latitude`/`longitude` | per §7.1 |
| `contact` | `field_groups.contacts[]` (polymorphic: phone, email, website, address, social) | per §5.10 |
| `action_url` | `field_groups.link.url` with `label: "Visit site"` (or similar) | optional |
| `ownership_type` | **not a structured Editorial field** | Root Signal should use this locally to filter; Editorial only wants the ones where `ownership_type ∈ {independent, co-op, nonprofit-affiliated}`. Chain/franchise posts must be dropped by Root Signal before emitting. |

LocalBusiness → `post_type: "business"`, `weight: "medium"`, `is_evergreen: true` (see §6 of the request). See §16.7 for a worked example.

### 2.3 `Opportunity` (new) → Editorial `action`

**Captures:** Deadline-driven, reader-takeable actions with a procedure. Covers both **civic participation** (voting, public comment, testifying, running for office, contacting reps) and **benefit access** (MNsure, SNAP, M1PR, EAP, WIC, state grants, free radon kits, colon cancer screening). Unified because the data shape and urgency discipline are identical; the difference (political vs. administrative) is downstream editorial nuance.

**Why not stretch Resource or Announcement:** Resource is ongoing mutual aid — no deadline discipline, no procedure. Announcement tells the reader something happened — it doesn't ask them to do something by a date. The thing that's missing is a signal whose *center* is the reader's procedural obligation with a hard cutoff.

**Distinct scouting:** state/county/city .gov domains (state.mn.us, revenue.state.mn.us, mnsure.org, county sites), advocacy action pages, filing calendars, public comment portals.

**Proposed Opportunity fields (Signal-side):** `subject`, `deadline` (structured DateTime + `deadline_text`), `eligibility`, `procedure`, `outcome`, `action_url`, `channel` (portal/form/phone/in-person), plus NodeMeta.

**Editorial envelope mapping:**

| Opportunity field | Editorial envelope | Notes |
|---|---|---|
| `subject` | `title` | 20–120 chars, no calendar dates in title |
| `deadline` (DateTime) | `field_groups.link.deadline` | required — the whole point of `action` post_type |
| `deadline_text` | `meta.kicker` (e.g., "Deadline: April 28") or inline in `body_raw` | optional |
| `eligibility` | inline in `body_raw` and `body_medium` | no structured field |
| `procedure` | inline in body tiers (`body_raw` / `body_heavy`) | describe in the body |
| `outcome` | `meta.deck` (if `weight=heavy`) or `body_medium` | one-sentence "what you get" |
| `action_url` | `field_groups.link.url` with `label: "Take action"` (or similar) | required |
| `channel` | flavours `meta.kicker` or `meta.byline` | e.g., kicker="Public Comment" vs "Benefit" |

Opportunity → `post_type: "action"`, `weight: "medium"` (heavy for high-urgency civic deadlines). See §16.4 for a worked Announcement-style example; Opportunity shape is near-identical with deadline always populated.

**On Q2 below** (is Opportunity one type or split CivicAction + BenefitProgram?) — **Editorial is agnostic**. Both would map to `action`. A single `Opportunity` type is fine for us; if you split for your own reasons, both variants land at the same `post_type` and we'll distinguish on `meta.kicker`.

### 2.4 `Job` (new) → no current Editorial destination

**Captures:** Paid employment at local, independent employers — nonprofit roles, part-time at locally-owned businesses, municipal employment, community org openings. Not remote-national roles.

**Why not stretch HelpRequest:** HelpRequest is framed as community-sourced volunteer and mutual-aid asks. Paid hiring shares the "someone needs someone" shape but has fundamentally different scouting (job boards vs. community posts), different editorial treatment, and different reader expectation. Lumping them means a volunteer cleanup and a full-time nonprofit director role appear as the same kind of thing in the graph.

**Distinct scouting:** state job listings (mn.gov/careers), nonprofit job aggregators (MCN jobs, MAP for Nonprofits), chamber job pages, community foundation job boards, independent biz hiring signs (hard, may need social scraping).

**Proposed Job fields (Root Signal-side):** `role`, `employer`, `employer_type` (independent local / municipal / nonprofit / gov), `compensation`, `schedule_type` (full-time / part-time / seasonal), `location`, `application_url`, `deadline`, plus NodeMeta.

**Editorial envelope mapping:** none. Editorial has no `job` post_type. If Root Signal builds a Job type for other consumers, **do not emit Job signals to Editorial's ingest endpoint** — they will 422 on unknown `post_type`. If civic-content job coverage becomes a priority later, a `job` post_type would be added and this mapping extended; until that happens, Job signals stay inside Root Signal.

---

## 3. What stays within the existing types

### 3.1 Local government events → Gathering → Editorial `event`

4th of July at the water park, summer rec programs, neighborhood cleanup days, council meetings. These are genuinely time-bound events at a place. Gathering is the right home. Just needs prompt examples covering civic events, and .gov seed domains.

### 3.2 Policy advisories → Announcement → Editorial `action` or `update`

Policy changes, shelter openings, closures, effective dates for new ordinances. Announcement already fits, though it would benefit from a structured `deadline` field for effective dates. Announcements with a structured deadline map to Editorial `action`; those without map to `update`.

### 3.3 Cultural events → Gathering → Editorial `event`

Art fairs, gallery openings, poetry readings, festivals, music in the park, powwows, library programs, maker markets, author talks, film screenings. All time-bound events at a place. Gathering fits cleanly. Just needs:

- **Prompt examples** covering culture explicitly: *"art fair, gallery opening, poetry reading, festival, maker market, powwow, author talk, film screening, library program, cultural celebration."* Current examples skew toward protests, cleanups, workshops.
- **Seed domains**: arts/cultural nonprofits, galleries, theaters, music venues, library event calendars, and MN-specific cultural community orgs (Indigenous cultural orgs, East African community groups, Latino cultural orgs, Hmong cultural orgs, neighborhood arts collectives).
- **Optional** `kind` or `category` label on Gathering (civic / protest / cultural / workshop / cleanup) so Editorial can route without re-inferring. If you add this label, Editorial will use it to populate `meta.kicker` and `tags.topic` appropriately.

---

## 4. Cross-cutting asks (apply regardless of taxonomy decision)

### 4.1 Seed domain expansion
The single biggest leverage point. Current bootstrap favors community/mutual-aid platforms (Eventbrite, GoFundMe, Linktree, Reddit) and rejects some federal .gov explicitly. Needs region-aware expansion — for MN v1:
- State, county, and city .gov (state.mn.us subdomains, county sites, city sites)
- Chamber of commerce and local business association directories
- Arts/cultural nonprofit domains
- Nonprofit job boards and community foundation sites
- Newspaper feature archives
- Nonprofit staff/team pages

### 4.2 Prompt broadening
Current ResourceOffer prompt's *"must be free or publicly available"* constraint narrows Scout away from civic access, local biz, and paid work. If we adopt new types, this prompt can stay narrow (preserving Resource's mutual-aid meaning) and the new types carry their own precise prompts. If we don't, Resource's prompt becomes a long, hedged definition.

### 4.3 Deadline as a first-class field
Benefit programs, filing windows, application cutoffs, and public comment periods all have hard dates driving editorial urgency and layout. Required on the new `Opportunity` type. Also worth adding to `Announcement` for effective-date discipline. On Editorial's side, both map to `field_groups.link.deadline` as a timestamptz; omission on an `action` post_type is a soft-fail (post lands `in_review` with an editor note).

### 4.4 Actor projection
Regardless of whether Profile becomes its own type, ActorNode could use a richer output projection. If we add Profile, ActorNode stays as attribution infrastructure. If we don't, ActorNode needs display-layer fields bolted on. **Related ask from the request doc (§17 item 3):** Editorial needs Signal to persist the `organization_id` / `individual_id` we return on 201 alongside your `ActorNode.canonical_key` for the dedup fast-path. That's orthogonal to Profile-as-a-type, but it lives in the same actor-graph neighbourhood.

---

## 5. Suggested priority

1. **Seed domain expansion** (gov, local biz, arts, jobs) — high impact, low complexity, unblocks all five new-type categories.
2. **Add `Profile` and `LocalBusiness` signal types** — these unlock `person` and `business` post_types on the Editorial side, which are otherwise unusable.
3. **Add `Opportunity` signal type** — unlocks deadline-driven `action` post coverage for civic participation and benefit access.
4. **Gathering prompt examples + category label** — culture coverage, trivial edit.
5. **Deadline field on Announcement** — smaller-scope schema addition.
6. **`Job`** — builds internally if useful, but no Editorial destination until a `job` post_type is added (see §2.4).

---

## 6. Open design questions

These are unresolved conversations, not blockers. The integration proceeds without them; answers refine it.

1. **Opportunity split or unified?** Is `Opportunity` the right single type for both civic participation and benefit access, or should the two split (`CivicAction` vs `BenefitProgram`)? Editorial has no preference — both variants land at `action` either way. Root Signal decides based on scouting and extraction needs.

2. **Long-form journalism as a first-class signal?** Currently the briefing on `SituationNode` handles narrative synthesis, but scouted long-form journalism (a reporter's investigative piece, a community-newspaper feature archive) has nowhere to land as a first-class signal — it doesn't reduce to atomic signals the way a scraped event does. A `Report` or `Article` signal type carrying `body_markdown`, `byline`, `publication`, `original_url`, `publication_date` would map directly to Editorial's `story` post_type and close the gap. Worth considering alongside the four proposed expansions.

3. **Other gaps?** Coverage categories not surfaced above but that civic broadsheets carry in practice — flag them. The taxonomy is a live conversation for as long as the integration is evolving.
