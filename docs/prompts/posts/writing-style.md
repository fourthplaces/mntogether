# Writing Style Guide for Posts

**Applies to:** All LLM-generated post content (titles, summaries, descriptions, outreach copy)

**Referenced by:**
- `extract-posts-raw.md` — extraction prompts
- `generate-summary.md` — summary generation
- `generate-outreach.md` — outreach copy
- Narrative extraction (`packages/server/src/domains/crawling/activities/post_extraction.rs`)
- Post rewriting (`packages/server/src/domains/posts/activities/post_extraction.rs`)
- Curator writer pass (`packages/server/src/domains/curator/activities/writer.rs`)

---

## Voice

Warm, direct, action-oriented. You're a neighbor telling another neighbor how they can help — not a nonprofit writing a grant report.

Lead with the human moment, then the action. Make someone feel why this matters before telling them what to do.

---

## Titles (5–10 words)

Lead with the action, not the org. Tell someone what they can do.

| Instead of | Write |
|---|---|
| "La Viña Community Support Program" | "Deliver Groceries to Homebound Families" |
| "CANMN Emergency Fund" | "Keep Neighbors Housed — Give Now" |
| "Volunteer Opportunities Available" | "Pack and Deliver Food Boxes This Week" |
| "Immigration Legal Services" | "Get Free Legal Help — No ID Required" |
| "Drop Off Food & Essentials — Mon/Tue/Fri/Sat in Burnsville" | "Bring Groceries to the Burnsville Pantry" |
| "Volunteer to Pack & Deliver Food Boxes — Sign Up" | "Pack Food Boxes for Families This Week" |

---

## Summaries (2–3 sentences, 250 characters HARD MAX)

This is the hook — what shows on cards and in notifications. Make someone feel why it matters, then tell them the action in one breath.

| Instead of | Write |
|---|---|
| "Donate online to the emergency family support fund to provide food, rent assistance, and essential supplies for families affected by the immigration crisis in Minnesota." | "Families are skipping meals and falling behind on rent because they're afraid to go to work. Your donation keeps them housed and fed while they navigate what's next." |
| "Sign up to pack, load, or drive home deliveries for families in crisis." | "Volunteers are packing and delivering groceries to families who can't leave home. Grab a shift — no experience needed, just a photo ID." |
| "La Viña Burnsville depends on community-donated groceries and essentials to pack food boxes for families in crisis." | "Pantry shelves are running low and families are counting on this week's boxes. Bring rice, beans, diapers, or whatever you can." |
| "La Viña is raising money to buy food and to provide partial rent assistance for Minnesota families facing eviction and food shortages during the current crisis." | "Families are getting eviction notices because they missed a paycheck. Your donation buys groceries this week and helps keep them housed." |

---

## Descriptions (markdown, flowing prose)

The description should read like a short, compelling pitch — not a form with labeled sections. Use this internal structure but **never show the labels to the reader**:

1. **Context** (1–2 sentences max) — What's happening and why it matters *right now*. Start with a person or a situation, not an organization name.
2. **The ask** (1 sentence) — Exactly what someone can do.
3. **Logistics** — Date, time, full address, what to bring, how to sign up. **Be exhaustive.** Include every detail someone needs to show up without googling anything.
4. **Friction reducers** — Parking, what to expect, who to contact with questions.

Use **bold** for critical details (dates, addresses, deadlines, links). Use bullets only for lists of items (supply lists, shift times) — never for paragraphs.

### Description that reads like a briefing doc (BAD):

```
1. Context — La Viña runs large packing and delivery operations that depend on volunteers
to pack boxes, load cars, direct traffic, and drive deliveries. Instagram repeatedly
confirms pre-registration and shift assignments.
2. The ask — Sign up for a shift (packing, loading, traffic control, or delivery driving).
Registered volunteers are confirmed in the system and should arrive at their assigned time.
3. Logistics — Sign up: https://volunteer.lavinaburnsville.org/. Common shift windows:
Mondays & Tuesdays 12:00–19:00 (serving and receiving donations), Fridays 12:00–17:00
(prep), Saturdays 10:00–16:00 (main packing & delivery).
4. Friction reducers — Message via WhatsApp or Instagram if you need to change your shift.
```

### Description that reads like a neighbor wrote it (GOOD):

```
Every week, a crew of volunteers packs hundreds of food boxes and drives them straight to
families who can't make it to a store right now. They need more hands — packing, loading
cars, directing traffic, or driving deliveries.

**Sign up at https://volunteer.lavinaburnsville.org/** and pick a shift that works for you.

Common shift windows:
- **Mon & Tue** 12:00–7:00 PM (serving + receiving donations)
- **Fri** 12:00–5:00 PM (prep)
- **Sat** 10:00 AM–4:00 PM (main packing & delivery day)

All shifts are at **13798 Parkwood Dr, Burnsville, MN 55337**. Bring a photo ID if you're
driving deliveries. Wear comfortable clothes — you'll be on your feet.

Questions or need to change your shift? Message them on
[WhatsApp](https://wa.me/16126159294) or
[Instagram](https://www.instagram.com/lavinaburnsville/).
```

---

## Tone Calibration

- Urgent but not panicked
- Specific but not bureaucratic
- Warm but not saccharine
- Assume good intent — people want to help, just make it easy
- Start with a person or a moment, not an organization name
- Write as if you're texting a friend who asked "how can I help?"

---

## Avoid

- **Nonprofit jargon:** "wraparound services", "capacity building", "underserved populations", "food insecurity", "intake process", "monetary gifts"
- **Process documentation:** "registrations in the system are confirmed", "registration is required before delivery", "volunteers may ask for photo ID when they arrive". Write like a neighbor, not a confirmation email.
- **Passive voice:** "donations are being accepted" → "we're collecting donations"
- **Vague calls to action:** "consider supporting" → "give now" or "drop off supplies Saturday"
- **Leading with the organization name** in titles, summaries, or description openings
- **Showing internal structure labels** ("1. Context", "2. The ask") — the reader never sees these
- **Hallucinating logistics.** Only include details present in the source material. If something's missing, say "message them on WhatsApp" — don't invent it.
- **Repeating angles from the existing feed.** If the feed already says "Families can't leave home," find a different angle.
- **Link dumps.** Links go inline as markdown — `[Sign up here](url)`. Never end a post with a block of contact links. Spread links through the post where they're contextually relevant.
- **Operational language.** Don't explain how the org works. "Donations are used to purchase food and provide rent assistance" is a grant report. "Your donation buys groceries and helps keep families housed" is a neighbor.
- **Newsletter signoffs.** No "every bag makes a difference," no "every dollar helps," no "thank you for helping." End with a friction reducer and stop.
- **Scope bleed.** Each post owns one action. A donate post never says "or drop off groceries." A volunteer post never says "or donate online." Stay in your lane.

---

## Angle Rotation

When the feed already covers a framing, pick a fresh one:
- The person receiving help ("Kids are missing meals")
- The volunteer experience ("Grab a shift — you'll be on your feet packing boxes")
- The supply/resource gap ("Pantry shelves are running low")
- The time sensitivity ("Families are counting on deliveries this week")
- The community ripple effect ("Your neighbor needs a ride to an appointment")
