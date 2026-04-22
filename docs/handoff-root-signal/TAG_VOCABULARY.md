# Controlled Tag Vocabulary

Editorial accepts three kinds of tags on each submitted post: `topic`, `service_area`, and `safety`. This document is the authoritative reference for what values each kind accepts and how Editorial treats unfamiliar values.

**Source of truth:** `data/tags.json` in the Editorial repo. If anything in this doc contradicts the JSON, the JSON wins.

See `ROOT_SIGNAL_API_REQUEST.md` §5.7 and §10 for how tags flow through the envelope and validation.

---

## 1. `tags.topic`  —  thematic, open vocabulary

Required on every post; at least one topic tag.

Submit as hyphen-case slugs. **The list below is a starting point, not a ceiling.** Root Signal should propose new topic slugs freely whenever a post doesn't fit an existing one — the vocabulary is intentionally open and expected to grow as the system sees real content. Submissions with unknown topic slugs are accepted; Editorial auto-creates the tag row, flags the post `in_review`, and an editor confirms the new slug into the canonical list on review.

**Do not force-fit.** If a post is about, say, broadband infrastructure and "public-works" is the nearest existing slug, propose `broadband` as a new slug rather than conflating it. If the new slug appears repeatedly and editorially makes sense, it graduates to canonical.

**Current canonical list** (exported 2026-04-22):

| Slug | Display name |
|---|---|
| `food` | Food |
| `housing` | Housing |
| `community` | Community |
| `education` | Education |
| `employment` | Employment |
| `health` | Health |
| `legal` | Legal |
| `immigration` | Immigration |
| `language-access` | Language Access |
| `culture` | Culture |
| `voting` | Voting |
| `transit` | Transit |
| `community-voices` | Community Voices |
| `environment` | Environment |
| `donations` | Donations |
| `childcare` | Childcare |
| `winter-gear` | Winter Gear |
| `census` | Census |
| `resettlement` | Resettlement |
| `safety` | Safety |
| `public-works` | Public Works |

**Format conventions for proposing new slugs:**
- Lowercase.
- Words separated by hyphens.
- Concise: one or two words where possible. `mental-health`, not `mental-health-and-wellness`.
- Thematic, not geographic — geography belongs in `service_area` (see §2). Neighborhoods belong in the `tags.neighborhood` kind (if needed) or in `location`, not `topic`.
- Not a category discriminator. Post types (`business`, `person`, `action`, etc.) already carry that information — don't duplicate via topic tags like `restaurant` or `nonprofit`.

If a slug proposal would violate one of these, drop it and surface the thematic core instead. `restaurant-opening-in-hennepin` → topic `food` + service_area `hennepin-county` + post_type `business` or `event`.

---

## 2. `tags.service_area`  —  geographic, closed vocabulary

Required on every post; at least one service-area tag.

Submit as hyphen-case slugs. **The list is closed**: 87 Minnesota counties plus the pseudo-value `statewide`. Unknown slugs hard-fail with error code `unknown_service_area`.

- `statewide` — post is MN-wide. Lands in the Statewide edition (pseudo-county, `is_pseudo = true`). Does not propagate to individual county editions.
- Multi-county posts: include multiple slugs in the array, e.g., `["hennepin-county", "ramsey-county", "dakota-county"]`. The post appears in each tagged county's edition independently.

**Full list (88 slugs):**

```
aitkin-county           anoka-county            becker-county           beltrami-county
benton-county           big-stone-county        blue-earth-county       brown-county
carlton-county          carver-county           cass-county             chippewa-county
chisago-county          clay-county             clearwater-county       cook-county
cottonwood-county       crow-wing-county        dakota-county           dodge-county
douglas-county          faribault-county        fillmore-county         freeborn-county
goodhue-county          grant-county            hennepin-county         houston-county
hubbard-county          isanti-county           itasca-county           jackson-county
kanabec-county          kandiyohi-county        kittson-county          koochiching-county
lac-qui-parle-county    lake-county             lake-of-the-woods-county le-sueur-county
lincoln-county          lyon-county             mahnomen-county         marshall-county
martin-county           mcleod-county           meeker-county           mille-lacs-county
morrison-county         mower-county            murray-county           nicollet-county
nobles-county           norman-county           olmsted-county          otter-tail-county
pennington-county       pine-county             pipestone-county        polk-county
pope-county             ramsey-county           red-lake-county         redwood-county
renville-county         rice-county             rock-county             roseau-county
scott-county            sherburne-county        sibley-county           stearns-county
steele-county           stevens-county          st-louis-county         swift-county
todd-county             traverse-county         wabasha-county          wadena-county
waseca-county           washington-county       watonwan-county         wilkin-county
winona-county           wright-county           yellow-medicine-county  statewide
```

**Slug derivation rules** (for Signal's own validation):
- Lowercase the county name.
- Replace spaces with `-`.
- Strip `.` characters (e.g., `St. Louis` → `st-louis`).
- Append `-county`.
- Exception: the pseudo-county is `statewide`, no suffix.

If a generator produces a slug not in this list, the county name is likely misspelled — surface the mismatch rather than coercing.

---

## 3. `tags.safety`  —  access-policy modifiers, reserved vocabulary

Optional. **Safety tags are access-policy modifiers**, not descriptions of what a post is about. They surface policies at a service or organisation that a reader would otherwise have to ask about — policies that, if unstated, would cause someone to hesitate before seeking care, showing up, or using the offer.

The shape of a safety tag is always "something about how this service is delivered that removes hesitation."

**Not safety tags:** descriptions of what the service is (`warming-shelter`, `food-pantry`, `mental-health-crisis-line`). Those live in topic tags or are inferred from post_type.

**Are safety tags:** written-down policies that change who feels comfortable walking through the door.

Submit as hyphen-case slugs. **Unknown slugs hard-fail** with error code `unknown_value`. The list is intended to be reasonably exhaustive — if a service has a genuinely relevant policy that isn't represented, surface it through the integration channel for addition; do not invent slugs per-submission.

**Identity and documentation** — who will I need to prove I am?

| Slug | Meaning |
|---|---|
| `no-id-required` | No identification is asked for or required to receive the service. |
| `immigration-status-safe` | The service will not ask about, record, or report a person's immigration status to any agency. |
| `ice-safe` | Explicit sanctuary posture: the service will not voluntarily cooperate with ICE. Narrower than `immigration-status-safe`; a service may qualify for both. |
| `anonymous-access` | No name or identifying information required to receive service. |
| `no-background-check` | No criminal-record or background check for entry (for housing, employment, training programs, some shelters). |

**Cost and coverage** — can I afford this?

| Slug | Meaning |
|---|---|
| `free-service` | Service is genuinely no-cost. Not "free with insurance," not "free up to a limit." |
| `sliding-scale` | Fees are adjusted based on ability to pay; no one is turned away for inability to pay. |
| `no-insurance-required` | Health, legal, or other services provided regardless of insurance status. |

**Privacy and reporting** — what happens to what I share?

| Slug | Meaning |
|---|---|
| `confidential` | The visit and information shared are kept confidential; no sharing with other agencies, family members, employers, etc., without explicit consent. |
| `trauma-informed` | Staff are trained in trauma-informed care; questions and processes account for past trauma rather than re-triggering it. |

**How the service is accessed** — do I need to plan this out?

| Slug | Meaning |
|---|---|
| `walk-in` | No appointment required; drop in during listed hours. |
| `no-referral-required` | No physician, caseworker, or other referral needed to be seen. |
| `same-day-service` | You will be seen or served the same day you arrive, not scheduled out. |

**Cultural and identity affirmation** — will I be safe and respected here?

| Slug | Meaning |
|---|---|
| `women-only` | Space or service restricted to women (survivor services, certain shelters, certain clinics). Signals safety for people for whom co-ed spaces are a barrier. |
| `lgbtq-affirming` | Explicit affirming posture: staff trained, intake forms inclusive — not merely "tolerant." |
| `trans-affirming` | Explicit affirming posture toward trans, non-binary, and gender-nonconforming people. Correct name/pronoun use, no gatekeeping on services. |
| `indigenous-led` | Service led by and for Indigenous community; culturally grounded practice, not merely culturally-aware. |
| `peer-led` | Service delivered by people with lived experience of the issue (substance use, housing instability, mental health, survivorship). |
| `survivor-centered` | Designed with and for survivors of domestic violence, sexual violence, or trafficking. |
| `secular` | No religious component, requirement, or expectation. Service doesn't require profession of faith, attendance at worship, or exposure to proselytising. |

**Accessibility** — can I physically / sensorily use this?

| Slug | Meaning |
|---|---|
| `disability-accessible` | ADA-compliant physical access; accommodations available on request. |
| `asl-available` | American Sign Language interpretation available on-site, on-call, or scheduled. |
| `sensory-friendly` | Environment or specific hours designed for sensory-processing needs (autism, PTSD, anxiety) — reduced noise, soft lighting, predictable routines. |
| `language-accessible` | Interpreters available or multilingual staff routinely on duty. Name specific languages in `body_raw` where known. |

**Substance use** — will I be judged or reported for what I'm using?

| Slug | Meaning |
|---|---|
| `harm-reduction` | Non-judgmental service delivery regardless of substance use; does not require abstinence, does not call law enforcement over use. |

**Minors** — can I be here without a parent?

| Slug | Meaning |
|---|---|
| `minors-without-parent` | Minors can access the service without parental presence, notification, or consent (where Minnesota law permits — certain reproductive-health, mental-health, and substance-use services). |

**Law enforcement and reporting** — will calling this bring the police?

| Slug | Meaning |
|---|---|
| `no-law-enforcement` | The service does not involve law-enforcement presence, will not call police as policy (outside of immediate life-threat situations), and will not share information with police without consent. |

**Family logistics** — can I bring what I need to bring?

| Slug | Meaning |
|---|---|
| `childcare-provided` | Free on-site childcare available while the parent or guardian receives the service (for clinics, trainings, shelter intake). |
| `pets-welcome` | Pets or service animals permitted — especially meaningful for shelters, recovery housing, and drop-in services where "I can't leave my dog" is a common barrier. |
