---
title: Emergency Resource Aggregator - MVP Execution Plan
type: feat
date: 2026-01-27
status: active
priority: critical
approach: spike-based
timeline: 3-5 days to shippable MVP
---

# MVP Execution Plan - Spike-Based Approach

## Overview

Build the core product in 3 focused spikes, in order of priority. Each spike ships value independently.

**Philosophy**: Ship working features incrementally. SPIKE 1 alone is a useful product.

---

## SPIKE 1: Need Discovery Pipeline + Display (CRITICAL PATH)

**Goal**: Scrape websites → Extract needs → Sync to database → Display in app

**Why First**: This is the core value prop. Without needs, there's nothing to match volunteers to.

### What We're Building

**Backend**:
1. **Website Scraper** (Firecrawl)
   - Scrape organization websites
   - Extract clean content

2. **AI Need Extraction** (rig.rs + GPT-4o)
   - Parse scraped content
   - Extract structured needs (title, description, urgency, contact)
   - **AI-extracted needs start as `pending_approval`** (never auto-publish!)

3. **👤 Human-in-the-Loop Approval** (CRITICAL)
   - Admin reviews all AI-extracted needs before they go live
   - Can approve, reject, or edit before approval
   - Provides quality control and prevents AI hallucinations

4. **Sync Mechanism**
   - Content hash for deduplication
   - Detect new, changed, removed needs
   - Store in PostgreSQL

5. **GraphQL API**
   - Query needs (paginated, filtered by status)
   - Admin mutations (approve, reject, edit)

**Admin UI (React SPA)**:
5. **Need Approval Queue** (CRITICAL - Human-in-the-Loop)
   - **List all `pending_approval` needs** in table/cards
   - Show: Organization name, title, TLDR, source URL
   - **Quick actions per need**:
     - ✅ Approve → Moves to "active", visible in app
     - ✏️ Edit → Opens editor, then approve
     - ❌ Reject → Moves to "rejected", hidden forever
   - **Preview mode**: Click need → See full details before decision
   - **Batch operations**: Select multiple → Approve/reject all
   - **Filter/sort**: By organization, urgency, date scraped

6. **Need Editor** (For fixing AI mistakes)
   - Edit title, TLDR, description, markdown, contact info
   - Live markdown preview
   - Save → Immediately approves and activates
   - Cancel → Back to queue (stays pending)

7. **Organization Source Management**
   - List scraped sources
   - Trigger manual scrape button
   - View scrape history (X needs extracted, Y approved)
   - Add new source manually

**Frontend (Expo)**:
7. **Need List Screen**
   - Display ONLY `active` (approved) needs
   - Clean, readable format
   - Markdown rendering support

8. **Need Detail Screen**
   - TLDR (1-2 sentences)
   - Expanded description (full markdown)
   - Contact info (phone, email, website)
   - "I'm interested" button (placeholder for now)

### Database Schema

```sql
-- Organization sources (websites we monitor)
CREATE TABLE organization_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_name TEXT NOT NULL,
    source_url TEXT NOT NULL UNIQUE,
    last_scraped_at TIMESTAMPTZ,
    scrape_frequency_hours INTEGER DEFAULT 24,
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Organization needs
CREATE TABLE organization_needs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_name TEXT NOT NULL,

    -- Content
    title TEXT NOT NULL,
    description TEXT NOT NULL,  -- Plain text for search
    description_markdown TEXT,  -- Markdown for display
    tldr TEXT,  -- Short summary (1-2 sentences)

    -- Contact
    contact_info JSONB,  -- { phone, email, website }

    -- Metadata
    urgency TEXT,
    status TEXT DEFAULT 'pending_approval',
    content_hash TEXT,  -- SHA256 for deduplication

    -- Sync tracking
    source_id UUID REFERENCES organization_sources(id),
    last_seen_at TIMESTAMPTZ DEFAULT NOW(),
    disappeared_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_needs_status ON organization_needs(status);
CREATE INDEX idx_needs_content_hash ON organization_needs(content_hash);
```

### GraphQL Schema

```graphql
type Need {
  id: ID!
  organizationName: String!
  title: String!
  tldr: String!
  description: String!
  descriptionMarkdown: String
  contactInfo: ContactInfo!
  urgency: String
  status: String!
  createdAt: DateTime!
}

type ContactInfo {
  phone: String
  email: String
  website: String
}

type Query {
  # Public - list active needs
  needs(
    status: String = "active",
    limit: Int = 50,
    cursor: ID
  ): NeedConnection!

  # Public - get single need
  need(id: ID!): Need
}

type Mutation {
  # Admin - trigger scrape for an organization
  scrapeOrganization(sourceId: ID!): ScrapeResult!

  # Admin - approve extracted need (moves to "active" status)
  approveNeed(needId: ID!): Need!

  # Admin - edit then approve (fix AI mistakes)
  editAndApproveNeed(needId: ID!, input: EditNeedInput!): Need!

  # Admin - reject extracted need (moves to "rejected" status)
  rejectNeed(needId: ID!, reason: String!): Boolean!
}

input EditNeedInput {
  title: String
  description: String
  descriptionMarkdown: String
  tldr: String
  contactInfo: ContactInfoInput
  urgency: String
}

input ContactInfoInput {
  phone: String
  email: String
  website: String
}
```

### Human-in-the-Loop Workflow

```
1. Scrape Website
   ↓
2. AI Extracts Needs
   ↓
3. Save as "pending_approval"  ← AI NEVER auto-publishes
   ↓
4. 👤 Admin Reviews in Queue
   ├─ Approve → Status: "active" → Visible in Expo app
   ├─ Edit + Approve → Fix errors → Status: "active"
   └─ Reject → Status: "rejected" → Never shown
```

**Why Human-in-the-Loop?**
- ✅ Prevents AI hallucinations (made-up needs)
- ✅ Catches extraction errors (wrong contact info)
- ✅ Ensures quality control before volunteers see it
- ✅ Allows admins to add context/formatting

### Success Criteria

- [ ] Can scrape 5 test organization websites
- [ ] AI extracts needs with good quality (>70% precision)
- [ ] **All AI-extracted needs start as `pending_approval`**
- [ ] **Admin UI shows approval queue clearly**
- [ ] **Admin can approve, edit+approve, or reject**
- [ ] **Only approved needs appear in Expo app**
- [ ] Content hash detects duplicates correctly
- [ ] Needs sync properly (new, unchanged, disappeared)
- [ ] Expo app displays approved needs beautifully
- [ ] Tapping need shows detail with TLDR + full description + contact
- [ ] Markdown renders correctly in app

### Time Estimate

**2 days**

---

## SPIKE 2: Simple Volunteer Intake Form (ESSENTIAL)

**Goal**: Basic form to register volunteers with expo push token

**Why Second**: We need volunteers in the system to test matching. Keep it simple - no AI, no chat.

### What We're Building

**Backend**:
1. **Volunteer Registration API**
   - Simple GraphQL mutation
   - Store: expo_push_token + capabilities (checkboxes) + text description

2. **Database Schema**

```sql
-- Volunteers (privacy-first, zero PII, text-first)
CREATE TABLE volunteers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Anonymous identifier
    expo_push_token TEXT UNIQUE NOT NULL,

    -- TEXT-FIRST: Everything goes into searchable text
    -- UI can suggest checkboxes, but they write to this field
    searchable_text TEXT NOT NULL,

    -- Example: "Can drive, Spanish speaker, legal aid volunteer.
    --           Available weekends and Wednesday evenings.
    --           Located in Minneapolis."

    -- Optional structured hints (parsed from text, not user input)
    availability TEXT,  -- Extracted: "Weekends", "Evenings", etc.
    location TEXT,      -- Extracted: "Minneapolis", "St Paul", etc.

    -- Status
    active BOOLEAN DEFAULT true,
    notification_count_this_week INTEGER DEFAULT 0,
    paused_until TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_volunteers_token ON volunteers(expo_push_token);
CREATE INDEX idx_volunteers_active ON volunteers(active) WHERE active = true;
```

**Frontend (Expo)**:
3. **Registration Screen** (TEXT-FIRST UI)
   - Welcome message
   - Expo push token collection (automatic)

   - **Quick suggestions** (checkboxes for convenience):
     ```
     What can you help with? (Select all that apply)
     ☐ I can drive
     ☐ I can lift heavy items
     ☐ I can cook/prepare food
     ☐ I can translate (Spanish)
     ☐ I can translate (Somali)
     ☐ I can provide legal aid
     ☐ I can provide medical aid
     ☐ I can tutor/teach
     ☐ I can donate supplies
     ```

   - Large text area: "Tell us more about how you'd like to help"
     - Pre-filled from checkboxes: "Can drive, Spanish speaker, legal aid..."
     - User can edit/expand freely

   - Text field: "When are you available?"
   - Text field: "Where are you located?"

   - **On submit**: Combine all text into `searchable_text`:
     ```
     "Can drive, Spanish speaker, legal aid volunteer. Available weekends
     and Wednesday evenings. Located in Minneapolis. Also interested in
     tutoring and food distribution."
     ```

   - Submit button

### GraphQL Schema

```graphql
type Volunteer {
  id: ID!
  expoPushToken: String!
  searchableText: String!  # TEXT-FIRST: Everything in one field
  availability: String     # Optional parsed hint
  location: String         # Optional parsed hint
  active: Boolean!
  createdAt: DateTime!
}

input RegisterVolunteerInput {
  expoPushToken: String!
  searchableText: String!  # Combines all capabilities, skills, interests
  availability: String
  location: String
}

type Mutation {
  registerVolunteer(input: RegisterVolunteerInput!): Volunteer!
}
```

### Success Criteria

- [ ] Expo app collects push token automatically
- [ ] Registration form is simple and clear
- [ ] Checkboxes work, text fields work
- [ ] Submit → Creates volunteer in database
- [ ] Can view registered volunteers in database
- [ ] No PII collected (just push token + capabilities)

### Time Estimate

**1 day**

---

## SPIKE 3: AI Chat Intake (NICE-TO-HAVE)

**Goal**: Conversational AI for richer volunteer profiles

**Why Last**: This is a UX enhancement, not core functionality. We can ship without it.

**Scope**:
- Real-time chat UI
- Redis pub/sub for broadcasting
- GraphQL subscriptions
- rig.rs conversational AI
- Replaces simple form with chat interface

**Decision Point**: After SPIKE 2, evaluate if we even need this. The simple form might be good enough for MVP.

### Time Estimate

**2 days** (if we decide to build it)

---

## Execution Order

### Week 1

**Day 1-2**: SPIKE 1 (Need Discovery Pipeline)
- Backend: Scraper + AI extraction + sync
- Frontend: Need list + detail screens
- **Deliverable**: Can view real needs in app

**Day 3**: SPIKE 2 (Volunteer Intake)
- Backend: Registration API
- Frontend: Simple form with checkboxes
- **Deliverable**: Volunteers can register

**Day 4-5**: (TBD based on progress)
- Option A: Build SPIKE 3 (AI Chat) if time permits
- Option B: Build notification engine (matching + push)
- Option C: Polish SPIKE 1 + 2, add admin UI

### Decision Point

After SPIKE 2 is complete, we'll have:
- ✅ Needs being discovered and displayed
- ✅ Volunteers registering with capabilities

At that point, decide:
1. **Add AI chat?** (SPIKE 3)
2. **Build matching engine?** (connect volunteers to needs)
3. **Polish and ship?** (MVP is done)

---

## Success Definition

**Minimum Viable Product** (after SPIKE 1 + 2):
1. Organizations → Website scraped → Needs extracted → Admin approves
2. Needs appear in Expo app with TLDR + description + contact
3. Volunteers register with capabilities + availability
4. System has data to start building matching logic

**This is shippable** even without AI chat or automated notifications. Volunteers can browse needs manually and reach out directly.

---

## File Structure

```
src/domains/
├── organization/           # SPIKE 1
│   ├── commands/
│   │   ├── scrape.rs
│   │   ├── approve_need.rs      # 👤 Human approval
│   │   └── reject_need.rs       # 👤 Human rejection
│   ├── effects/
│   │   ├── scraper_effects.rs   # Firecrawl
│   │   ├── ai_effects.rs        # rig.rs extraction
│   │   └── sync_effects.rs      # Content hash comparison
│   ├── models/
│   │   ├── source.rs
│   │   └── need.rs
│   └── edges/
│       ├── query.rs
│       └── mutation.rs
│
└── volunteer/              # SPIKE 2
    ├── commands/
    │   └── register.rs
    ├── effects/
    │   └── db_effects.rs
    ├── models/
    │   └── volunteer.rs
    └── edges/
        └── mutation.rs

frontend/
├── admin-spa/              # SPIKE 1 - Admin UI
│   └── src/pages/
│       ├── NeedApprovalQueue.tsx   # 👤 Review pending needs
│       ├── NeedEditor.tsx          # 👤 Edit before approve
│       └── SourceManagement.tsx    # Trigger scrapes
│
└── expo-app/               # Public volunteer app
    └── src/screens/
        ├── NeedsListScreen.tsx     # Browse approved needs
        ├── NeedDetailScreen.tsx    # View full need
        └── RegisterScreen.tsx      # SPIKE 2
```

---

## Ready to Start?

Want to dive into **SPIKE 1 (Need Discovery Pipeline)** right now?

I can:
1. Create migrations for organization_sources + organization_needs
2. Build Firecrawl scraper client
3. Implement AI need extraction with rig.rs
4. Build content hash sync logic
5. Create GraphQL API
6. Build Expo screens for browsing needs

Let's build the scraper pipeline first.
