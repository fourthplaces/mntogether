# Emergency Resource Aggregator: Problem + Solution

## The Problem

**During the current crisis in Minnesota, critical volunteer opportunities are being missed.**

### What's Happening
- **Organizations have specific needs** - "We need Spanish-speaking intake volunteers" or "We need drivers to deliver food"
- **Volunteers want to help** - A bilingual lawyer exists, ready to volunteer
- **They never connect** - The lawyer never hears that the immigrant center needs them

### Why Current Approaches Fail
1. **Broadcasting is inefficient** - Organizations post generic "help wanted" messages to Facebook/email lists
2. **Volunteers get overwhelmed** - People see 100 generic posts, tune out, miss the one that matches their skills
3. **Timing is critical** - By the time someone sees a post and reaches out, the need is already filled
4. **Manual coordination doesn't scale** - Matching people to needs requires someone to know everyone's skills and availability

### The Cost
- Organizations waste time broadcasting instead of serving
- Skilled volunteers never hear about needs they could fill
- Vulnerable populations wait longer for help
- Helpers experience burnout from information overload

---

## The Solution

**A relevance notifier that surfaces opportunities volunteers might care about—without coordination overhead.**

### Core Philosophy: We Don't Match, We Surface

**What we're building:** A system that says "thought you might be interested" not "you are the perfect match"

**Key insight:**
- **Cost of false positive:** 2 seconds of attention, one ignored notification
- **Cost of false negative:** Someone never hears about a need they could have helped with

→ **We bias toward recall. Let humans self-select.**

**This means:**
- ✅ We show opportunities that might be relevant
- ✅ Volunteers decide if they actually want to help
- ✅ We don't try to predict outcomes
- ✅ We optimize for "making sure people hear about it" not "perfect matching"

### How It Works

```
CSV Import → AI Suggestion → Admin Approval → Vector Matching → Push Notification → Direct Contact
```

**Step 1: Import Resources (CSV → Database)**
- Admins upload CSV files (exported from Excel, Google Sheets, etc.)
- Generic column mapper: "Which column is org name? Website? Contact?"
- System imports organizations into database
- **Admins oversee imports and approve extracted needs to prevent noise or misuse**

**Step 2: AI Suggests Needs (Scraping → Structured Data)**
- System scrapes each organization's website
- AI (rig.rs + OpenAI) **suggests** potential needs based on public language:
  - "Spanish-speaking intake volunteers" ✓
  - "Drivers for food delivery" ✓
  - "Translators for legal documents" ✓
- **Admins review and approve suggestions** before they become active
- Approved needs get "Last Verified" timestamps and badges
- Creates searchable vector embeddings for each approved need

**Step 3: Volunteers Register (Mobile App)**
- Person opens Expo app, taps "I can help"
- Enters free-form text: "I'm a bilingual lawyer with immigration experience. Available weekends. Based in Minneapolis."
- System stores as **searchable text** (no rigid fields)
- Creates embedding from text
- Registers push notification token
- **No accounts required, but submissions are rate-limited and moderated**

**Why text-first?**
- Anti-fragile: Can re-embed with better models later
- Evolvable: Add structure gradually as we learn what matters
- Zero migration cost when AI improves
- Humans write what's actually relevant, not what fields ask for

**Step 4: Relevance Notification (Simple & Generous)**
```sql
"Who might want to know about Spanish-speaking intake volunteers?"
  1. Vector search: Find top 20 semantically similar volunteers
  2. AI quick check: "Is this relevant to them?" (generous threshold)
  3. Apply simple limits: Max 3 notifications per volunteer per week
  4. Send to top 5 relevant volunteers
```

**Why Simple?**
- We're not trying to be perfect, just helpful
- Better to over-notify than under-notify
- Volunteers will self-select ("not interested" vs. "let me reach out")
- False positive cost: 2 seconds to dismiss
- False negative cost: Someone never hears about it

**Step 5: Push Notification (Invitational Tone)**
- Top 5 relevant volunteers get notification
- **Example message:**

  > "Thought you might be interested:
  >
  > Church of Hope is looking for Spanish-speaking intake volunteers.
  >
  > We thought of you because: [Your profile mentions legal aid and Spanish fluency]
  >
  > No pressure—just wanted to make sure you knew about it."

- Volunteer taps notification if interested

**Step 6: Contact Reveal → Direct Outreach**
- App shows need details + organization contact info
- Phone: 555-1234
- Email: intake@churchofhope.org
- Last Verified: 2 days ago ✓
- Volunteer reaches out directly
- **No intermediary** - platform just facilitates the connection

---

## What This Platform Does (and Doesn't Do)

### ✅ What It Does
- Discovers help opportunities automatically from existing sources
- Matches skills to needs using semantic understanding
- Sends targeted notifications only when there's a real match
- Facilitates fast, direct contact between volunteers and organizations

### ❌ What It Doesn't Do
- **Does not manage volunteers** - no scheduling, shift management, or logistics
- **Does not coordinate activities** - organizations handle their own operations
- **Does not replace human judgment** - admins vet all extracted needs
- **Does not guarantee outcomes** - we create introductions, not commitments

**Core Principle:** This platform discovers and connects. Everything else is left to the humans involved.

---

## Why This Works

### For Organizations
- ✅ Stop broadcasting to everyone, start reaching the right people
- ✅ Needs are discovered automatically from your website (with admin verification)
- ✅ No account needed - just submit your website URL
- ✅ Get calls from pre-qualified volunteers with relevant skills

### For Volunteers
- ✅ Hear about opportunities you might care about (not spam, not broadcast)
- ✅ Simple "yes/no" decision - no pressure, no commitment
- ✅ **Complete anonymity** - no login required (your biggest competitive advantage)
- ✅ Control notification frequency (max 3 per week default)
- ✅ You decide if it's a real match - we just surface it

### For the Community
- ✅ Faster response times (minutes instead of days)
- ✅ Higher match quality (right skills, right need)
- ✅ Scalable (handles 10 orgs or 1000 orgs)
- ✅ Safe (rate-limited, moderated, admin-reviewed)

---

## Addressing Key Risks

### Risk 1: Stale Website Data
- **Mitigation**: "Last Verified" badges on all needs
- Admins can manually verify needs via phone call
- Auto-flag needs older than 30 days for re-verification
- Organizations can update needs via simple web form

### Risk 2: Safety and Trust
- **Volunteer Side**: Rate-limited submissions, spam detection, moderation queue
- **Organization Side**: Self-attestation field gives orgs "caller ID" before contact
- **Platform**: All matches logged, abuse reports handled by admins

### Risk 3: Notification Relevance
- **Generous threshold**: Better to show than hide (volunteers self-select)
- **Simple limits**: 3 notifications per volunteer per week (prevents spam)
- **AI reasoning**: Each notification explains "why we thought of you"
- **Feedback loop**: Track which notifications get responses (improves prompts over time)

---

## MVP Scope

**What We're Building First:**

1. **Generic CSV Importer** - Works with any Excel export, column mapper
2. **AI Need Suggestion** - Scrape websites, suggest needs (admin approval required)
3. **Volunteer Registration** - Free-form text profiles, no rigid fields (anti-fragile)
4. **Relevance Notification** - Vector search (top 20) → AI check → Notify top 5
5. **Push Notifications** - Invitational tone: "thought you might be interested"
6. **Contact Reveal** - Tap notification → see org contact info + verification status
7. **Admin Panel** - Review queue, CSV upload, need approval, moderation

**What We're NOT Building (Yet):**
- ❌ Perfect matching algorithms
- ❌ Multi-stage filtering
- ❌ Confidence scores (fake precision)
- ❌ Complex notification policies
- ❌ In-app messaging
- ❌ Outcome tracking beyond "did they click?"

**Two Apps:**
- 📱 **Expo App** (public) - iOS, Android, Web - volunteers submit offers, view matches
- 🖥️ **Admin SPA** (private) - React web app - admins import CSV, review/approve needs

**Tech Stack:**
- Backend: Rust + seesaw-rs (event-driven) + GraphQL (Juniper)
- Database: PostgreSQL + pgvector (vector similarity)
- AI: rig.rs with OpenAI (gpt-4o for extraction, text-embedding-3-small for matching)
- Frontend: Expo (mobile/web) + React (admin) + Apollo Client

---

## The Outcome

**Before:**
- Organization posts "We need volunteers" → 5 people see it → 0 are bilingual lawyers → Need goes unfilled

**After:**
- Organization's website says "Spanish-speaking legal aid needed" → AI suggests need → Admin approves → Matches bilingual lawyer (with credentials) → Push notification sent → Lawyer contacts org within 30 minutes → Connection made

**The difference:** Right person, right need, right time—without coordination overhead.

---

## Why This Approach is Robust Long-Term

### 1. Storage is Anti-Fragile
Everything is stored as **searchable text** → You can:
- ✅ Re-embed with better models later (no data migration)
- ✅ Change prompts without schema changes
- ✅ Add structure gradually as you learn what matters
- ✅ Backfill with zero data loss

**Example evolution:**
```
Today:     "I'm a bilingual lawyer" → embedding → notify
Tomorrow:  Same text → better embedding model → notify
Later:     Same text → reranker → better prompts → notify
```

No ingestion changes needed. Text is the source of truth.

### 2. Intelligence Layer is Replaceable
The matching logic can evolve without touching data:

```
MVP:       vector search → AI relevance check → notify
V2:        vector search → reranker → AI check → notify
V3:        vector search → reranker → policy layer → notify
```

Same data, better intelligence over time.

### 3. Humans Close the Loop
```
System: "Thought you might be interested in this"
Human:  "Yes, I'll reach out" or "No thanks"
```

Real feedback → You observe outcomes → You improve prompts

This is a **learning system**, not a fixed algorithm.

### 4. New Concepts Fit Naturally
Want to add later?
- ✅ Urgency weighting → Change notification prompt
- ✅ Volunteer responsiveness → Add to searchable text
- ✅ Cooldown periods → Update notification limits
- ✅ Crisis mode → Change top_k parameter

All without schema changes. Text-first architecture pays dividends.

---

## Key Metrics to Watch

### Leading Indicators (What We Control):
- Notifications sent per need
- Average time from scrape to notification
- Volunteer notification frequency (staying under 3/week?)

### Lagging Indicators (What Matters):
- Do volunteers click/respond? (if tracking)
- Do organizations report getting help?
- Volunteer retention (still active after 4 weeks?)

### What NOT to Measure (Yet):
- "Match accuracy" (undefined in our model)
- "Precision/recall" (no ground truth)
- "Satisfaction scores" (premature)

**We optimize for awareness, not outcomes.** The outcome is the human decision.

---

## Next Technical Questions

Based on Gemini's feedback, we should answer:

1. **SQL Schema Design**: How should `organization_need` and `volunteer_offer` tables support both pgvector embeddings and keyword indexes for hybrid search?

2. **System Prompt Design**: What's the exact prompt we send to OpenAI for need extraction? How do we instruct it to identify actionable, specific needs vs. vague "help us" statements?

3. **Hybrid Search Implementation**: Should we use PostgreSQL full-text search (tsvector) for keywords or a simpler LIKE/regex approach? What's the performance trade-off?

4. **Admin Approval Workflow**: Should need approval be binary (approve/reject) or should admins be able to edit the extracted need before publishing?

Let me know which of these you'd like me to tackle next.
