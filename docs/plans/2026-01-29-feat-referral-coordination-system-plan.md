---
title: Referral Coordination System for Healthcare Workers
type: feat
date: 2026-01-29
status: planning
revision: 6 (dynamic multi-language + conversational AI + LiveKit)
---

# Referral Coordination System for Healthcare Workers

## MVP Core: Direct Impact Definition

**Direct Impact** =

> A healthcare worker can, in one sitting, produce a safe, language-appropriate referral that allows a scared patient to access care they would otherwise avoid.

**If the MVP does not reliably do that, nothing else matters.**

### The One-Sentence Cut Line

If a Somali-speaking parent who is afraid to leave home can use a document from their provider to get care or medication they would otherwise skip, the MVP succeeded.

Everything else is iteration.

---

## The 20% That Creates 80% of Impact

### 1. Three Entry Points (THE SPINE)

**Homepage Design** - Three clear paths for three user types:

```
┌─────────────────────────────────────────────────────────────┐
│  MN Digital Aid                                             │
│                                                             │
│  ┌─────────────────────────┐  ┌─────────────────────────┐  │
│  │                         │  │                         │  │
│  │    🆘 I NEED HELP       │  │    🤝 I WANT TO HELP    │  │
│  │                         │  │                         │  │
│  │  Find food, shelter,    │  │  Volunteer, donate,     │  │
│  │  healthcare, legal aid  │  │  support businesses     │  │
│  │                         │  │                         │  │
│  └─────────────────────────┘  └─────────────────────────┘  │
│                                                             │
│  🏥 Healthcare workers: [Create referral for patient →]    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Path A: "I Need Help"** (Get Services)
- Browse services directory (food, housing, healthcare, legal)
- Filter by fear constraints ("No ID required", "Won't contact authorities")
- Filter by delivery mode (in-person, phone, online, home visit)
- Click listing → see details, contact organization
- **Use case**: Person in crisis finding help themselves

**Path B: "I Want to Help"** (Give Back)
- Browse opportunities to help (volunteer, donate, support businesses)
- Filter by type: volunteer opportunities, donation needs, businesses needing customers
- See immigrant-owned businesses in crisis that need support
- Click listing → see how to help (donate link, volunteer signup, visit business)
- **Use case**: Community member wanting to give back

**Path C: "Create Referral"** (Healthcare Workers)
- **Big button**: "📝 Create Personalized List" (no login required)
- Anyone (healthcare worker, social worker, volunteer, friend) chats with AI about someone's needs
- AI generates **personalized** document
- Person edits and gets **private link** to share
- **NOT browsable** - these are 1:1 private documents, not public directory
- **Use case**: Helper wants to curate/personalize for someone specific

**Key Distinctions**:
- Paths A + B = **Public Directory** (everyone sees same listings, browsable/searchable)
- Path C = **Private Documents** (1:1 personalized, not indexed/browsable)

### 1a. Healthcare Worker → Conversational AI → Editable Document

This is the spine. Must work flawlessly:
- **Chat interface** (left): Healthcare worker describes patient in natural language
- **Document preview** (right): AI updates document in real-time as conversation progresses
- AI has **tools** to search listings, organizations, contacts
- AI surfaces suggestions: listings, who to talk to, whatever's helpful
- Healthcare worker can **edit document directly** while chatting
- When satisfied, finalize and share link
- Patient can read document in their language

**UX**: Like vibe coding (preview document with chat window next to it)

**If this flow breaks, the product breaks.**

### 2. Constraint Filtering for Fear (Not General Safety)

Fear-specific safety only. Absolute MVP constraints:
- `requires_id` (undocumented patients)
- `contacts_authorities` / `mandatory_reporting` (hiding from enforcement)
- `avoids_facility_visit` (afraid to leave home)

**That's it.** No perfect eligibility modeling, no exhaustive restriction enums, no fancy ranking.

If these three constraints work perfectly, the system already beats Google, 211, and internal hospital lists.

### 3. Telehealth / Home / Medication Delivery (New Impact Vector)

MVP scope is ruthless:

**Include**:
- `service_delivery_mode`
- `avoids_facility_visit`

**Explicitly do NOT solve**:
- All telehealth friction flags
- Full medication supply chain modeling

Just make it possible to say: "Show me care that does not require leaving home."

That alone unlocks huge value.

### 4. Multi-Language Output (Patient-Facing, Not Polish)

This is not polish. This is impact.

**MVP includes**:
- English + Somali only (one high-need language)
- Auto-translation
- Clear indicator if machine-translated

**Do NOT need**:
- All five languages on day one
- Perfect translation review flows
- Community translation

**One language that works > five that are half-baked.**

### 5. Zero-Match Response (Safety Over Helpfulness)

Quietly one of the strongest design decisions.

MVP must:
- Refuse unsafe matches
- Explain why
- Offer next-best human steps (hotline, expand radius)

This builds trust even when the system can't help.

---

## The 20% to Intentionally Under-Polish

These are important but NOT MVP-critical for direct impact.

### Translation Review UI
- Admin review can be crude
- Even "approve all" is acceptable short-term
- Accuracy risk < no language access

### Organization Self-Service Portal
- Admin-managed listings are fine for MVP
- Org UX polish doesn't affect patient access immediately
- Don't block launch on org adoption

### Verification Sophistication
- Keep VERIFIED vs UNVERIFIED
- Skip fancy decay dashboards
- Manual "last checked" date is enough

### Analytics
- Views count only
- No dashboards
- No success metrics beyond qualitative feedback

### The AI
**Do NOT need**:
- Perfect parsing
- Elegant reasoning text
- Long explanations

**DO need**:
- Correct exclusion
- Conservative defaults
- Fast drafts

**A blunt but safe AI beats a smooth but risky one.**

---

## Overview

Expand the existing volunteer matching platform to help **healthcare workers make informed, contextual referrals** for patients with needs outside their scope of practice.

**NOT a marketplace.** This is a **vetted directory** that:
- Surfaces available services with current status
- Helps workers generate referrals in patient's language
- **Does not decide** who gets served or guarantee availability
- **Does not track outcomes** or manage cases

### The Four User Groups

1. **Healthcare workers** → Generate informed referrals using AI-assisted search + create documents for patients
2. **People seeking help** → Browse services directly OR view shared links (no account required, multi-language, self-service search)
3. **Organizations** → Manage service listings + communicate capacity + receive contact from people
4. **Volunteers/donors** → Discover opportunities to help (existing feature, expanded)

**Core Problem** (From Sister's Experience):

> "Even healthcare workers who speak English can't figure out how patients get care when they're afraid to go to facilities. Legal immigrants are avoiding hospitals, not picking up medications. People are suffering because they can't navigate this."

**Critical Gaps**:
- How do patients get doctor care at home or virtually?
- How do they get medications delivered?
- How do they find pediatricians who do home/virtual visits?
- **Even healthcare workers can't figure this out**

**Solution**: A multi-language vetted directory that helps healthcare workers find services that:
- Don't require facility visits (telehealth, home visits, delivery)
- Support patient's language (Somali, Spanish, Oromo, Amharic)
- Are safe for patients avoiding hospitals due to fear

---

## System Boundaries (Simple)

**This is a directory, not case management.**

We will NOT:
- Track outcomes (where patients went, what happened)
- Store patient data (workers describe needs generically, no PII)
- Make decisions (only surface options)
- Guarantee accuracy (info can get stale, workers/patients verify)

---

## Key Features

### 1. **Dynamic Multi-Language System** (CRITICAL - MVP: English + Spanish + Somali)
- **MVP Languages**: English, Spanish, Somali
- **Key Feature**: Adding a language is as simple as telling the system "add Dutch"
  - System automatically translates all existing listings
  - System automatically translates all future listings
- Referral documents generated in patient's language
- Services tagged by languages they support
- **Why**: Most patients don't speak English. System is useless if English-only.
- **Why these 3**: Spanish (largest non-English), Somali (highest fear barriers), English (default)

**Adding More Languages** (post-MVP, but system ready):
- Oromo, Amharic, Dutch, Arabic, etc.
- Just tell system: "add [language]"
- System handles batch translation + future auto-translation

#### Translation Pipeline Architecture (Dynamic & Automated)

**Automatic Translation System**: When a listing is submitted, it's automatically translated to ALL active languages using OpenAI GPT-4o.

**Two-Layer Translation**:
1. **UI Layer** (react-i18next): Navigation, buttons, labels, static content
2. **Content Layer** (Database): Listing titles, descriptions, eligibility criteria

**Dynamic Language Addition** (First-Class Feature):
```
User tells AI: "Add Dutch language"

System automatically:
1. Adds 'nl' to active_languages table
2. Creates /locales/nl/translation.json (UI strings)
3. Triggers batch job: translate all existing listings to Dutch
4. Future listings auto-translate to Dutch + all other active languages
```

**Active Languages Table**:
```sql
CREATE TABLE active_languages (
  language_code TEXT PRIMARY KEY,  -- ISO 639-1: 'en', 'es', 'so', 'nl', etc.
  language_name TEXT NOT NULL,     -- 'English', 'Spanish', 'Somali', 'Dutch'
  native_name TEXT NOT NULL,       -- 'English', 'Español', 'Soomaali', 'Nederlands'
  enabled BOOL DEFAULT true,
  added_at TIMESTAMPTZ DEFAULT NOW()
);

-- MVP: Start with 3 languages
INSERT INTO active_languages VALUES
  ('en', 'English', 'English', true, NOW()),
  ('es', 'Spanish', 'Español', true, NOW()),
  ('so', 'Somali', 'Soomaali', true, NOW());
```

**Database Schema for Translations**:

```sql
-- Listings stored in source language
ALTER TABLE listings
  ADD COLUMN source_language TEXT NOT NULL DEFAULT 'en';  -- ISO 639-1: 'en', 'es', 'so', 'om', 'am'

-- Translations cached in database for instant access
CREATE TABLE listing_translations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
  language TEXT NOT NULL CHECK (language IN ('so')),  -- MVP: Somali only. Post-MVP: 'es', 'om', 'am'

  -- Translated fields
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  tldr TEXT,
  hours_of_operation TEXT,
  eligibility_criteria TEXT,

  -- Translation metadata
  translated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  translation_method TEXT NOT NULL DEFAULT 'openai',  -- 'openai', 'human', 'community'
  needs_review BOOLEAN NOT NULL DEFAULT true,  -- Flag for admin review (can be crude)
  reviewed_at TIMESTAMPTZ,
  reviewed_by_member_id UUID REFERENCES members(id),

  -- Ensure one translation per language per listing
  UNIQUE(listing_id, language),

  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX idx_listing_translations_listing_id ON listing_translations(listing_id);
CREATE INDEX idx_listing_translations_language ON listing_translations(language);
CREATE INDEX idx_listing_translations_needs_review ON listing_translations(needs_review)
  WHERE needs_review = true;

COMMENT ON TABLE listing_translations IS 'Cached translations of listing content in multiple languages';
COMMENT ON COLUMN listing_translations.needs_review IS 'Auto-translated content flagged for human review';
```

**Translation Effect (Rust - Generic for Any Language)**:

```rust
// Effect: TranslateListingEffect
// When: Listing created, updated, or new language added
// What: Translates listing to all active languages (except source)

async fn execute(cmd: TranslateListingCommand, ctx: EffectContext) -> Result<Event> {
    let listing = Listing::find_by_id(cmd.listing_id, &ctx.db).await?;

    // Get all active languages (except source language)
    let active_languages = ActiveLanguage::find_all_enabled(&ctx.db).await?;
    let target_languages: Vec<String> = active_languages
        .into_iter()
        .map(|lang| lang.language_code)
        .filter(|code| code != &listing.source_language)
        .collect();

    // Translate to all target languages in parallel
    let translation_tasks: Vec<_> = target_languages
        .iter()
        .map(|lang| translate_to_language(&listing, lang, &ctx.ai_client))
        .collect();

    let translations = futures::join_all(translation_tasks).await;

    // Store successful translations
    for translation_result in translations {
        if let Ok(translation) = translation_result {
            ListingTranslation::upsert(translation, &ctx.db).await?;
        }
    }

    Ok(OrganizationEvent::ListingTranslated {
        listing_id: cmd.listing_id,
        languages: target_languages,
    })
}

// Helper: Generic translation to any language
async fn translate_to_language(
    listing: &Listing,
    target_language: &str,
    ai_client: &OpenAIClient,
) -> Result<ListingTranslation> {
    // Language name lookup for prompt
    let language_name = get_language_name(target_language)?;

    let prompt = format!(
        r#"Translate this service listing to {language_name}.

Keep contact info unchanged. Preserve formatting. Professional healthcare tone.

Title: {title}
Description: {description}
Hours: {hours}

JSON response:
{{
  "title": "...",
  "description": "...",
  "hours_of_operation": "..."
}}"#,
        language_name = language_name,
        title = listing.title,
        description = listing.description,
        hours = listing.hours_of_operation.as_deref().unwrap_or(""),
    );

    let response = ai_client.chat_completion(&prompt).await?;
    let translated: serde_json::Value = serde_json::from_str(&response)?;

    Ok(ListingTranslation {
        id: Uuid::new_v4(),
        listing_id: listing.id,
        language: target_language.to_string(),
        title: translated["title"].as_str().unwrap().to_string(),
        description: translated["description"].as_str().unwrap().to_string(),
        tldr: None,
        hours_of_operation: translated["hours_of_operation"].as_str().map(String::from),
        eligibility_criteria: None,
        translated_at: Utc::now(),
        translation_method: "openai".to_string(),
        needs_review: true,
        reviewed_at: None,
        reviewed_by_member_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

// Helper: Get language name from code (or query from active_languages table)
fn get_language_name(code: &str) -> Result<&'static str> {
    match code {
        "es" => Ok("Spanish"),
        "so" => Ok("Somali"),
        "om" => Ok("Oromo"),
        "am" => Ok("Amharic"),
        "nl" => Ok("Dutch"),
        "ar" => Ok("Arabic"),
        _ => Err(anyhow::anyhow!("Unsupported language: {}", code)),
    }
}
```

**Batch Translation Effect (When New Language Added)**:

```rust
// Effect: BatchTranslateAllListingsEffect
// When: New language added to active_languages
// What: Translates all existing listings to the new language

async fn execute(cmd: BatchTranslateCommand, ctx: EffectContext) -> Result<Event> {
    let target_language = cmd.language_code;

    // Get all active listings
    let all_listings = OrganizationNeed::find_active(&ctx.db).await?;

    // Translate each listing to new language
    for listing in all_listings {
        // Check if translation already exists (idempotent)
        let existing = ListingTranslation::find_by_listing_and_language(
            listing.id,
            &target_language,
            &ctx.db
        ).await?;

        if existing.is_none() {
            let translation = translate_to_language(&listing, &target_language, &ctx.ai_client).await?;
            ListingTranslation::upsert(translation, &ctx.db).await?;
        }
    }

    Ok(OrganizationEvent::BatchTranslationCompleted {
        language: target_language,
        listings_translated: all_listings.len(),
    })
}
```

**GraphQL Mutation for Adding Language**:

```graphql
mutation AddLanguage($code: String!, $name: String!, $nativeName: String!) {
  addLanguage(
    languageCode: $code,
    languageName: $name,
    nativeName: $nativeName
  ) {
    languageCode
    languageName
    nativeName
    enabled
    addedAt
    batchTranslationJobId  # Track background job
  }
}

# Example usage:
# addLanguage(languageCode: "nl", languageName: "Dutch", nativeName: "Nederlands")
# → Adds to active_languages
# → Triggers batch translation of ~100 listings (~$0.25 cost)
# → Future listings auto-include Dutch
```

**GraphQL API with Language Parameter**:

```graphql
# Query listings with automatic language switching
query GetListings(
  $language: String = "en",
  $filters: ListingFilters
) {
  listings(language: $language, filters: $filters) {
    edges {
      node {
        id

        # These fields automatically return translated version if language != source
        title
        description
        tldr
        hoursOfOperation
        eligibilityCriteria

        # Metadata about translation
        sourceLanguage
        translatedLanguage  # Returns null if viewing in source language
        translationNeedsReview  # True if auto-translated and not yet reviewed
        translationMethod  # 'openai', 'human', or 'community'
      }
    }
  }
}

# Admin mutation to approve translation
mutation ApproveTranslation($translationId: UUID!) {
  approveTranslation(translationId: $translationId) {
    id
    needsReview  # Now false
    reviewedAt
    reviewedBy {
      id
      name
    }
  }
}
```

**React i18next Setup (UI Layer - Dynamic Languages)**:

```typescript
// src/i18n.ts - Dynamically loads from active_languages
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import Backend from 'i18next-http-backend';

// Fetch active languages from API
async function getActiveLanguages() {
  const response = await fetch('/api/active-languages');
  const languages = await response.json();
  return languages.map((lang: any) => lang.language_code);
}

// Initialize i18n dynamically
async function initI18n() {
  const supportedLngs = await getActiveLanguages();

  i18n
    .use(Backend)
    .use(initReactI18next)
    .init({
      fallbackLng: 'en',
      supportedLngs,  // Dynamically from database
      debug: false,

      backend: {
        loadPath: '/locales/{{lng}}/translation.json',
      },

      interpolation: {
        escapeValue: false,
      },
    });
}

initI18n();
export default i18n;
```

**Language Toggle Component (Dynamic)**:

```typescript
// src/components/LanguageToggle.tsx
import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';

export function LanguageToggle() {
  const { i18n } = useTranslation();

  // Fetch active languages from API
  const { data: languages } = useQuery({
    queryKey: ['active-languages'],
    queryFn: async () => {
      const response = await fetch('/api/active-languages');
      return response.json();
    },
  });

  return (
    <select
      value={i18n.language}
      onChange={(e) => i18n.changeLanguage(e.target.value)}
      className="language-selector"
    >
      {languages?.map((lang: any) => (
        <option key={lang.language_code} value={lang.language_code}>
          {lang.native_name}
        </option>
      ))}
    </select>
  );
}
```

**Admin Review Interface (MVP: Crude is Fine)**:

```typescript
// MVP: Simple list, "Approve All" button acceptable
function TranslationReviewQueue() {
  const { data } = useQuery(GET_PENDING_TRANSLATIONS);

  return (
    <div>
      <h2>Somali Translations Needing Review ({data.pendingTranslations.length})</h2>

      <button onClick={() => approveAllTranslations()}>
        ✅ Approve All (Quick Review OK)
      </button>

      {data.pendingTranslations.map(trans => (
        <div key={trans.id}>
          <h3>{trans.listing.title}</h3>

          <p><strong>English:</strong> {trans.listing.description}</p>
          <p><strong>Somali:</strong> {trans.description}</p>

          <button onClick={() => approveTranslation(trans.id)}>✅ Approve</button>
        </div>
      ))}
    </div>
  );
}

// Note: Translation accuracy risk < no language access at all
// Crude review is acceptable for MVP
```

**Cost Estimation (MVP: Spanish + Somali)**:

- GPT-4o pricing: ~$0.005 per 1K tokens
- Average listing: ~500 tokens (title + description + fields)
- Translation to 2 languages (Spanish + Somali): ~1000 tokens
- Cost per listing: ~$0.005
- 100 listings: ~$0.50 total
- Adding a new language (e.g., Dutch): +$0.25 for batch translation
- Extremely affordable, caching makes subsequent queries free

**Why Database Caching**:
- Real-time translation would be slow (2-3 seconds per request)
- Real-time translation would be expensive (every page load = API call)
- Cached translations = instant language switching
- Admin review ensures quality
- Can always re-translate if quality issues found

### 2. **Conversational AI with Tools** (THE INTERACTION MODEL)

Healthcare worker doesn't fill out forms. They **talk to AI in natural language** about their patient.

**UX Layout** (Like Vibe Coding):
```
┌─────────────────────────────────────────────────────────┐
│  Chat with AI                │  Document Preview        │
│  (Left Side)                 │  (Right Side)            │
│  ┌─────────────────────┐    │                          │
│  │ [🎤 Talk] [⌨️ Type]  │    │  Resources for Patient   │
│  └─────────────────────┘    │  ─────────────────────  │
│                              │                          │
│  You: 🎤                     │  Immigration Help        │
│  "Somali patient, visa       │  • [AI is searching...] │
│  expired, needs food and     │                          │
│  immigration help. Afraid    │  Food Resources          │
│  to leave home."             │  • [AI is searching...] │
│                              │                          │
│  AI: 🔊                      │  [Document updates       │
│  I found 3 immigration       │   in real-time as       │
│  services with home visits.  │   you speak]            │
│  Should I exclude services   │                          │
│  that require ID?            │  [Edit Document]         │
│                              │  [Finalize & Share]      │
│  You: 🎤                     │                          │
│  "Yes, exclude ID required." │                          │
│                              │                          │
│  AI: 🔊                      │                          │
│  ✓ Found 2 services          │                          │
│  [Updates document]          │                          │
│                              │                          │
└─────────────────────────────────────────────────────────┘
```

**Voice Interaction (LiveKit)**:
- Healthcare worker clicks 🎤 to talk to AI
- AI transcribes speech in real-time
- AI processes request and responds via voice
- Healthcare worker can interrupt/correct mid-conversation
- Fallback to text chat if voice not available

**AI Tools** (What AI Can Do):

1. **search_listings(query, constraints, location, radius_km)**
   - Searches organization_needs for services
   - Applies fear constraints (requires_id, contacts_authorities, avoids_facility)
   - **Location-based filtering**: finds listings within radius_km of location
   - Returns listings sorted by distance (if location provided)
   - Example: `search_listings("food bank", {requires_id: false}, [44.9778, -93.2650], 10)`

2. **get_organization_info(org_name)**
   - Gets details about an organization
   - Returns contact info, hours, services offered, location

3. **find_contacts(category, location)**
   - Finds who to talk to for specific needs
   - Returns people, hotlines, organizations

4. **create_document(language, content)**
   - Generates document in markdown + components format
   - Embeds listings as `<Listing id="..." />` components
   - Can embed map: `<Map><Listing id="..." /></Map>`
   - AI writes natural narrative text around listings
   - Returns draft for healthcare worker to edit

5. **add_language(language_code, language_name, native_name)**
   - Dynamically adds a new language to the system
   - Triggers batch translation of all existing listings
   - Future listings auto-translate to this language
   - Example: `add_language("nl", "Dutch", "Nederlands")`

**Conversational Flow Example**:

```
Healthcare Worker: "Somali patient, undocumented, needs doctor who does home visits"

AI: [Calls search_listings(query="telehealth home visit doctor", constraints={requires_id: false, avoids_facility: true})]
    ✓ Found 2 services:
    1. Somali Health Clinic - Home Visits
    2. Minneapolis Telehealth Network

    Should I add both to the document?

Healthcare Worker: "Yes, add both. Also need medication delivery."

AI: [Calls update_document(action="add", listing_ids=[1,2])]
    ✓ Added to document

    [Calls search_listings(query="medication delivery pharmacy")]
    ✓ Found 1 pharmacy:
    1. Community Pharmacy - Free Delivery

    Should I add this?

Healthcare Worker: "Yes. Make the document in Somali."

AI: [Calls update_document(action="set_language", language="so")]
    ✓ Document will be generated in Somali

    Ready to finalize?

Healthcare Worker: [Edits document directly, adds personal note]
                  "Yes, share link."

AI: ✓ Document finalized
    Link: mndigitalaid.org/referral/abc123
```

**Why This Works**:
- Natural conversation (not form filling)
- AI pulls from all available data sources
- Healthcare worker stays in control (can edit document)
- Iterative (can refine as conversation progresses)
- Fast (AI does the searching, worker does the deciding)

---

### Public Self-Service Interface (For People Seeking Help Directly)

**URL**: `mndigitalaid.org` (public homepage)

**No authentication required** - anyone can use it

**Two Modes**:

#### Mode 1: Browse/Catalog (Simple Directory)

Just show all listings - no AI needed:

```
┌─────────────────────────────────────────────────────────┐
│  MN Digital Aid                                          │
│  🌐 Language: [English ▾]  [🔍 Search] [📍 Near Me]    │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Categories:                                             │
│  [Food] [Housing] [Healthcare] [Legal] [All]           │
│                                                          │
│  Filters:                                                │
│  ☐ No ID required                                       │
│  ☐ Home delivery available                              │
│  ☐ No contact with authorities                          │
│  ☐ Free services only                                   │
│                                                          │
│  ─────────────────────────────────────────────────────  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │ Halal Food Bank              [📍 0.5 km away]   │  │
│  │ ✅ No ID required • Home delivery                │  │
│  │                                                   │  │
│  │ Free groceries for families in need...          │  │
│  │                                                   │  │
│  │ 📞 (612) 555-0123  📧 help@halal.org            │  │
│  │ [View Details] [Get Directions]                  │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │ Somali Home Health              [📍 1.2 km]     │  │
│  │ ✅ No ID required • Home visits                  │  │
│  │                                                   │  │
│  │ In-home medical care, Somali-speaking staff...  │  │
│  │                                                   │  │
│  │ 📞 (612) 555-0456  📧 intake@shhc.org           │  │
│  │ [View Details] [Call Now]                        │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  [Load More...]                                          │
│                                                          │
│  ─────────────────────────────────────────────────────  │
│  💬 Need help finding something? [Chat with AI]         │
└─────────────────────────────────────────────────────────┘
```

**Features**:
- See all active listings immediately
- Filter by category, constraints, location
- Sort by distance (if location enabled)
- Click to see full details
- Direct contact buttons
- **No AI chat required** (but available if needed)

#### Mode 2: Chat with AI (Personalized Search)

**UX**: Same conversational interface, but simplified:

```
┌─────────────────────────────────────────────────────────┐
│  🌐 Select Language: [English ▾] [Español] [Soomaali]  │
├─────────────────────────────────────────────────────────┤
│  Chat with AI               │  Your Resources           │
│  (Left Side)                │  (Right Side)             │
│  ┌─────────────────────┐   │                           │
│  │ [🎤 Talk] [⌨️ Type]  │   │  Building your list...    │
│  └─────────────────────┘   │                           │
│                             │                           │
│  You: 🎤                    │  Food Banks Near You      │
│  "I need food. I don't      │  • [AI is searching...]  │
│  have ID and I'm afraid     │                           │
│  to go to facilities."      │  Home Healthcare          │
│                             │  • [AI is searching...]  │
│  AI: 🔊                     │                           │
│  I found 3 food banks       │  [Save My List]          │
│  that:                      │  [Print]                 │
│  - Don't require ID         │  [Text to Myself]        │
│  - Deliver to your home     │                           │
│  - Are within 5km           │                           │
│                             │                           │
│  Should I add them?         │                           │
│                             │                           │
│  You: 🎤                    │                           │
│  "Yes, please."             │                           │
│                             │                           │
└─────────────────────────────────────────────────────────┘
```

**Key Differences from Healthcare Worker Interface**:
- **No login** - completely public
- **Simplified language** - talks directly to person, not about a patient
- **Self-save** - person saves their own list (no shareable slug needed)
- **Privacy-focused** - no data stored, session only
- **Direct contact** - shows organization contact info prominently

**Example Conversation**:

```
Person: "I need help with food"

AI: "I can help you find food resources. A few questions:
     - Do you have ID? (It's okay if you don't)
     - Are you able to go to a location, or do you need delivery?
     - What language do you prefer?"

Person: "No ID. Need delivery. Somali."

AI: "I found 2 food banks that deliver and don't require ID.
     They're both within 3 km of you.

     Would you like to see them on a map?"

Person: "Yes"

AI: [Adds map to document]
    "Here they are. I've added their phone numbers.
     You can call them directly to arrange delivery."
```

**Contact Organizations Directly**:

Each listing shows:
```markdown
<Listing id="abc-123" showContact={true} />
```

Renders as:
```
┌─────────────────────────────────────────┐
│ Halal Food Bank                         │
│                                         │
│ Free groceries, home delivery available│
│                                         │
│ 📞 Call: (612) 555-0123                │
│ 📧 Email: help@halalfoodbank.org       │
│ 🌐 Website: halalfoodbank.org          │
│                                         │
│ [📱 Call Now] [✉️ Email] [🗺️ Directions]│
└─────────────────────────────────────────┘
```

Person can contact organization directly - no intermediary needed.

### 3. **Geographical/Proximity Search** (Find Nearest Locations)

**Problem**: "Where's the nearest food bank/donation center?"

**Solution**: Location-based search with distance calculation + interactive maps

**Features**:
- **Proximity search**: Find listings within X km of patient location
- **Distance sorting**: Results sorted by distance (closest first)
- **Interactive maps**: Show all locations on a map with pins
- **"Near me"**: Uses browser geolocation for patient location

**SQL Query (PostGIS)**:
```sql
-- Find listings within 10km of patient location
SELECT
  *,
  earth_distance(
    ll_to_earth(latitude, longitude),
    ll_to_earth($patient_lat, $patient_lng)
  ) / 1000 AS distance_km
FROM organization_needs
WHERE
  status = 'active'
  AND requires_id = false  -- Apply constraints
  AND earth_box(
    ll_to_earth($patient_lat, $patient_lng),
    10000  -- 10km radius in meters
  ) @> ll_to_earth(latitude, longitude)
ORDER BY distance_km ASC
LIMIT 20;
```

**Map Component Example**:
```markdown
## Donation Centers Near You

<Map center="patient_location" zoom={12} showDistance={true}>
  <Listing id="abc-123" />  <!-- 0.5 km away -->
  <Listing id="def-456" />  <!-- 1.2 km away -->
  <Listing id="ghi-789" />  <!-- 3.4 km away -->
</Map>
```

**Patient sees**:
- Interactive map with pins for each location
- Distance badges: "0.5 km away", "1.2 km away"
- Click pin → see listing details
- "Get Directions" link → opens Google Maps

### 4. **Service Delivery Modes** (CRITICAL)
- **Telehealth/Virtual** - Video doctor visits, no facility needed
- **Home Visit** - Doctor/nurse comes to patient
- **Medication Delivery** - Pharmacy delivers
- **In-Person** - Traditional facility visit
- **Why**: Patients avoiding hospitals due to fear need alternatives

### 3. **Vetting Process** (AI + Human)
- AI scrapes/extracts service info
- Admin reviews and approves
- Periodic verification (admins check if info is stale)
- Verification badges: ✅ Verified (<30 days) | ⚠️ Unverified

### 4. **Constraint-Based Filtering** (Safety)
- If patient undocumented → exclude services requiring ID
- If patient avoids authorities → exclude services that report
- If patient avoids facilities → show only home/virtual/delivery options
- Hard filters (not just semantic search)

### 5. **Editable Referral Documents**
- AI drafts document in patient's language
- Worker customizes (add notes, remove services, edit text)
- Worker shares permanent link with patient
- **Staleness indicator**: Show "Created [date]" on document so recipient knows how old info is

### 4. **Zero-Match Response**

**Problem**: AI assumes "there will be 3-5 services". What happens when zero safe matches exist?

**Response Design**:

### When No Services Pass Constraints

**AI Output**:
```
Based on your description, I could not find services that safely match all constraints:
  - Patient is undocumented (requires services with no ID requirement)
  - Patient wants to avoid authorities (requires services with no mandatory reporting)
  - Need: immigration help

What I found:
  • 3 immigration services found, but all require ID
  • 2 services found that don't require ID, but contact immigration authorities

I cannot recommend services that violate patient safety constraints.

Suggestions:
  1. Consider relaxing constraint: [which constraint could be relaxed?]
  2. Expand search radius beyond 30km
  3. Contact general legal aid hotline: 1-800-XXX-XXXX (they may know informal resources)

Would you like to try again with different criteria?
```

**UI Treatment**:
- No "Generate Document" button (nothing to share)
- Clear explanation of why no matches
- Actionable alternatives (expand radius, relax constraints, call hotline)
- No apology (this is safety, not failure)

**Worker Can Override**:
- "Show me services anyway (ignore constraints)" - advanced mode
- Worker explicitly acknowledges they're making an unsafe recommendation

**Rationale**: Sometimes the safest answer is "none". High-emotion moment requires clear communication, not algorithmic guessing.

---

## Problem Statement

### Healthcare Workers (Primary User)

**Liability Risk**:
- Making a referral creates exposure (bad info, wrong fit, outdated capacity)
- Need confidence signaling: "verified" vs "unverified" vs "stale"
- Need temporal awareness: "this was true 2 hours ago, confirm before referring"

**Time Risk**:
- Explaining system failures to patients is costly
- Need fast, contextual results (not generic search)
- Need shareable artifacts (not verbal handoffs)

**Current Gaps**:
- Can't keep mental map of all services + their current capacity
- Don't know which services accept undocumented patients
- Don't know which services require ID, proof of residency, etc.
- Can't filter by language, community, or avoidance needs ("no religious affiliation")

**Example Scenario**:
> "Somali patient, immigration visa expired 6 months ago, needs food assistance. Patient specifically wants to avoid services that contact authorities or require ID."

**Desired Outcome**:
- 3-5 services that match constraints (Somali-speaking, no ID required, no authority contact)
- Confidence level for each (verified/unverified)
- Shareable link valid for 24 hours
- Clear disclaimer: "Call to confirm availability"

### People Seeking Help

**Current Pain Points**:
- Immigration status issues (visa expired, uncertain status, in hiding)
- Can't navigate complex systems while in crisis
- Language barriers
- Don't know which organizations serve their community

**Agency Gaps** (CRITICAL):
Not just "what do you need?" but also:
- **What do you want to avoid?** (authorities, religious orgs, ID requirements)
- **What are your preferences?** (walk-ins, phone vs in-person, language)
- **What is your risk tolerance?** (willing to provide info, need anonymity)

**Example Avoidance Needs**:
- "Do NOT contact authorities" (immigration enforcement risk)
- "No religious affiliation required" (secular preference)
- "No mandatory reporting" (domestic violence survivors)
- "Cash only, no paper trail" (economic precarity)

### Organizations

**Capacity Management**:
- Some overwhelmed ("please stop sending people")
- Others invisible ("we have capacity but no referrals")
- Need real-time status updates
- Need to communicate intake process + priority logic

**Individual Providers** (e.g., pro-bono lawyers):
- Risk of being instantly overwhelmed
- Need exposure limits (max N referrals/week)
- Need "warm intro only" mode (not public listing)

### Volunteers/Donors (Existing Feature)

- Want to help but don't know where opportunities are
- Donors outside Minnesota want to support from afar
- ✅ Already supported via existing volunteer matching system

---

## Proposed Solution

### Unified Listing Architecture

**Core Concept**: Extend `organization_needs` table with `listing_type` enum to support multiple content types under one approval workflow, search index, and referral system.

**Key Addition**: Safety fields for constraint-based filtering (NOT just semantic similarity)

```sql
ALTER TABLE organization_needs
  RENAME TO listings;

ALTER TABLE listings
  ADD COLUMN listing_type TEXT NOT NULL DEFAULT 'volunteer_opportunity'
    CHECK (listing_type IN (
      'volunteer_opportunity',  -- Org needs volunteers (existing)
      'service_offered'         -- Org provides help to people in need (NEW)
      -- NOTE: 'donation_request' DEFERRED to Phase 5 (requires economic safeguards)
      -- NOTE: 'business_support' and 'event' REMOVED (out of scope for MVP)
    )),

  -- Service-specific fields
  ADD COLUMN resource_category TEXT,  -- 'food', 'legal', 'immigration', 'shelter', 'telehealth', 'home_healthcare', 'medication_delivery'
  ADD COLUMN eligibility_criteria TEXT,
  ADD COLUMN required_documents TEXT[],
  ADD COLUMN hours_of_operation TEXT,
  ADD COLUMN walk_ins_accepted BOOL,
  ADD COLUMN appointment_required BOOL,
  ADD COLUMN languages_available TEXT[],  -- ['en', 'es', 'so', 'om', 'am'] - ISO 639-1 codes
  ADD COLUMN cost TEXT,  -- 'free', 'sliding_scale', '$50-100'
  ADD COLUMN serves_area TEXT,

  -- CRITICAL: Service delivery modes (how patient accesses care)
  ADD COLUMN service_delivery_mode TEXT[] DEFAULT ARRAY['in_person'],
    -- ['in_person', 'home_visit', 'telehealth', 'medication_delivery', 'virtual', 'mobile_clinic']
  ADD COLUMN avoids_facility_visit BOOL DEFAULT false,  -- True if patient never has to visit hospital/clinic

  -- SAFETY: Explicit constraint fields (NOT just semantic)
  ADD COLUMN requires_id BOOL DEFAULT false,
  ADD COLUMN requires_proof_of_residency BOOL DEFAULT false,
  ADD COLUMN requires_income_verification BOOL DEFAULT false,
  ADD COLUMN immigration_status_accepted TEXT[] DEFAULT ARRAY['all'],
    -- ['all', 'documented', 'undocumented', 'refugee', 'asylum_seeker']
  ADD COLUMN service_restrictions TEXT[],
    -- ['contacts_authorities', 'mandatory_reporting', 'religious_affiliation_required']

  -- SAFETY: Verification and temporal truth
  ADD COLUMN verification_status TEXT DEFAULT 'unverified'
    CHECK (verification_status IN ('verified', 'unverified', 'community_reported')),
  ADD COLUMN last_verified_at TIMESTAMPTZ,
  ADD COLUMN verified_by_admin_id UUID REFERENCES admins(id),

  -- Provider type and throttling (for individuals)
  ADD COLUMN provider_type TEXT DEFAULT 'organization'
    CHECK (provider_type IN ('organization', 'individual', 'network')),
  ADD COLUMN max_referrals_per_week INT,  -- NULL = unlimited
  ADD COLUMN referral_count_this_week INT DEFAULT 0,
  ADD COLUMN referral_method TEXT DEFAULT 'direct'
    CHECK (referral_method IN ('direct', 'warm_intro_only', 'waitlist')),

  -- Capacity management (for all listing types)
  ADD COLUMN capacity_status TEXT DEFAULT 'accepting'
    CHECK (capacity_status IN (
      'accepting',        -- ✅ Currently accepting
      'waitlist',         -- ⏸️ Waitlist available
      'paused',          -- 🚫 Temporarily closed
      'at_capacity'      -- ⚠️ Not accepting new
    )),
  ADD COLUMN capacity_notes TEXT,
  ADD COLUMN capacity_updated_at TIMESTAMPTZ,

  -- Priority logic (org decides, we surface it)
  ADD COLUMN intake_priority TEXT,  -- "First come", "Referral only", "Community priority"
  ADD COLUMN referral_process TEXT,  -- "Call to schedule", "Walk-in Mon-Wed", "Email referrals@"

  -- Geography and remote services
  ADD COLUMN service_delivery_mode TEXT[] DEFAULT ARRAY['in_person'],
    -- ['in_person', 'remote', 'phone', 'virtual']
  ADD COLUMN jurisdictions_served TEXT[],  -- ['MN', 'hennepin_county', 'US']
  ADD COLUMN remote_eligible BOOL DEFAULT false,

  -- Featured/priority (admin controlled)
  ADD COLUMN priority INT DEFAULT 0,
  ADD COLUMN featured_until TIMESTAMPTZ;

-- Indexes
CREATE INDEX idx_listings_type_status ON listings(listing_type, status);
CREATE INDEX idx_listings_category ON listings(resource_category)
  WHERE listing_type = 'service_offered';
CREATE INDEX idx_listings_verification ON listings(verification_status, last_verified_at DESC)
  WHERE listing_type = 'service_offered';
CREATE INDEX idx_listings_constraints ON listings USING GIN(immigration_status_accepted)
  WHERE listing_type = 'service_offered';
CREATE INDEX idx_listings_provider_capacity ON listings(provider_type, referral_count_this_week)
  WHERE provider_type = 'individual';
```

---

## Three-Tier Confidence System

**Healthcare workers need to know: "How confident should I be in this info?"**

### ✅ VERIFIED (High Confidence)
- Admin confirmed contact, capacity, and eligibility within last 30 days
- Shows: ✅ badge + "Verified [date]"
- Recommended for direct referrals

### ⚠️ UNVERIFIED (Medium Confidence)
- Scraped or submitted, but not yet verified by admin
- Shows: ⚠️ badge + "Unverified - confirm before referring"
- Use with caution

### ❓ COMMUNITY-REPORTED (Low Confidence)
- User-submitted, not yet reviewed
- Shows: ❓ badge + "Community report - verify independently"
- For awareness only, not primary referrals

**Verification Decay**:
- After 30 days: VERIFIED → UNVERIFIED
- Admins can "refresh" verification without re-approving
- Weekly cron job flags stale verifications

---

## Editable Referral Documents (Temporal Truth Solution)

**Problem**: Link generated at 10am, org closes intake at noon, patient arrives at 4pm with dead referral.

**Solution**: Healthcare worker **creates an editable referral document** that they review, customize, and approve before sharing.

### Key Insight
Instead of "AI-generated link that expires", we create "AI-drafted document that worker edits and owns".

**Why This Works**:
- Healthcare worker explicitly approves info (by editing)
- Worker can add personal notes: "I called them yesterday, they have capacity"
- Patient sees this as "from my doctor", not "from AI"
- Worker can update and re-share if info changes (living document)
- No expiration needed (worker took ownership at send time)

### Database Schema

```sql
CREATE TABLE referral_documents (
  id UUID PRIMARY KEY,
  session_id UUID NOT NULL REFERENCES assist_sessions(id),
  created_by_user_id UUID NOT NULL,  -- Healthcare worker

  -- Document content (editable rich text)
  title TEXT DEFAULT 'Resources for You',
  header_note TEXT,  -- Personal intro from healthcare worker
  footer_note TEXT,  -- Closing note, contact info

  -- Included services (can be removed/reordered)
  included_listings JSONB,  -- Array of {listing_id, custom_notes, rank}

  -- Document state
  status TEXT DEFAULT 'draft' CHECK (status IN ('draft', 'finalized', 'updated')),
  finalized_at TIMESTAMPTZ,
  last_edited_at TIMESTAMPTZ DEFAULT NOW(),

  -- Shareable URL (never expires)
  slug TEXT UNIQUE NOT NULL,  -- e.g., 'dr-smith-resources-abc123'

  -- Tracking (aggregate only, no patient PII)
  view_count INT DEFAULT 0,
  last_viewed_at TIMESTAMPTZ,

  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_referral_docs_user ON referral_documents(created_by_user_id, created_at DESC);
CREATE INDEX idx_referral_docs_slug ON referral_documents(slug);
```

### Document Structure (Rich Text)

```markdown
# [Editable Title: "Resources Recommended by Dr. Smith"]

[Header Note - Editable by healthcare worker:]
Hi,

Based on our conversation today, I've put together some resources that might help with [immigration help, food assistance]. I've worked with these organizations before and recommend reaching out to them.

If you have questions, you can reach me at [doctor's contact].

---

## Service 1: Somali Law Center
✅ VERIFIED (Confirmed Jan 28, 2026)

**What they offer**: Free immigration legal consultations, visa renewal assistance, know-your-rights workshops

**Languages**: Somali, English, Arabic

**Operational Hours**:
- Monday-Friday: 9:00 AM - 5:00 PM
- Saturday: 10:00 AM - 2:00 PM (appointments only)
- Closed Sundays

**Contact Information**:
- Phone: (612) 555-0123
- Email: intake@somalilawcenter.org
- Website: somalilawcenter.org

**How to Access**:
- Walk-ins welcome during business hours
- Call ahead to schedule appointment (recommended)
- No ID required for initial consultation
- They do not contact immigration authorities

**Location**:
1234 Main Street, Minneapolis, MN 55408
[View on Map]

**Eligibility**:
- All immigration statuses welcome
- Free for low-income individuals
- Sliding scale fees available

[Custom Note - Editable by healthcare worker:]
_I spoke with them last week and they currently have capacity for new clients. Ask for Maria when you call._

---

## Service 2: Halal Food Shelf

**What they offer**: Free groceries, no questions asked

**Operational Hours**:
- Tuesday and Thursday: 10:00 AM - 4:00 PM

**Contact**: (612) 555-0456

**Location**: 5678 Central Ave, Minneapolis, MN

[Custom Note - Editable:]
_This is a walk-in food shelf. No appointment needed, no paperwork required._

---

[Footer Note - Editable:]
Please don't hesitate to reach out if you have trouble accessing these services or if you need additional help. I'm here to support you.

Take care,
Dr. Smith
(612) 555-XXXX

---

_This document was created on Jan 29, 2026. If you're viewing this more than a few weeks later, please call the services to confirm their current hours and availability._
```

### Healthcare Worker UI (Document Editor)

**Step 1: AI generates draft**
```
[AI Assistant]
Based on your description, I've created a referral document with 3 services.

[Preview Draft Document]

Actions:
  [Edit Document] ← Opens rich text editor
  [Share Now] (if worker doesn't want to edit)
  [Add More Services]
  [Start Over]
```

**Step 2: Worker edits in rich text editor**
```
[Rich Text Editor - Like Google Docs]

Title: [Resources Recommended by Dr. Smith] ✏️

Header Note: ✏️
Hi,

Based on our conversation today, I've put together...

---

Services Included:
  1. ✅ Somali Law Center
     [Edit] [Remove] [Move Up/Down]

     Custom Note: ✏️
     "I spoke with them last week..."

  2. Halal Food Shelf
     [Edit] [Remove]

[+ Add Another Service]

---

Footer Note: ✏️
Please don't hesitate to reach out...

[Save Draft] [Finalize & Share]
```

**Step 3: Worker finalizes and shares**
```
[Document Finalized]

Your referral document is ready to share:

📄 https://mndigitalaid.org/referral/dr-smith-resources-abc123

Options:
  [Copy Link]
  [Send via Text Message]
  [Send via Email]
  [Print PDF]
  [Edit Document] ← Can always update later

Stats:
  Created: Jan 29, 2026 2:15 PM
  Last edited: Jan 29, 2026 2:20 PM
  Views: 0
  Status: Finalized ✅
```

### Patient View (Shared Link)

```
[https://mndigitalaid.org/referral/dr-smith-resources-abc123]

[MN Digital Aid Logo]

# Resources Recommended by Dr. Smith

[Shows full document as formatted by healthcare worker]

[Footer - Subtle timestamp]
This document was created by your healthcare provider on Jan 29, 2026.
If you have questions, contact your provider directly.

[No "expired" warnings, no "stale info" banners]
[Worker took ownership by editing, so we trust their judgment]
```

### Living Document (Updates)

**If healthcare worker needs to update**:
```
[My Referral Documents]

📄 Resources for Patient (Jan 29)
   Views: 5
   Last viewed: 2 hours ago

   [Edit Document] ← Makes changes
   [View Public Link]
   [Duplicate] ← Create variant for another patient

When edited:
  - URL stays the same (slug doesn't change)
  - Patient sees updated content
  - "Last updated" timestamp shown
  - No "version history" (keeps it simple)
```

---

## Constraint-Based Matching (NOT Just Semantic)

**Critical Principle**: Semantic similarity is secondary to constraint compatibility.

### AI Prompt for Healthcare Worker Tool

```
SYSTEM PROMPT:

You are helping healthcare workers find services for patients in crisis.

Your role:
1. Parse the worker's description to extract:
   - Service categories (food, legal, immigration, shelter, healthcare)
   - Languages required
   - Communities served
   - Urgency level

2. CRITICAL: Extract AVOIDANCE NEEDS (hard constraints):
   - requires_no_id: exclude services that require ID
   - avoid_authorities: exclude services that contact immigration/police
   - avoid_religious: exclude services with religious affiliation requirement
   - avoid_mandatory_reporting: exclude services with reporting obligations
   - undocumented_safe: ONLY show services accepting undocumented immigrants

3. You will receive services with:
   - Semantic similarity scores
   - Constraint compatibility flags

4. RANKING LOGIC (in order):
   a) Constraint compatibility (MUST satisfy constraints, or exclude)
   b) Verification status (verified > unverified > community)
   c) Capacity status (accepting > waitlist > paused)
   d) Semantic similarity (for tiebreaking only)

5. For each service, generate reasoning (1-2 sentences) explaining:
   - Why it fits patient needs
   - Any caveats (waitlist, unverified, etc.)

CRITICAL RULES:
- If patient is undocumented, EXCLUDE all services where requires_id = true
- If patient wants to avoid authorities, EXCLUDE all services where 'contacts_authorities' IN service_restrictions
- Semantic similarity does NOT override constraints
- When in doubt, err on side of caution (exclude, don't include)

OUTPUT FORMAT (JSON):
{
  "parsed_needs": {
    "categories": ["immigration", "food"],
    "languages": ["somali"],
    "communities": ["somali", "east_african"],
    "urgency": "immediate"
  },
  "parsed_avoidances": {
    "requires_no_id": true,
    "avoid_authorities": true,
    "avoid_religious": false
  },
  "recommendations": [
    {
      "listing_id": "uuid",
      "reasoning": "Specializes in Somali immigration cases, no ID required, free consultations",
      "confidence": "high",  // based on verification_status
      "caveats": ["waitlist: 1-2 weeks"]
    }
  ]
}
```

### Healthcare Worker UI

```
Patient needs: [text area]

⚠️ IMPORTANT: Patient safety considerations
  □ Patient is undocumented (exclude services requiring ID)
  □ Patient wants to avoid authorities (exclude services with mandatory reporting)
  □ Patient prefers secular services (exclude religious affiliation)
  □ Patient needs walk-in access (exclude appointment-only)

Optional filters:
  □ Somali-speaking staff
  □ Free services only
  □ Within 10 miles of [zip code]

[Generate Recommendations]
```

**Effect Logic**:
```rust
// In AIRecommendationEffect

// 1. Parse needs + avoidances from worker input
let parsed = parse_needs_with_avoidances(&needs_description).await?;

// 2. Build constraint filters (HARD EXCLUSIONS)
let mut filters = vec![];

if parsed.avoidances.requires_no_id {
  filters.push("requires_id = false");
}

if parsed.avoidances.avoid_authorities {
  filters.push("NOT ('contacts_authorities' = ANY(service_restrictions))");
}

if parsed.avoidances.undocumented_safe {
  filters.push("'undocumented' = ANY(immigration_status_accepted) OR 'all' = ANY(immigration_status_accepted)");
}

// 3. Semantic search (with hard filters applied)
let candidates = semantic_search_with_filters(
  &parsed.needs,
  &filters,
  limit: 20
).await?;

// 4. Rank by: constraints > verification > capacity > similarity
let ranked = rank_by_safety_first(candidates);

// 5. AI generates reasoning for top 5
let recommendations = generate_reasoning(ranked.take(5)).await?;
```

---

## Four User Journeys

### 1. Healthcare Worker (AI-Assisted Referral Document Creation)

**URL**: `mndigitalaid.org/assist`

**Flow**:
1. Worker describes patient needs + avoidances
2. AI parses constraints, performs semantic search with hard filters
3. Returns 3-5 services ranked by: constraints > verification > capacity > similarity
4. **AI generates draft referral document** (rich text)
5. **Worker edits document** (adds personal notes, removes services, updates text)
6. Worker finalizes and shares permanent link with patient

**Key Features**:
- No patient PII stored (worker describes generically)
- Worker explicitly approves content by editing
- Rich text editor (like Google Docs)
- Permanent links (no expiration - worker took ownership)
- Can be updated and re-shared (living document)

**Example Flow**:

**Step 1: AI Draft**
```
Based on your description, I've created a referral document:

[Preview:]

# Resources for You

Hi,

Based on our conversation, here are some services that can help with immigration and food assistance.

---

## 1. Somali Law Center
✅ VERIFIED (Jan 28, 2026)

Specializes in visa issues, Somali-speaking staff, no ID required

Hours: Mon-Fri 9am-5pm
Contact: (612) 555-0123
Cost: Free initial consultation

[Custom note: _Add your personal recommendation here_]

---

## 2. Halal Food Shelf
⚠️ UNVERIFIED

Serves Somali community, no questions asked

Hours: Tue/Thu 10am-4pm
Contact: (612) 555-0456

---

[Edit Document] [Share Now]
```

**Step 2: Worker Edits**
```
[Rich Text Editor]

# Resources Recommended by Dr. Smith ✏️

Hi,

Based on our conversation today, I've put together resources for immigration help and food assistance. I've worked with the Somali Law Center before and highly recommend them.

---

## Somali Law Center
✅ VERIFIED

Immigration legal services, visa renewals, know-your-rights

Hours: Monday-Friday 9am-5pm
Contact: (612) 555-0123
Website: somalilawcenter.org

How to access:
- Walk-ins welcome
- No ID required for consultation
- They do not contact authorities

✏️ [Worker adds:] I called them yesterday and they have immediate capacity. Ask for Maria.

---

## Halal Food Shelf

Free groceries, no questions asked

Hours: Tuesday/Thursday 10am-4pm
Contact: (612) 555-0456
Location: 5678 Central Ave

✏️ [Worker adds:] No appointment needed, just walk in during hours.

---

Please call me if you need help: (612) 555-XXXX

- Dr. Smith

[Finalize & Share]
```

**Step 3: Worker Shares**
```
✅ Document finalized!

📄 https://mndigitalaid.org/referral/dr-smith-jan29

[Copy Link] [Text to Patient] [Email]

This link never expires. You can update the document anytime and changes will appear immediately.
```

---

### 2. Person Seeking Help (Referral Document View)

**URL**: `mndigitalaid.org/referral/dr-smith-jan29` (from healthcare worker)

**Features**:
- No account required
- Shows exactly what healthcare worker finalized (no filtering, no changes)
- Rich text formatting (headers, sections, notes)
- Personal notes from healthcare worker included
- Clean, printable layout

**Example Patient View**:
```
[Clean, branded header]
MN Digital Aid - Referral Resources

---

# Resources Recommended by Dr. Smith

Hi,

Based on our conversation today, I've put together resources for immigration help and food assistance. I've worked with the Somali Law Center before and highly recommend them.

---

## Somali Law Center
✅ VERIFIED (Confirmed by our team Jan 28, 2026)

**What they offer**: Immigration legal services, visa renewals, know-your-rights workshops

**Languages**: Somali, English, Arabic

**Operational Hours**:
- Monday-Friday: 9:00 AM - 5:00 PM
- Saturday: 10:00 AM - 2:00 PM (appointments only)

**Contact**:
- Phone: (612) 555-0123
- Email: intake@somalilawcenter.org
- Website: somalilawcenter.org

**How to Access**:
- Walk-ins welcome during business hours
- Call ahead to schedule (recommended)
- No ID required for initial consultation
- They do not contact immigration authorities

**Location**:
1234 Main Street, Minneapolis, MN 55408
[View on Map] [Get Directions]

**Note from Dr. Smith**:
_I called them yesterday and they have immediate capacity for new clients. Ask for Maria when you call._

---

## Halal Food Shelf

**What they offer**: Free groceries, no questions asked

**Hours**: Tuesday and Thursday, 10:00 AM - 4:00 PM

**Contact**: (612) 555-0456

**Location**: 5678 Central Ave, Minneapolis, MN

**Note from Dr. Smith**:
_No appointment needed, just walk in during hours. Bring your own bags if you can._

---

Please call me if you need help: (612) 555-XXXX

Take care,
Dr. Smith

---

[Footer - subtle]
This referral document was created by your healthcare provider on Jan 29, 2026.
If you have questions, contact your provider directly.

[Print] [Email to Myself]
```

**No Staleness Warnings**:
- Healthcare worker approved this by editing
- Worker can update document if info changes
- Trust the healthcare worker's judgment
- Only shows timestamp in footer (non-intrusive)

---

### 3. Organization Portal (Self-Service)

**URL**: `mndigitalaid.org/org`

**Features**:
- Organizations sign up via phone/email OTP
- CRUD interface for listings
- Real-time capacity updates
- Set intake priority + referral process
- Set exposure limits (for individuals: max N referrals/week)
- View analytics (views, clicks, but NOT individual patient data)

**Capacity Management UI**:
```
Service: Immigration Legal Services

Current Status: ✅ Accepting new clients
  ○ Accepting (✅)
  ○ Waitlist (⏸️) — Notes: _________
  ○ Paused (🚫) — Notes: _________
  ○ At Capacity (⚠️) — Notes: _________

Intake Priority: [dropdown]
  ○ First come, first served
  ○ Referral only (not accepting walk-ins)
  ○ Priority: Somali community
  ○ Other: [text input]

Referral Process: [text area]
  "Call (612) 555-0123 to schedule. Ask for Maria."

[Update Status]
```

**For Individual Providers** (pro-bono lawyers):
```
Provider Type: Individual

Exposure Limits:
  Max referrals per week: [5] (prevents being overwhelmed)
  Current count this week: 3 / 5

  ☑ Pause visibility when at limit

Referral Method:
  ○ Direct (show in public search)
  ○ Warm intro only (require existing client referral)
  ○ Waitlist (collect names, I'll reach out)
```

---

### 4. Admin Approval + Verification

**URL**: `mndigitalaid.org/admin`

**Unified Queue**:
- All listing types in one queue (services, volunteer opportunities, donations)
- Actions per listing:
  - ✅ **Approve** (set status=active, verification_status=unverified)
  - ✅ **Approve + Verify** (set verification_status=verified, last_verified_at=NOW)
  - ✏️ **Edit + Approve**
  - ❌ **Reject**

**Verification Workflow**:
```
Pending: Somali Law Center - Immigration Services

Title: Immigration Legal Services
Category: immigration
Cost: Free
Requires ID: No ✓ (safe for undocumented)
Contact: (612) 555-0123

Admin Actions:
  [Approve Only] (status → active, unverified)
  [Approve + Verify] (status → active, verified ✅)
    → Requires: Called to confirm contact, hours, eligibility
  [Edit] [Reject]
```

**Verification Refresh** (for stale verified listings):
```
⚠️ 3 listings need verification refresh (>30 days old)

Somali Law Center - Immigration Services
  Last verified: Dec 28, 2025 (32 days ago)

  [Refresh Verification] → Calls org, confirms info still accurate
  [Mark Unverified] → Downgrades to unverified status
```

---

## Organization Capacity Updates (MVP - Wire Up Now)

### The Problem
Organizations need to signal "we're full" or "we're back open" without waiting for admin intervention.

### Simple MVP Solution: Email Verification Claiming

**Flow**:
1. Admin adds organization with `domain_id` linked to verified domain
2. Organization can claim their profile via email verification
3. Once claimed, org can update `capacity_status`

**Claiming Mechanism**:

```sql
-- Add claiming support to organizations
ALTER TABLE organizations
  ADD COLUMN claimed_at TIMESTAMPTZ,
  ADD COLUMN claim_token TEXT UNIQUE,
  ADD COLUMN claim_email TEXT;  -- Email that claimed this org

CREATE INDEX idx_organizations_claimed ON organizations(claimed_at) WHERE claimed_at IS NOT NULL;
```

**Claiming Flow**:

```
Admin creates org → System generates claim_token
                  ↓
Admin clicks "Send Claim Email"
                  ↓
System sends email to admin@{domain} or contact email
                  ↓
Email: "Claim your organization on MN Digital Aid"
       [Claim Link: https://mndigitalaid.org/claim/{claim_token}]
                  ↓
Org rep clicks link → Simple form:
  - Confirm org name
  - Set contact email (for future updates)
  - [Claim Organization]
                  ↓
Organization claimed! Redirects to capacity update page
```

**Capacity Update Page** (Simple for MVP):

```
┌─────────────────────────────────────────────────┐
│ Somali Law Center - Immigration Services        │
│                                                 │
│ Current Status: 🟢 Accepting New Clients       │
│                                                 │
│ ○ Accepting (we can take new clients)          │
│ ○ Paused (temporarily not taking new clients)  │
│ ○ At Capacity (full, check back later)         │
│                                                 │
│ [Update Status]                                 │
│                                                 │
│ Last updated: Jan 29, 2026 by you              │
└─────────────────────────────────────────────────┘
```

**Implementation**:

```rust
// Seesaw command
pub struct ClaimOrganization {
    pub claim_token: String,
    pub contact_email: String,
}

pub struct UpdateCapacityStatus {
    pub organization_id: Uuid,
    pub new_status: CapacityStatus,
    pub updated_by_email: String,
}

// Simple GraphQL mutations
type Mutation {
  claimOrganization(token: String!, email: String!): Organization!
  updateCapacityStatus(orgId: ID!, status: CapacityStatus!): Organization!
}
```

**Security**:
- Claim token is single-use, expires after 30 days
- Once claimed, future updates require magic link to `claim_email`
- No passwords, no user accounts - just email verification

**Future**: This foundation supports Phase 3 org portals without rearchitecting.

---

## Implementation Phases

### Phase 1: MVP Core Only (Weeks 1-2)

**Goal**: Ship the smallest version that produces direct impact

**If it doesn't enable "person finds safe service that helps them", it's not in Phase 1.**

**Priority Order**:
1. Three entry points (I need help / I want to help / Create referral) - MOST IMPORTANT
2. Constraint filtering (safety)
3. Multi-language
4. Org capacity updates (wire up now)

**MVP Core Tasks** (in priority order):

**1. Homepage with Three Entry Points**:
- [ ] Public homepage at `mndigitalaid.org` (no auth required)
- [ ] Three clear buttons:
  - [ ] 🆘 "I Need Help" → Browse services (food, housing, healthcare, legal)
  - [ ] 🤝 "I Want to Help" → Browse opportunities (volunteer, donate, support businesses)
  - [ ] 🏥 "Create Referral" → Healthcare worker document builder
- [ ] Mobile-responsive design

**2. "I Need Help" - Services Directory** (Get Services):
- [ ] **Catalog view**: Show all service listings as cards
  - [ ] Listing card: title, description, contact, distance badge, verification badge
  - [ ] "View Details" → full page with map
- [ ] **Category filters**: Food, Housing, Healthcare, Legal, All
- [ ] **Constraint filters** (fear-specific):
  - [ ] ☐ No ID required
  - [ ] ☐ Home delivery available
  - [ ] ☐ No contact with authorities
  - [ ] ☐ Free services only
- [ ] **Location features**:
  - [ ] "📍 Near Me" button (browser geolocation)
  - [ ] Distance badges on listings
  - [ ] Sort by distance
- [ ] **Direct action buttons**: 📱 Call Now, ✉️ Email, 🗺️ Get Directions
- [ ] **Capacity badges**: 🟢 Accepting / 🟡 Limited / 🔴 At Capacity
- [ ] **Language selector**: English, Spanish, Somali (switches all content)
- [ ] Test with sister: "Can regular people find food banks easily?"

**3. "I Want to Help" - Opportunities Directory** (Give Back):
- [ ] **Catalog view**: Show opportunities + businesses as cards
- [ ] **Type filters**: All, Volunteer, Donate, Support Businesses
- [ ] **Opportunity cards** show:
  - [ ] Type badge (volunteer/donation/business support)
  - [ ] What's needed (time commitment, donation amount, customers)
  - [ ] Current situation (why they need help)
  - [ ] How to help (volunteer link, donation link, visit business)
- [ ] **Business cards** show:
  - [ ] Community-owned badge (Somali-owned, Ethiopian-owned, etc.)
  - [ ] Support needed (customers, donations, visibility)
  - [ ] Current situation ("Owners temporarily in hiding")
  - [ ] Actions: 💰 Donate, 🎟️ Buy Gift Card, 🚗 Visit, 🌐 Order Online
- [ ] **Community filter**: Show businesses/orgs serving specific communities
- [ ] Language selector
- [ ] Test: "Can I find Somali businesses to support?"

**2. Fear-Specific Constraints (THE SAFETY SPINE)**:
- [ ] Add 3 constraint fields to `organization_needs`:
  - `requires_id BOOL DEFAULT false`
  - `contacts_authorities BOOL DEFAULT false`
  - `avoids_facility_visit BOOL DEFAULT false`
- [ ] Add `service_delivery_mode TEXT[]` (in_person, telehealth, home_visit, medication_delivery)
- [ ] Update GraphQL schema for constraint filtering
- [ ] Implement hard constraint filtering in search (SQL WHERE clauses, not semantic ranking)
- [ ] Test: Undocumented patient → excludes requires_id=true services

**2. Zero-Match Response**:
- [ ] Implement AI response when no safe matches found
- [ ] Show clear explanation of why no matches
- [ ] Offer next-best steps (hotline, expand radius)
- [ ] No "Generate Document" button on zero-match

**3. Editable Referral Documents (THE FLOW SPINE)**:
- [ ] Create `referral_documents` table (title, header_note, footer_note, included_listings, slug)
- [ ] AI generates draft document from search results
- [ ] Simple rich text editor (or even plain text for MVP)
- [ ] Healthcare worker can edit, remove services, add notes
- [ ] Finalize → generates permanent link
- [ ] Patient view: clean, printable layout

**4. Dynamic Multi-Language System (MVP: English + Spanish + Somali)**:
- [ ] Create `active_languages` table (language_code, language_name, native_name, enabled)
- [ ] Insert MVP languages: English, Spanish, Somali
- [ ] Create `listing_translations` table (listing_id, language FK, title, description)
- [ ] Add `source_language TEXT DEFAULT 'en'` to `organization_needs`
- [ ] **Generic translation effect**: Translates to ALL active languages (no hardcoded language logic)
- [ ] **Batch translation effect**: When new language added, translate all existing listings
- [ ] GraphQL: `language: String` parameter returns translated fields
- [ ] GraphQL mutation: `addLanguage(code, name, nativeName)` → triggers batch translation
- [ ] AI tool: `add_language()` for conversational language addition
- [ ] Frontend: Dynamic language toggle (fetches from active_languages API)
- [ ] Create `/locales/en/translation.json` + `/locales/es/translation.json` + `/locales/so/translation.json`
- [ ] Crude admin review: "Approve All" button is fine

**5. Geographical/Proximity Search**:
- [ ] Enable PostGIS extension in PostgreSQL
- [ ] Add geospatial index: `ll_to_earth(latitude, longitude)`
- [ ] Implement proximity search query (find within X km)
- [ ] Add distance calculation to search results
- [ ] Sort results by distance when location provided
- [ ] **Map component** (`<Map>`) in referral documents:
  - [ ] Integrate Leaflet or Mapbox
  - [ ] Parse `<Map>` from markdown content
  - [ ] Plot listings as interactive pins
  - [ ] Show distance badges if `showDistance={true}`
  - [ ] "Get Directions" links to Google Maps
- [ ] Browser geolocation: `center="patient_location"`
- [ ] Test with sister: "show nearest food banks"

**6. Conversational AI Interface (The Interaction Model)**:
- [ ] **Split-pane UI**: Chat (left) + Document Preview (right)
- [ ] **LiveKit Integration** for voice chat:
  - [ ] Add LiveKit client SDK
  - [ ] "Talk" button for voice input
  - [ ] Real-time speech-to-text transcription
  - [ ] AI voice responses (text-to-speech)
  - [ ] Fallback to text chat if voice unavailable
- [ ] **AI Tools Implementation**:
  - [ ] `search_listings(query, constraints)` - searches organization_needs
  - [ ] `get_organization_info(org_name)` - gets org details
  - [ ] `find_contacts(category, location)` - finds relevant contacts
  - [ ] `update_document(action, content)` - modifies document
- [ ] Real-time document preview updates as conversation progresses
- [ ] Healthcare worker can click into document and edit directly
- [ ] Language toggle (English/Somali) applies to document
- [ ] AI applies fear constraints automatically based on conversation
- [ ] "Finalize" → generates permanent shareable link
- [ ] Test with sister using real scenarios (voice + text)

**Implementation Notes**:
- Use Claude with tools (like MCP but simpler)
- Tools return structured data that AI formats naturally
- Document updates happen client-side (instant feedback)
- Keep chat history in session for context
- **LiveKit integration** for live voice chat with AI agent
  - Healthcare worker can talk to AI instead of typing
  - Faster for busy healthcare workers
  - More natural conversation flow
  - AI transcribes speech → processes → responds via voice

**7. Organization Capacity Updates** (Wire Up Now):
- [ ] Add capacity fields to organizations:
  - [ ] `claimed_at TIMESTAMPTZ`
  - [ ] `claim_token TEXT UNIQUE`
  - [ ] `claim_email TEXT`
- [ ] Admin UI: "Send Claim Email" button for each org
  - [ ] Generates claim token, sends to admin@{domain} or contact email
  - [ ] Email template: "Claim your organization on MN Digital Aid"
- [ ] Claim page: `/claim/{token}`
  - [ ] Simple form: confirm org name, set contact email
  - [ ] Claims org, redirects to capacity update page
- [ ] Capacity update page: `/org/{id}/capacity`
  - [ ] Radio buttons: Accepting / Paused / At Capacity
  - [ ] "Update Status" button
  - [ ] Shows last updated time
- [ ] GraphQL mutations:
  - [ ] `claimOrganization(token: String!, email: String!)`
  - [ ] `updateCapacityStatus(orgId: ID!, status: CapacityStatus!)`
- [ ] Seesaw commands:
  - [ ] `ClaimOrganization { claim_token, contact_email }`
  - [ ] `UpdateCapacityStatus { organization_id, new_status, updated_by_email }`
- [ ] Security: claim token single-use, expires after 30 days
- [ ] Test: Admin sends claim, org rep claims, updates status to "At Capacity"

**NOT in Phase 1** (intentionally under-polish):
- ❌ Full organization self-service portal (just capacity updates for now)
- ❌ Verification sophistication (manual "last checked" date)
- ❌ Analytics dashboards (views count only)
- ❌ Perfect AI parsing (blunt but safe)
- ❌ Fancy translation review UI
- ❌ Provider throttling
- ❌ User accounts for public (stay anonymous)

**Deliverable** (in priority order):
1. **PRIMARY**: Person can browse public directory, find services with "No ID required" filter, contact directly
2. **SECONDARY**: Healthcare worker can generate personalized 1:1 referral document (private link, not browsable)

**Key Principle**: Public browse is the main interface. Healthcare worker links are personalized/private for specific patients only.

---

### Phase 2: Polish What Works (Weeks 3-4)

**Goal**: Improve MVP based on sister's feedback

**Only add features that solve real problems discovered in Phase 1 usage.**

**Potential additions** (only if needed):
- [ ] Spanish translation (if Spanish-speaking patients are barrier)
- [ ] Verification badges (if trust is issue)
- [ ] Better constraint parsing (if AI misunderstands inputs)
- [ ] Service detail pages (if document isn't enough context)

**Deliverable**: MVP works reliably for 5+ healthcare workers

---

### Phase 3: Organization Self-Service (Weeks 5-6)

**Goal**: Let organizations update their own listings

**Tasks**:
- [ ] Organization signup (phone/email OTP)
- [ ] Simple listing CRUD interface
- [ ] Capacity status updates (accepting/paused/at_capacity)
- [ ] Manual verification workflow for admins

**Deliverable**: 3+ organizations managing their own listings

---

### Phase 4: Additional Languages (Weeks 7-8)

**Goal**: Expand language support based on demand

**Add in order of need**:
1. Spanish (if needed)
2. Oromo (if needed)
3. Amharic (if needed)

**Tasks per language**:
- [ ] Update `listing_translations` CHECK constraint
- [ ] Add `/locales/{lang}/translation.json`
- [ ] Batch translate existing listings
- [ ] Native speaker quality check

**Deliverable**: Support for 3-4 languages total

---

## Complete Database Schema (Final - Hybrid Approach)

### Organizations & Domains

```sql
-- Domains (websites we scrape)
CREATE TABLE domains (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  domain_url TEXT NOT NULL UNIQUE,
  scrape_frequency_hours INT DEFAULT 24,
  last_scraped_at TIMESTAMPTZ,
  active BOOL DEFAULT true,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_domains_active ON domains(active);

-- Specific URLs to scrape within a domain
CREATE TABLE domain_scrape_urls (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  domain_id UUID NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  active BOOL DEFAULT true,
  added_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(domain_id, url)
);

-- Organizations
CREATE TABLE organizations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  domain_id UUID REFERENCES domains(id),

  -- Contact
  website TEXT,
  phone TEXT,
  email TEXT,

  -- Location
  primary_address TEXT,
  latitude FLOAT,
  longitude FLOAT,

  -- Verification
  verified BOOL DEFAULT false,
  verified_at TIMESTAMPTZ,

  -- Claiming (for capacity updates)
  claimed_at TIMESTAMPTZ,
  claim_token TEXT UNIQUE,
  claim_email TEXT,

  organization_type TEXT CHECK (organization_type IN ('nonprofit', 'government', 'business', 'community', 'other')),

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_organizations_name ON organizations(name);
CREATE INDEX idx_organizations_domain ON organizations(domain_id);
CREATE INDEX idx_organizations_verified ON organizations(verified);
CREATE INDEX idx_organizations_claimed ON organizations(claimed_at) WHERE claimed_at IS NOT NULL;
```

### Listings (Base + Type-Specific)

```sql
-- Base listings table (hot path fields hardcoded)
CREATE TABLE listings (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  organization_id UUID REFERENCES organizations(id),
  organization_name TEXT NOT NULL,

  title TEXT NOT NULL,
  description TEXT NOT NULL,
  description_markdown TEXT,
  tldr TEXT,

  -- Hot path: core type (required, affects routing)
  listing_type TEXT NOT NULL CHECK (listing_type IN ('service', 'opportunity', 'business')),

  -- Hot path: primary navigation filter (food, housing, healthcare, legal, etc.)
  category TEXT NOT NULL,

  -- Hot path: affects visibility in search
  capacity_status TEXT DEFAULT 'accepting' CHECK (capacity_status IN ('accepting', 'paused', 'at_capacity')),

  -- Hot path: affects ordering and notifications
  urgency TEXT CHECK (urgency IN ('low', 'medium', 'high', 'urgent')),

  -- Hot path: lifecycle state
  status TEXT DEFAULT 'pending_approval' CHECK (status IN ('pending_approval', 'active', 'filled', 'rejected', 'expired')),

  -- Trust signal (date-based verification logic)
  verified_at TIMESTAMPTZ,

  source_language TEXT NOT NULL DEFAULT 'en',

  location TEXT,
  latitude FLOAT,
  longitude FLOAT,

  submitted_by_admin_id UUID,
  submission_type TEXT CHECK (submission_type IN ('scraped', 'admin', 'org_submitted')),

  domain_id UUID REFERENCES domains(id),
  source_url TEXT,
  last_seen_at TIMESTAMPTZ DEFAULT NOW(),
  disappeared_at TIMESTAMPTZ,

  embedding vector(1536),

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_listings_type ON listings(listing_type);
CREATE INDEX idx_listings_category ON listings(category);
CREATE INDEX idx_listings_organization ON listings(organization_id);
CREATE INDEX idx_listings_domain ON listings(domain_id);
CREATE INDEX idx_listings_status ON listings(status);
CREATE INDEX idx_listings_capacity ON listings(capacity_status);
CREATE INDEX idx_listings_urgency ON listings(urgency);
CREATE INDEX idx_listings_language ON listings(source_language);
CREATE INDEX idx_listings_verified ON listings(verified_at) WHERE verified_at IS NOT NULL;
CREATE INDEX idx_listings_embedding ON listings USING ivfflat(embedding vector_cosine_ops);

-- Service-specific properties
CREATE TABLE service_listings (
  listing_id UUID PRIMARY KEY REFERENCES listings(id) ON DELETE CASCADE,
  requires_id BOOL DEFAULT false,
  contacts_authorities BOOL DEFAULT false,
  avoids_facility_visit BOOL DEFAULT false,
  remote_ok BOOL DEFAULT false
);

CREATE INDEX idx_service_requires_id ON service_listings(requires_id);
CREATE INDEX idx_service_contacts_authorities ON service_listings(contacts_authorities);
CREATE INDEX idx_service_avoids_facility ON service_listings(avoids_facility_visit);

-- Opportunity-specific properties
CREATE TABLE opportunity_listings (
  listing_id UUID PRIMARY KEY REFERENCES listings(id) ON DELETE CASCADE,
  opportunity_type TEXT NOT NULL CHECK (opportunity_type IN ('volunteer', 'donation', 'customer', 'partnership', 'other')),
  time_commitment TEXT,
  requires_background_check BOOL DEFAULT false,
  minimum_age INT,
  skills_needed TEXT[],
  remote_ok BOOL DEFAULT false
);

CREATE INDEX idx_opportunity_type ON opportunity_listings(opportunity_type);

-- Business-specific properties (economic solidarity)
CREATE TABLE business_listings (
  listing_id UUID PRIMARY KEY REFERENCES listings(id) ON DELETE CASCADE,

  business_type TEXT,  -- 'restaurant', 'retail', 'service', 'grocery', etc.

  -- Support context (why they need help)
  support_needed TEXT[],  -- ['customers', 'donations', 'visibility', 'volunteers']
  current_situation TEXT,  -- "Owners temporarily in hiding" / "Reduced hours due to staff fears"

  -- How to support financially
  accepts_donations BOOL DEFAULT false,
  donation_link TEXT,  -- GoFundMe, Venmo, PayPal, etc.
  gift_cards_available BOOL DEFAULT false,
  gift_card_link TEXT,

  -- Accessibility/reach
  remote_ok BOOL DEFAULT false,  -- Can order online
  delivery_available BOOL DEFAULT false,
  online_ordering_link TEXT
);

CREATE INDEX idx_business_support_needed ON business_listings USING GIN(support_needed);

-- Shared: Delivery modes
CREATE TABLE listing_delivery_modes (
  listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
  delivery_mode TEXT NOT NULL CHECK (delivery_mode IN ('in_person', 'phone', 'online', 'mail', 'home_visit')),
  PRIMARY KEY (listing_id, delivery_mode)
);

-- Shared: Contact information
CREATE TABLE listing_contacts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
  contact_type TEXT NOT NULL CHECK (contact_type IN ('phone', 'email', 'website', 'address')),
  contact_value TEXT NOT NULL,
  contact_label TEXT,
  display_order INT DEFAULT 0
);

CREATE INDEX idx_listing_contacts_listing ON listing_contacts(listing_id);
```

### Universal Tagging System

```sql
-- Tags (flexible metadata for all entities)
CREATE TABLE tags (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  kind TEXT NOT NULL,
  value TEXT NOT NULL,
  display_name TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(kind, value)
);

CREATE INDEX idx_tags_kind ON tags(kind);
CREATE INDEX idx_tags_value ON tags(value);

-- Polymorphic tagging (works with any table)
CREATE TABLE taggables (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  taggable_type TEXT NOT NULL,  -- 'listing', 'organization', 'document', 'domain'
  taggable_id UUID NOT NULL,
  added_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(tag_id, taggable_type, taggable_id)
);

CREATE INDEX idx_taggables_tag ON taggables(tag_id);
CREATE INDEX idx_taggables_entity ON taggables(taggable_type, taggable_id);
CREATE INDEX idx_taggables_type ON taggables(taggable_type);

-- Example tags for MVP
INSERT INTO tags (kind, value, display_name) VALUES
  -- Community served (flexible metadata, not hot path)
  ('community_served', 'somali', 'Somali'),
  ('community_served', 'ethiopian', 'Ethiopian'),
  ('community_served', 'latino', 'Latino'),
  ('community_served', 'hmong', 'Hmong'),

  -- Service area
  ('service_area', 'minneapolis', 'Minneapolis'),
  ('service_area', 'st_paul', 'St. Paul'),

  -- Population served
  ('population', 'seniors', 'Seniors'),
  ('population', 'youth', 'Youth'),
  ('population', 'families', 'Families with Children'),

  -- Organization metadata
  ('org_leadership', 'community_led', 'Community-Led'),
  ('verification_source', 'admin_verified', 'Admin Verified'),
  ('verification_source', 'community_vouched', 'Community Vouched');
```

### Multi-Language System

```sql
-- Active languages (dynamic)
CREATE TABLE active_languages (
  language_code TEXT PRIMARY KEY,
  language_name TEXT NOT NULL,
  native_name TEXT NOT NULL,
  enabled BOOL DEFAULT true,
  added_at TIMESTAMPTZ DEFAULT NOW()
);

INSERT INTO active_languages (language_code, language_name, native_name) VALUES
  ('en', 'English', 'English'),
  ('es', 'Spanish', 'Español'),
  ('so', 'Somali', 'Soomaali');

-- Cached translations for listings
CREATE TABLE listing_translations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
  language_code TEXT NOT NULL REFERENCES active_languages(language_code),
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  tldr TEXT,
  translated_at TIMESTAMPTZ DEFAULT NOW(),
  translation_model TEXT DEFAULT 'gpt-4o',
  UNIQUE(listing_id, language_code)
);

CREATE INDEX idx_listing_translations_listing ON listing_translations(listing_id);
CREATE INDEX idx_listing_translations_language ON listing_translations(language_code);
```

### Conversational AI (Anonymous)

```sql
-- Chatrooms (anonymous - no auth required)
CREATE TABLE chatrooms (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  language TEXT DEFAULT 'en' REFERENCES active_languages(language_code),
  created_at TIMESTAMPTZ DEFAULT NOW(),
  last_activity_at TIMESTAMPTZ DEFAULT NOW()
);

-- Messages in chatrooms
CREATE TABLE messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  chatroom_id UUID NOT NULL REFERENCES chatrooms(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
  content TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  sequence_number INT NOT NULL
);

CREATE INDEX idx_messages_chatroom ON messages(chatroom_id, sequence_number);
```

### Referral Documents

```sql
-- Generated referral documents (markdown + components)
CREATE TABLE referral_documents (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  chatroom_id UUID REFERENCES chatrooms(id),

  -- Source language (language it was created in)
  source_language TEXT NOT NULL REFERENCES active_languages(language_code),

  -- Content (Markdown + JSX-like components)
  -- Example:
  -- # Resources for You
  -- <Listing id="abc-123" highlight="They speak Spanish" />
  -- <Map center="patient_location" zoom={12}>
  --   <Listing id="abc-123" />
  -- </Map>
  content TEXT NOT NULL,

  slug TEXT UNIQUE NOT NULL,
  title TEXT,
  status TEXT DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived')),

  -- Edit capability (no auth - just know the secret token)
  edit_token TEXT UNIQUE,

  view_count INT DEFAULT 0,
  last_viewed_at TIMESTAMPTZ,

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_documents_slug ON referral_documents(slug);
CREATE INDEX idx_documents_chatroom ON referral_documents(chatroom_id);
CREATE INDEX idx_documents_language ON referral_documents(source_language);

-- Translations for referral documents
CREATE TABLE referral_document_translations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  document_id UUID NOT NULL REFERENCES referral_documents(id) ON DELETE CASCADE,
  language_code TEXT NOT NULL REFERENCES active_languages(language_code),

  -- Translated content and title
  content TEXT NOT NULL,
  title TEXT,

  translated_at TIMESTAMPTZ DEFAULT NOW(),
  translation_model TEXT DEFAULT 'gpt-4o',

  UNIQUE(document_id, language_code)
);

CREATE INDEX idx_document_translations_document ON referral_document_translations(document_id);
CREATE INDEX idx_document_translations_language ON referral_document_translations(language_code);

-- Track referenced entities (for staleness detection)
CREATE TABLE document_references (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  document_id UUID NOT NULL REFERENCES referral_documents(id) ON DELETE CASCADE,
  reference_kind TEXT NOT NULL CHECK (reference_kind IN ('listing', 'organization', 'contact')),
  reference_id TEXT NOT NULL,
  referenced_at TIMESTAMPTZ DEFAULT NOW(),
  display_order INT DEFAULT 0,
  UNIQUE(document_id, reference_kind, reference_id)
);

CREATE INDEX idx_document_refs_document ON document_references(document_id);
CREATE INDEX idx_document_refs_kind_id ON document_references(reference_kind, reference_id);
```

### Key Design Decisions

**Anonymous by Design**:
- No auth required for chatrooms or document creation
- `edit_token` provides edit capability without accounts
- Anyone can browse listings, anyone can create documents

**Hybrid Approach (Hot Path + Tags)**:
- **Hardcoded fields** for hot path queries (category, capacity_status, urgency, fear constraints)
- **Universal tags** for flexible metadata (community_served, service_area, population)
- Hot path gets direct column access (fast), discovery gets tag flexibility
- Add new tag kinds without schema changes

**Type-Specific Tables**:
- Base `listings` table with common fields
- Separate tables for service/opportunity/business-specific properties
- Clean separation, no nullable field sprawl

**Multi-Language**:
- Dynamic language system - add languages without code changes
- Cached translations via seesaw commands: `TranslateRequest`, `BatchTranslate`
- Documents can be translated same as listings

**Verification & Capacity**:
- Simple `verified_at` timestamp (NULL = unverified)
- `capacity_status` for "we're full" signaling
- Verification badge: fresh (✅), stale (⚠️), unverified (none)

**Staleness Detection**:
- `document_references` tracks all entities in a document
- Can detect when referenced listings expire/change
- Show warning: "This document may be outdated"

---

## Database Schema Changes (MVP Only)

### Migration: 000031_mvp_fear_constraints.sql

```sql
-- MVP: Add only the 3 fear-specific constraints
ALTER TABLE organization_needs
  ADD COLUMN requires_id BOOL DEFAULT false,
  ADD COLUMN contacts_authorities BOOL DEFAULT false,
  ADD COLUMN avoids_facility_visit BOOL DEFAULT false;

-- Add service delivery modes (MVP: simple array)
ALTER TABLE organization_needs
  ADD COLUMN service_delivery_mode TEXT[] DEFAULT ARRAY['in_person'];
  -- Options: 'in_person', 'telehealth', 'home_visit', 'medication_delivery'

-- Add source language tracking
ALTER TABLE organization_needs
  ADD COLUMN source_language TEXT NOT NULL DEFAULT 'en';

-- Simple indexes for constraint filtering
CREATE INDEX idx_needs_requires_id ON organization_needs(requires_id) WHERE requires_id = false;
CREATE INDEX idx_needs_contacts_authorities ON organization_needs(contacts_authorities) WHERE contacts_authorities = false;
CREATE INDEX idx_needs_avoids_facility ON organization_needs(avoids_facility_visit) WHERE avoids_facility_visit = true;

COMMENT ON COLUMN organization_needs.requires_id IS 'MVP: Fear constraint - if true, exclude for undocumented patients';
COMMENT ON COLUMN organization_needs.contacts_authorities IS 'MVP: Fear constraint - if true, exclude for patients avoiding authorities';
COMMENT ON COLUMN organization_needs.avoids_facility_visit IS 'MVP: Fear constraint - if true, patient never has to visit facility';
```

### Migration: 000032_mvp_dynamic_translations.sql

```sql
-- Active languages table (dynamic language addition)
CREATE TABLE active_languages (
  language_code TEXT PRIMARY KEY,  -- ISO 639-1: 'en', 'es', 'so', etc.
  language_name TEXT NOT NULL,     -- 'English', 'Spanish', 'Somali'
  native_name TEXT NOT NULL,       -- 'English', 'Español', 'Soomaali'
  enabled BOOL DEFAULT true,
  added_at TIMESTAMPTZ DEFAULT NOW()
);

-- MVP: Start with 3 languages
INSERT INTO active_languages (language_code, language_name, native_name) VALUES
  ('en', 'English', 'English'),
  ('es', 'Spanish', 'Español'),
  ('so', 'Somali', 'Soomaali');

-- Translation table (works with any active language)
CREATE TABLE listing_translations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES organization_needs(id) ON DELETE CASCADE,
    language TEXT NOT NULL REFERENCES active_languages(language_code),

    -- Translated fields
    title TEXT NOT NULL,
    description TEXT NOT NULL,

    -- Translation metadata (crude is fine)
    translated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    needs_review BOOLEAN NOT NULL DEFAULT true,

    UNIQUE(listing_id, language)
);

CREATE INDEX idx_listing_translations_listing_id ON listing_translations(listing_id);
CREATE INDEX idx_listing_translations_language ON listing_translations(language);

COMMENT ON TABLE active_languages IS 'Dynamically add languages - system auto-translates all listings';
COMMENT ON TABLE listing_translations IS 'Translations for all active languages';
```

### Migration: 000033_mvp_chat_sessions.sql

```sql
-- Chat sessions (conversation history between healthcare worker and AI)
CREATE TABLE assist_sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL,  -- Healthcare worker
  user_type TEXT NOT NULL DEFAULT 'healthcare_worker',

  created_at TIMESTAMPTZ DEFAULT NOW(),
  last_activity_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_assist_sessions_user ON assist_sessions(user_id, last_activity_at DESC);

-- Chat messages (the conversation itself)
CREATE TABLE assist_messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID NOT NULL REFERENCES assist_sessions(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
  content TEXT NOT NULL,

  -- Tool calls made during this message (for AI messages)
  tool_calls JSONB,  -- [{tool: 'search_listings', args: {...}, result: {...}}]

  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_assist_messages_session ON assist_messages(session_id, created_at);

COMMENT ON TABLE assist_sessions IS 'Chat conversations between healthcare workers and AI';
COMMENT ON TABLE assist_messages IS 'Individual messages in a conversation';
```

### Migration: 000034_mvp_referral_documents.sql

```sql
-- Referral documents CREATED FROM chat sessions
CREATE TABLE referral_documents (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID NOT NULL REFERENCES assist_sessions(id) ON DELETE CASCADE,
  created_by_user_id UUID NOT NULL,  -- Healthcare worker

  -- Language (references active_languages, dynamically supports all enabled languages)
  language TEXT NOT NULL REFERENCES active_languages(language_code),

  -- Document content (rich DSL - markdown-like with embedded components)
  -- AI generates this, healthcare worker edits as plain text
  content TEXT NOT NULL,
    -- Example:
    -- # Resources for You
    --
    -- Hi, based on our conversation, here are services that can help.
    --
    -- {{listing id="abc-123" highlight="They do home visits"}}
    --
    -- I spoke with them yesterday and they have capacity.
    --
    -- {{listing id="def-456"}}
    --
    -- Please call me if you need help: (612) 555-1234

  -- Document state
  status TEXT DEFAULT 'draft' CHECK (status IN ('draft', 'finalized')),
  finalized_at TIMESTAMPTZ,

  -- Shareable URL (never expires - healthcare worker owns this document)
  slug TEXT UNIQUE NOT NULL,

  -- Simple tracking (views count only, no PII)
  view_count INT DEFAULT 0,

  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_referral_docs_slug ON referral_documents(slug);
CREATE INDEX idx_referral_docs_session ON referral_documents(session_id);
CREATE INDEX idx_referral_docs_user ON referral_documents(created_by_user_id, created_at DESC);
CREATE INDEX idx_referral_docs_language ON referral_documents(language);

COMMENT ON TABLE referral_documents IS 'PRIVATE 1:1 referral documents created by healthcare workers. NOT browsable/indexed - only accessible via private slug. These are personalized for specific patients, not a public directory.';
COMMENT ON COLUMN referral_documents.session_id IS 'The chat conversation that produced this document';
COMMENT ON COLUMN referral_documents.content IS 'Rich DSL (markdown variant) with embedded listing components';
COMMENT ON COLUMN referral_documents.slug IS 'Private URL - not indexed, not browsable, not searchable. Only shared 1:1 from healthcare worker to patient.';
```

---

## Referral Document Format (Markdown + Components)

**Approach**: Standard markdown + embedded components (like MDX). Custom parser for our needs.

### Standard Markdown
Use full markdown spec - no restrictions:
```markdown
# Heading 1
## Heading 2

**bold** *italic* ~~strikethrough~~

- Bullet lists
1. Numbered lists

> Blockquotes

[Links](https://example.com)

---

Tables, code blocks, etc.
```

### Embedded Components (JSX-like syntax)

**1. Listing Component**
```jsx
<Listing id="abc-123" />

<Listing id="abc-123" highlight="They do home visits" />

<Listing
  id="abc-123"
  highlight="I called them yesterday"
  show="contact,hours"
/>
```

**Props**:
- `id` (required): Listing ID
- `highlight`: Custom note
- `show`: Fields to display (comma-separated)

**Rendering**: Fetches listing + translation based on document language

---

**2. Callout Component**
```jsx
<Callout type="info">
Please call ahead to confirm they have capacity.
</Callout>

<Callout type="warning">
Bring your medical records if you have them.
</Callout>
```

**Props**: `type` - info, warning, success, error

---

**3. Contact Component**
```jsx
<Contact
  name="Dr. Sarah Smith"
  phone="(612) 555-1234"
  hours="Mon-Fri 9am-5pm"
/>
```

---

**4. Map Component** - Show listings on interactive map
```jsx
<Map center={[44.9778, -93.2650]} zoom={12}>
  <Listing id="abc-123" />
  <Listing id="def-456" />
  <Listing id="ghi-789" />
</Map>

<Map
  center="patient_location"
  zoom={10}
  showDistance={true}
>
  <Listing id="abc-123" />
  <Listing id="def-456" />
</Map>
```

**Props**:
- `center`: Coordinates `[lat, lng]` or `"patient_location"` (uses browser geolocation)
- `zoom`: Map zoom level (1-20)
- `showDistance`: Show distance from patient location to each listing

**Rendering**:
- Uses Leaflet or Mapbox
- Plots listings as pins on map
- Clicking pin shows listing details
- Shows distance if `showDistance={true}`

---

### Complete Example

```markdown
# Resources for You

Hi,

Based on our conversation today, I've found some services that can help with your situation. All of these services:
- Do NOT require ID
- Do NOT contact immigration authorities
- Offer home visits or telehealth

---

## Map of Nearby Services

Here's where these services are located:

<Map center="patient_location" zoom={12} showDistance={true}>
  <Listing id="abc-123" />
  <Listing id="def-456" />
  <Listing id="ghi-789" />
</Map>

---

## Home Healthcare

<Listing
  id="abc-123"
  highlight="I spoke with them yesterday - they have immediate availability for Somali-speaking patients"
/>

<Callout type="info">
They offer medication delivery too. Ask about their pharmacy program.
</Callout>

---

## Donation Centers Nearby

These are the closest places to donate or get donated items:

<Listing id="def-456" />

<Listing id="ghi-789" show="title,contact,hours" />

---

## Important Notes

- All services are free or sliding scale
- You can bring a family member to interpret
- If you have trouble accessing any of these, please call me

<Contact
  name="Dr. Sarah Smith"
  phone="(612) 555-1234"
  hours="Mon-Fri 9am-5pm"
/>

---

*This document was created for you on January 29, 2026. If you're viewing this more than a few weeks later, please call to confirm services are still available.*
```

### AI Tool Interface

**Tool: `create_document`**
```typescript
{
  tool: 'create_document',
  args: {
    language: 'so',
    content: `# Kheyraadka Adiga

Salaamu calaykum,

Ku saleysan sheekadayada maanta, waxaan helay adeegyo kaa caawin kara...

<Listing id="abc-123" highlight="Waxaan la hadlay iyaga shalay" />

<Callout type="info">
Dawooyinka guriga ayey keenaan.
</Callout>

<Contact name="Dr. Sarah Smith" phone="(612) 555-1234" />
`
  }
}
```

**Benefits for AI**:
- Standard markdown (familiar format)
- JSX-like components (clear structure)
- Easy to edit (healthcare worker can modify as plain text)
- Components handle data fetching (AI just provides IDs)

---

### Frontend Parsing & Rendering (Custom MDX-like Parser)

```typescript
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkReact from 'remark-react';

// Custom parser for our markdown + components
function parseReferralDocument(content: string, language: string) {
  const processor = unified()
    .use(remarkParse)  // Parse markdown
    .use(remarkCustomComponents)  // Handle <Listing>, <Callout>, <Contact>
    .use(remarkReact, {
      createElement: React.createElement,
      components: {
        // Map component tags to React components
        Listing: ListingCard,
        Callout: CalloutBox,
        Contact: ContactCard,
      },
    });

  // Parse content
  const ast = processor.parse(content);

  // Extract listing IDs to pre-fetch
  const listingIds = extractListingIds(ast);
  const listings = await fetchListingsWithTranslations(listingIds, language);

  // Render with data context
  return (
    <ListingContext.Provider value={listings}>
      {processor.stringify(ast)}
    </ListingContext.Provider>
  );
}

// Component implementations
function ListingCard({ id, highlight, show }) {
  const listings = useContext(ListingContext);
  const listing = listings[id];

  return (
    <div className="listing-card">
      <h3>{listing.title}</h3>
      <p>{listing.description}</p>
      {highlight && <p className="highlight">💡 {highlight}</p>}
      <div className="contact">
        📞 {listing.contact_info.phone}
        📍 {listing.location}
      </div>
    </div>
  );
}
```

---

### Editor Experience

Healthcare worker sees split view:
```
┌─────────────────────────────────────────────────────────────┐
│  Editor (Markdown)           │  Preview (Rendered)          │
├──────────────────────────────┼──────────────────────────────┤
│                              │                              │
│  # Resources for You         │  Resources for You           │
│                              │  ─────────────────────────   │
│  Hi, here are services...    │  Hi, here are services...    │
│                              │                              │
│  <Listing id="abc-123"       │  ┌─────────────────────────┐│
│    highlight="Great option"  │  │ Somali Home Health      ││
│  />                          │  │ ✅ VERIFIED             ││
│                              │  │                         ││
│  <Callout type="info">       │  │ Provides in-home care...││
│  Call ahead to confirm.      │  │                         ││
│  </Callout>                  │  │ 💡 Great option         ││
│                              │  │                         ││
│                              │  │ 📞 (612) 555-0123       ││
│                              │  └─────────────────────────┘│
│                              │                              │
│                              │  ℹ️ Call ahead to confirm.   │
│                              │                              │
└──────────────────────────────┴──────────────────────────────┘
```

- **Left**: Edit as markdown + JSX
- **Right**: Live preview with interactive components
- Changes sync in real-time

---

### Why Markdown + Components?

1. **Standard Format**: Markdown is familiar, widely supported
2. **AI-Friendly**: Easy to generate (just text, no complex JSON)
3. **Human-Editable**: Healthcare workers can edit as plain text
4. **Flexible**: Mix narrative text with structured data
5. **Interactive**: Components handle data fetching and translation
6. **Extensible**: Easy to add new component types
7. **Like MDX**: Proven pattern (MDX, Notion, Obsidian all use similar approach)

### Implementation: Custom Parser

Build lightweight custom parser (not full MDX dependency):
- Parse markdown with `unified` + `remark`
- Extract `<ComponentName prop="value">` tags
- Replace with React components
- Pre-fetch listing data for performance
- ~200 lines of code vs. heavy MDX dependency

```

**Post-MVP migrations** (not needed for direct impact):
- Rename `organization_needs` → `listings`
- Add verification fields
- Add capacity management
- Add provider throttling
- Add verification decay cron

---

-- Rename all indexes
ALTER INDEX idx_organization_needs_status RENAME TO idx_listings_status;
ALTER INDEX idx_organization_needs_content_hash RENAME TO idx_listings_content_hash;
ALTER INDEX idx_organization_needs_source_id RENAME TO idx_listings_source_id;
ALTER INDEX idx_organization_needs_last_seen RENAME TO idx_listings_last_seen;
ALTER INDEX idx_organization_needs_submitted_by RENAME TO idx_listings_submitted_by;
ALTER INDEX idx_organization_needs_source_url RENAME TO idx_listings_source_url;
ALTER INDEX idx_needs_embedding RENAME TO idx_listings_embedding;

-- Listing types (removed business_support, event for MVP)
ALTER TABLE listings
  ADD COLUMN listing_type TEXT NOT NULL DEFAULT 'volunteer_opportunity'
    CHECK (listing_type IN (
      'volunteer_opportunity',
      'service_offered',
      'donation_request'
    ));

-- Service-specific fields
ALTER TABLE listings
  ADD COLUMN resource_category TEXT,
  ADD COLUMN eligibility_criteria TEXT,
  ADD COLUMN required_documents TEXT[],
  ADD COLUMN hours_of_operation TEXT,
  ADD COLUMN walk_ins_accepted BOOL,
  ADD COLUMN appointment_required BOOL,
  ADD COLUMN languages_available TEXT[],
  ADD COLUMN cost TEXT,
  ADD COLUMN serves_area TEXT;

-- SAFETY: Explicit constraints (NOT just semantic)
ALTER TABLE listings
  ADD COLUMN requires_id BOOL DEFAULT false,
  ADD COLUMN requires_proof_of_residency BOOL DEFAULT false,
  ADD COLUMN requires_income_verification BOOL DEFAULT false,
  ADD COLUMN immigration_status_accepted TEXT[] DEFAULT ARRAY['all'],
  ADD COLUMN service_restrictions TEXT[];  -- ['contacts_authorities', 'mandatory_reporting', 'religious_required']

-- SAFETY: Verification and confidence
ALTER TABLE listings
  ADD COLUMN verification_status TEXT DEFAULT 'unverified'
    CHECK (verification_status IN ('verified', 'unverified', 'community_reported')),
  ADD COLUMN last_verified_at TIMESTAMPTZ,
  ADD COLUMN verified_by_admin_id UUID;

-- Provider type and throttling
ALTER TABLE listings
  ADD COLUMN provider_type TEXT DEFAULT 'organization'
    CHECK (provider_type IN ('organization', 'individual', 'network')),
  ADD COLUMN max_referrals_per_week INT,
  ADD COLUMN referral_count_this_week INT DEFAULT 0,
  ADD COLUMN referral_method TEXT DEFAULT 'direct'
    CHECK (referral_method IN ('direct', 'warm_intro_only', 'waitlist'));

-- Capacity management
ALTER TABLE listings
  ADD COLUMN capacity_status TEXT DEFAULT 'accepting'
    CHECK (capacity_status IN ('accepting', 'waitlist', 'paused', 'at_capacity')),
  ADD COLUMN capacity_notes TEXT,
  ADD COLUMN capacity_updated_at TIMESTAMPTZ;

-- Intake priority (org decides, we surface it)
ALTER TABLE listings
  ADD COLUMN intake_priority TEXT,
  ADD COLUMN referral_process TEXT;

-- Geography and remote services
ALTER TABLE listings
  ADD COLUMN service_delivery_mode TEXT[] DEFAULT ARRAY['in_person'],
  ADD COLUMN jurisdictions_served TEXT[],
  ADD COLUMN remote_eligible BOOL DEFAULT false;

-- Featured/priority
ALTER TABLE listings
  ADD COLUMN priority INT DEFAULT 0,
  ADD COLUMN featured_until TIMESTAMPTZ;

-- Indexes for safety queries
CREATE INDEX idx_listings_type_status ON listings(listing_type, status);
CREATE INDEX idx_listings_verification ON listings(verification_status, last_verified_at DESC)
  WHERE listing_type = 'service_offered';
CREATE INDEX idx_listings_constraints_id ON listings(requires_id)
  WHERE listing_type = 'service_offered' AND requires_id = false;
CREATE INDEX idx_listings_constraints_immigration ON listings USING GIN(immigration_status_accepted)
  WHERE listing_type = 'service_offered';
CREATE INDEX idx_listings_provider_throttle ON listings(provider_type, referral_count_this_week)
  WHERE provider_type = 'individual';
CREATE INDEX idx_listings_delivery_mode ON listings USING GIN(service_delivery_mode)
  WHERE listing_type = 'service_offered';

COMMENT ON TABLE listings IS 'Universal listings: services, volunteer opportunities, donation requests. NOT a marketplace, a referral coordination tool.';
COMMENT ON COLUMN listings.verification_status IS 'Confidence level: verified (admin confirmed <30d), unverified (scraped/pending), community_reported (user submit)';
COMMENT ON COLUMN listings.requires_id IS 'SAFETY: If true, exclude for undocumented patients';
COMMENT ON COLUMN listings.immigration_status_accepted IS 'SAFETY: Which statuses accepted (all, documented, undocumented, refugee, asylum)';
COMMENT ON COLUMN listings.service_restrictions IS 'SAFETY: Exclusions (contacts_authorities, mandatory_reporting, religious_required)';
```

### Migration: 000032_create_assist_domain_with_referral_documents.sql

```sql
-- AI assistant sessions (for conversation history)
CREATE TABLE assist_sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_type TEXT NOT NULL DEFAULT 'healthcare_worker'
    CHECK (user_type IN ('healthcare_worker', 'volunteer', 'public')),
  user_id UUID,  -- Healthcare worker identifier
  created_at TIMESTAMPTZ DEFAULT NOW(),
  last_activity_at TIMESTAMPTZ DEFAULT NOW(),
  metadata JSONB
);

CREATE INDEX idx_assist_sessions_user ON assist_sessions(user_id, last_activity_at DESC)
  WHERE user_id IS NOT NULL;

-- Messages in session (for chat history)
CREATE TABLE assist_messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID NOT NULL REFERENCES assist_sessions(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
  content TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_assist_messages_session ON assist_messages(session_id, created_at);

-- Referral documents (editable, shareable, no expiration, multi-language)
CREATE TABLE referral_documents (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID NOT NULL REFERENCES assist_sessions(id) ON DELETE CASCADE,
  created_by_user_id UUID NOT NULL,  -- Healthcare worker who created this

  -- Multi-language support (CRITICAL)
  language TEXT DEFAULT 'en',  -- 'en', 'es', 'so', 'om', 'am' (ISO 639-1)
  auto_translated BOOL DEFAULT false,  -- True if machine-translated vs human-written

  -- Document content (editable rich text, in patient's language)
  title TEXT DEFAULT 'Resources for You',
  header_note TEXT,  -- Personal intro from healthcare worker (markdown/rich text)
  footer_note TEXT,  -- Closing note, healthcare worker contact info (markdown/rich text)

  -- Included services (can be removed, reordered, customized)
  included_listings JSONB NOT NULL,  -- Array of {listing_id, custom_note, rank, listing_snapshot}

  -- Document state
  status TEXT DEFAULT 'draft' CHECK (status IN ('draft', 'finalized', 'updated')),
  finalized_at TIMESTAMPTZ,
  last_edited_at TIMESTAMPTZ DEFAULT NOW(),

  -- Shareable URL (never expires - healthcare worker took ownership)
  slug TEXT UNIQUE NOT NULL,  -- e.g., 'dr-smith-resources-abc123'

  -- Tracking (aggregate only, no patient PII)
  view_count INT DEFAULT 0,
  last_viewed_at TIMESTAMPTZ,

  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_referral_docs_user ON referral_documents(created_by_user_id, created_at DESC);
CREATE INDEX idx_referral_docs_slug ON referral_documents(slug) WHERE status IN ('finalized', 'updated');
CREATE INDEX idx_referral_docs_session ON referral_documents(session_id);

-- Document view log (for analytics, no PII)
CREATE TABLE referral_document_views (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  document_id UUID NOT NULL REFERENCES referral_documents(id) ON DELETE CASCADE,
  viewed_at TIMESTAMPTZ DEFAULT NOW(),
  user_agent TEXT,
  referrer TEXT
);

CREATE INDEX idx_referral_doc_views_document ON referral_document_views(document_id, viewed_at DESC);

COMMENT ON TABLE referral_documents IS 'Editable referral documents created by healthcare workers. No expiration - worker approved content by editing.';
COMMENT ON COLUMN referral_documents.included_listings IS 'JSONB array: [{listing_id, custom_note, rank, listing_snapshot: {title, hours, contact, verification_status}}]';
COMMENT ON COLUMN referral_documents.slug IS 'Permanent shareable URL (no expiration). Healthcare worker can update document anytime.';
COMMENT ON COLUMN referral_documents.status IS 'draft (being edited), finalized (shared with patient), updated (edited after initial share)';
```

### Migration: 000033_verification_decay_cron.sql

```sql
-- Function to decay stale verifications
CREATE OR REPLACE FUNCTION decay_stale_verifications()
RETURNS void AS $$
BEGIN
  -- Mark verified listings as unverified if >30 days old
  UPDATE listings
  SET verification_status = 'unverified'
  WHERE verification_status = 'verified'
    AND last_verified_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- Schedule weekly decay check (using pg_cron or scheduled task)
-- SELECT cron.schedule('decay-verifications', '0 0 * * 0', 'SELECT decay_stale_verifications()');

COMMENT ON FUNCTION decay_stale_verifications IS 'Auto-downgrade verified → unverified after 30 days';
```

---

## Acceptance Criteria

### MVP Acceptance Criteria (Phase 1 Only)

**The Core Flow (Must Work Flawlessly)**:
- [ ] Healthcare worker describes patient needs in text box
- [ ] Healthcare worker checks constraint boxes (undocumented, avoid authorities, avoid facilities)
- [ ] Healthcare worker selects language (English or Somali)
- [ ] AI returns 3-5 services that pass ALL constraints OR zero-match response
- [ ] Healthcare worker edits draft document (add notes, remove services)
- [ ] Healthcare worker finalizes and gets permanent shareable link
- [ ] Patient opens link and sees document in selected language
- [ ] Patient can print or save document

**Fear-Specific Constraints (Must Work Perfectly)**:
- [ ] `requires_id=false` filter excludes services requiring ID
- [ ] `contacts_authorities=false` filter excludes services that report
- [ ] `avoids_facility_visit=true` shows only telehealth/home/delivery options
- [ ] Constraint violations logged (for auditing safety)
- [ ] Zero false positives (no unsafe services slip through)

**Somali Translation (Must Work)**:
- [ ] Listing created in English → auto-translates to Somali within 5 seconds
- [ ] Language toggle switches UI between English/Somali
- [ ] Viewing listing in Somali returns Somali translation from database
- [ ] Contact info (phone, email) preserved unchanged
- [ ] Machine translation indicator shown ("Auto-translated to Somali")
- [ ] Translation cost: <$0.01 per listing

**Zero-Match Response (Must Be Clear)**:
- [ ] When no services pass constraints, AI explains why
- [ ] Shows what was found but excluded (e.g., "3 services require ID")
- [ ] Offers next steps (expand radius, call hotline)
- [ ] No "Generate Document" button shown

**NOT Required for MVP**:
- ❌ Organization self-service portal
- ❌ Verification badges
- ❌ Analytics dashboards
- ❌ Spanish/Oromo/Amharic
- ❌ Fancy translation review
- ❌ Provider throttling
- ❌ Capacity management
- ❌ Perfect AI parsing

### Safety Requirements

**Constraint Filtering**:
- [ ] If patient undocumented, EXCLUDE services with requires_id=true
- [ ] If patient avoids authorities, EXCLUDE services with 'contacts_authorities' restriction
- [ ] Constraint violations logged and audited
- [ ] Semantic similarity does NOT override constraints

**Document Ownership (Temporal Truth)**:
- [ ] Healthcare worker explicitly approves content by editing
- [ ] Document shows "Created by [Provider Name] on [Date]"
- [ ] Links never expire (healthcare worker took ownership)
- [ ] Worker can update document anytime (living document)
- [ ] Updated documents show "Last updated [Date]"
- [ ] No "stale" warnings (we trust healthcare worker's judgment)

**Verification**:
- [ ] Verified listings show ✅ badge + last_verified_at
- [ ] Unverified listings show ⚠️ badge + "Confirm before referring"
- [ ] Community listings show ❓ badge + "Verify independently"
- [ ] Verification decays after 30 days (auto-downgrade to unverified)

**Provider Protection**:
- [ ] Individual providers can set max_referrals_per_week
- [ ] When limit reached, listing hidden from search
- [ ] Referral count resets weekly
- [ ] Warm-intro-only mode hides from public search

### Non-Functional Requirements

**Performance**:
- [ ] Constraint-filtered search: < 500ms
- [ ] AI recommendation generation: < 3 seconds
- [ ] Page load (public directory): < 2 seconds
- [ ] Capacity updates: visible within 5 seconds

**Security**:
- [ ] No patient PII stored (workers describe generically)
- [ ] Healthcare worker sessions don't log patient names
- [ ] Organizations can only edit their own listings
- [ ] Admin approval required for all new listings
- [ ] Rate limiting on public API

**Usability**:
- [ ] Mobile-first responsive design
- [ ] Clear disclaimer on every page
- [ ] Confidence badges visible at-a-glance
- [ ] Staleness warnings prominent

---

## Success Metrics (MVP Focus)

### Week 1 Post-Launch
**The only metric that matters**: Can sister use this in one sitting to help a scared patient in their language?

Quantitative (secondary):
- Sister creates 3+ referral documents
- 1+ document in Spanish, 1+ in Somali (validates multi-language need)
- 0 constraint violations (safety check)
- At least 1 real patient accesses care they would have avoided

### Week 2-4
**The only metric that matters**: Are 3+ healthcare workers using this regularly?

Quantitative (secondary):
- 3-5 healthcare workers active
- 10+ referral documents created
- 50%+ of documents in Somali (validates language need)
- Constraint filtering works (no false positives reported)
- Qualitative feedback: "This saved me time" or "Patient got help"

### Month 1-3
**The only metric that matters**: Does this reliably help scared patients access care?

Quantitative (secondary):
- 5+ healthcare workers active monthly
- 30+ referral documents created
- 20+ services listed
- 0 major safety incidents (wrong service recommended)
- Qualitative: Stories of patients who got care they would have skipped


## Dependencies & Risks

### Dependencies
- ✅ Existing seesaw-rs event-driven architecture
- ✅ PostgreSQL with pgvector
- ✅ OpenAI API (GPT-4o for AI tools + conversation, text-embedding-3-small for search)
- ✅ Twilio Verify (OTP)
- ✅ Admin approval workflow
- React + Vite (new public web app)
- **LiveKit** (real-time voice chat with AI agent)
  - LiveKit Cloud or self-hosted server
  - LiveKit React SDK for client
  - Speech-to-text (Whisper or Deepgram)
  - Text-to-speech (OpenAI TTS or ElevenLabs)
- Hosting (Vercel/Netlify)

### Risks

**High Priority**:
1. **Constraint violations** (undocumented patient sent to ID-required service)
   - **Mitigation**: Hard filters in SQL, logged and audited, testing with realistic scenarios

2. **Healthcare worker liability** (bad referral, outdated info)
   - **Mitigation**: Confidence badges, temporal validity warnings, explicit disclaimers

3. **Stale information** (capacity changes after link generation)
   - **Mitigation**: Temporal validity, staleness warnings, verification decay

**Medium Priority**:
4. **Organization adoption** (won't self-manage)
   - **Mitigation**: Pilot with 1-2 orgs, admin can manage on their behalf

5. **Individual provider overwhelm** (pro-bono lawyer flooded)
   - **Mitigation**: Exposure limits (max N/week), warm-intro-only mode

**Low Priority**:
6. **AI recommendation quality** (poor recommendations)
   - **Mitigation**: Constraint-first ranking, worker feedback loop, A/B test prompts

---

## Future Considerations (Post-MVP)

**Phase 5: Advanced Safety Features**
- Cross-check with public databases (org still exists, license valid)
- Automated phone verification (call to confirm capacity weekly)
- Additional languages beyond Phase 1 (e.g., Arabic, Hmong, Vietnamese)

**Phase 6: Feedback Loop**
- Healthcare workers can flag outdated info
- Organizations can see which services referred to them most
- Aggregate "success rate" (not individual tracking)

**Phase 7: Network Effects**
- Organizations can "refer out" (know similar services)
- Warm intros between orgs
- Capacity coordination (if A full, auto-suggest B)

**NOT in Scope** (MVP):
- ❌ Donation requests (future phase)
- ❌ Immigrant business directory (future phase)
- ❌ Community events (future phase)
- ❌ Outcome tracking
- ❌ Case management
- ❌ Appointment booking

---

## References

### Internal
- `/docs/DOMAIN_ARCHITECTURE.md` - Seesaw-rs pattern
- `/docs/SEESAW_ARCHITECTURE.md` - Event-driven framework
- `/docs/NEED_SYNCHRONIZATION.md` - Content hash deduplication
- `/docs/AUTHENTICATION_SECURITY.md` - Multi-user auth

### External
- pgvector: https://github.com/pgvector/pgvector
- OpenAI Embeddings: https://platform.openai.com/docs/guides/embeddings
- GPT-4o: https://platform.openai.com/docs/guides/chat

### Related Platforms
- findhelp.org (Aunt Bertha) - US resource directory
- 211 (findhelp211.org) - Crisis helpline

**Key Differentiators**:
- ✅ Constraint-based filtering (not just semantic search)
- ✅ Confidence signaling (verified/unverified/community)
- ✅ Editable referral documents (worker approval through editing)
- ✅ Explicit avoidance modeling (patient agency)
- ✅ Provider protection (exposure limits for individuals)
- ✅ NOT a marketplace (referral coordination only)

---

## Next Steps

1. **Review with sister** (healthcare worker perspective on safety gaps)
2. **Walk through one scenario** (Somali undocumented patient) to validate constraint logic
3. **Begin Phase 1** (database schema + safety fields)
4. **Set up feedback loop** (healthcare workers test early, report constraint violations)

**Estimated Timeline**: 8 weeks to MVP
**Team**: 1-2 developers
**Budget**: $100/month (OpenAI, hosting)

---

**Last Updated**: 2026-01-29
**Revision**: 6 (dynamic multi-language system + conversational AI + LiveKit voice)
**Status**: Ready for implementation

**Core Principle**: One patient who speaks Spanish/Somali and is afraid to leave home gets care they would have skipped = MVP success

**Interaction Model**: Healthcare worker talks to AI (voice or text), AI searches and updates document in real-time, worker edits and shares

**Language System**: MVP ships with English + Spanish + Somali. Adding a new language is as simple as telling AI "add Dutch" → system auto-translates all existing + future listings
