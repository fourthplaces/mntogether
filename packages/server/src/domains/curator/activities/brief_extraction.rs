use anyhow::Result;
use extraction::types::page::CachedPage;
use tracing::{debug, info};

use crate::domains::curator::models::PageBriefExtraction;
use crate::kernel::{ServerDeps, GPT_5_MINI};

const PAGE_BRIEF_PROMPT: &str = r#"
You are extracting critical information from a web page belonging to a community organization.

Extract ONLY factual information present on the page. Do not infer or fabricate.

## Fields to Extract

- **summary**: 2-3 sentence overview of what this page tells us about the organization.

- **locations**: All physical addresses mentioned. Include full addresses.

- **calls_to_action**: Urgent needs and requests — donation drives, volunteer asks,
  sign-up opportunities, supply needs. Be specific about what's needed and deadlines.

- **critical_info**: Operating hours, eligibility requirements, deadlines, closures,
  capacity limits, waitlist info. Anything someone needs to know before showing up.
  **For eligibility restrictions (citizenship, residency, ID, age, registration),
  state clearly WHO the restriction applies to.** Different activities on the same page
  may have different restrictions. For example: "Volunteers: must be 18+, US citizens
  or legal residents. Delivery recipients: must register online, US citizens or legal
  residents only at this time. In-person pickup: open to all, no ID required."
  Never lump restrictions together — separate them by audience/activity.

- **services**: Programs, services, or opportunities offered by name.

- **contacts**: EVERY contact method found on the page. Be thorough:
  - Phone numbers (with labels: "main", "after-hours", "hotline")
  - Email addresses (with labels: "info", "intake", "referrals")
  - Website URLs for specific actions (intake forms, booking pages, sign-up links)
  - Physical addresses for in-person visits
  - Include the label/context for each (e.g., "Booking: https://...")

- **schedules**: ALL temporal information. Be precise about the pattern:
  - **Operating hours**: "Monday-Friday 9am-5pm" → schedule_type: "operating_hours"
  - **Recurring events**: "Every 2nd Tuesday at 6pm" → schedule_type: "recurring"
  - **One-off events**: "March 15, 2026 at 2pm" → schedule_type: "event"
  - **Seasonal patterns**: "September through May" → schedule_type: "seasonal"
  - Include days, times, dates, frequency, and any exceptions
    ("closed holidays", "1st and 3rd week only", "by appointment only")

- **languages_mentioned**: Languages services are offered in (e.g., "Spanish",
  "Somali", "Karen", "Hmong"). Look for "multilingual", "interpreter available", etc.

- **populations_mentioned**: Target populations served — "refugees", "immigrants",
  "asylum seekers", "seniors", "youth", "families", "unaccompanied minors", etc.

- **capacity_info**: Current capacity status if mentioned — "accepting new clients",
  "waitlist", "at capacity", "not accepting donations at this time", etc.

If a field has no relevant information on the page, return an empty list/null.
Be concise but thorough. Capture all specifics, especially for schedules and contacts.
"#;

/// Extract a brief from a single page, memoized by content.
/// If the same page content was briefed before, returns cached result.
pub async fn extract_page_brief(
    page_url: &str,
    page_content: &str,
    organization_name: &str,
    deps: &ServerDeps,
) -> Result<Option<PageBriefExtraction>> {
    if page_content.trim().len() < 100 {
        return Ok(None);
    }

    let content = truncate_safe(page_content, 50_000);
    let user_prompt = format!(
        "Organization: {}\nPage URL: {}\n\n---\n\n{}",
        organization_name, page_url, content
    );

    // Memo key includes system prompt + user prompt (page content).
    // Same content + same prompt → cache hit.
    // Content changes OR prompt changes → cache miss → new LLM call.
    // 30-day TTL is just garbage collection for pages no longer crawled.
    let brief: PageBriefExtraction = deps
        .memo("page_brief_v2", (PAGE_BRIEF_PROMPT, &user_prompt))
        .ttl(2_592_000_000) // 30 days in ms
        .get_or(|| async {
            deps.ai
                .extract::<PageBriefExtraction>(GPT_5_MINI, PAGE_BRIEF_PROMPT, &user_prompt)
                .await
                .map_err(Into::into)
        })
        .await?;

    // Filter out empty briefs
    if brief.summary.trim().is_empty()
        && brief.calls_to_action.is_empty()
        && brief.services.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(brief))
}

/// Extract briefs for all pages with bounded concurrency, with memo-based caching.
pub async fn extract_briefs_for_org(
    org_name: &str,
    pages: &[CachedPage],
    deps: &ServerDeps,
) -> Result<Vec<(String, PageBriefExtraction)>> {
    use futures::stream::{self, StreamExt};

    const MAX_CONCURRENT: usize = 10;

    debug!(
        org = org_name,
        page_count = pages.len(),
        "Extracting page briefs (max {} concurrent)",
        MAX_CONCURRENT,
    );

    let futures: Vec<_> = pages.iter().map(|page| {
        let org_name = org_name.to_string();
        let url = page.url.clone();
        let content = page.content.clone();
        async move {
            let brief = extract_page_brief(&url, &content, &org_name, deps).await?;
            Ok::<_, anyhow::Error>(brief.map(|b| (url, b)))
        }
    }).collect();

    let briefs: Vec<_> = stream::iter(futures)
    .buffer_unordered(MAX_CONCURRENT)
    .filter_map(|r| async {
        match r {
            Ok(Some(pair)) => Some(pair),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("Page brief extraction failed: {}", e);
                None
            }
        }
    })
    .collect()
    .await;

    // Log each brief so we can see what the LLM extracted
    for (url, brief) in &briefs {
        info!(
            org = org_name,
            url = url.as_str(),
            summary = brief.summary.as_str(),
            capacity_info = brief.capacity_info.as_deref().unwrap_or("none"),
            critical_info = brief.critical_info.as_deref().unwrap_or("none"),
            calls_to_action_count = brief.calls_to_action.len(),
            "Page brief extracted"
        );
    }

    debug!(
        org = org_name,
        briefs_extracted = briefs.len(),
        "Page brief extraction complete"
    );

    Ok(briefs)
}

/// Truncate a string at a char boundary, never panicking on multi-byte UTF-8.
fn truncate_safe(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
