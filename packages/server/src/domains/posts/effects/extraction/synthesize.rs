//! Pass 2: Synthesize listings from all page summaries
//!
//! Combines summaries from all pages of a website to extract posts.
//! One listing per distinct organization/program - services and opportunities
//! are captured as tags, not separate posts.

use anyhow::Result;
use tracing::info;

use crate::kernel::{BaseAI, LlmRequestExt};

use super::types::{ExtractedPost, SynthesisInput};

/// Synthesize listings from all page summaries for a website
pub async fn synthesize_posts(
    input: SynthesisInput,
    ai: &dyn BaseAI,
) -> Result<Vec<ExtractedPost>> {
    if input.pages.is_empty() {
        return Ok(vec![]);
    }

    info!(
        domain = %input.website_domain,
        pages = input.pages.len(),
        "Synthesizing listings from page summaries"
    );

    let system = SYNTHESIS_SYSTEM_PROMPT;

    // Build combined page content
    let mut pages_text = String::new();
    for page in &input.pages {
        pages_text.push_str(&format!(
            "\n--- PAGE: {} ---\n{}\n",
            page.url, page.content
        ));
    }

    let user = format!(
        "Website: {}\n\nPage Summaries:\n{}",
        input.website_domain, pages_text
    );

    let posts: Vec<ExtractedPost> = ai
        .request()
        .system(system)
        .user(user)
        .schema_hint(SYNTHESIS_SCHEMA)
        .max_retries(3)
        .output()
        .await?;

    info!(
        domain = %input.website_domain,
        listings_count = posts.len(),
        "Synthesis complete"
    );

    Ok(posts)
}

const SYNTHESIS_SYSTEM_PROMPT: &str = r#"You are analyzing page summaries from a single website to extract posts.

TASK: Identify distinct programs and services. Create SEPARATE listings for each distinct program.

IMPORTANT: Create separate listings when programs have:
- Different names (e.g., "Food Shelf" vs "StyleXchange" vs "Emergency Assistance")
- Different hours of operation
- Different eligibility requirements
- Different services provided

Do NOT combine everything into one generic organization post. Users need to find specific programs.

For each listing provide:

1. title: Clear name of the organization or program (5-10 words)

2. tldr: 1-2 sentence summary of what they do and who they help

3. description: Full details including:
   - What services/programs they offer
   - Who is eligible / who they serve
   - How to access services or get involved
   - Hours of operation if mentioned
   - Any requirements or application process

4. contact: Extract if found
   - phone: Phone number
   - email: Email address
   - website: Website URL (only if different from source)

5. location: Physical address or service area (e.g., "Minneapolis, MN", "Twin Cities metro area", "123 Main St, St Paul")

6. tags: Categorize thoroughly using these tag kinds:

   audience_role - who engages with this listing:
   - "recipient" - people receiving services/help
   - "volunteer" - people giving time
   - "donor" - people giving money/goods
   - "customer" - people buying products/services
   - "job-seeker" - people looking for employment
   - "participant" - people attending events/classes

   population - who it specifically serves:
   - "disabilities" - people with disabilities
   - "brain-injury" - people with brain injuries
   - "seniors" - elderly/older adults
   - "refugees" - refugees
   - "immigrants" - immigrants
   - "youth" - children/young people
   - "families" - families with children
   - "veterans" - military veterans
   - "homeless" - people experiencing homelessness

   community_served - cultural/ethnic community:
   - "somali", "hmong", "karen", "latino", "east-african", etc.

   service_offered - what's provided:
   - "legal-aid" - legal services
   - "immigration" - immigration assistance
   - "food-assistance" - food shelves, meals
   - "housing" - housing assistance
   - "transportation" - rides, transit help
   - "disability-services" - disability support
   - "life-skills" - independent living skills
   - "language-classes" - ESL, language learning
   - "job-training" - employment training
   - "mental-health" - counseling, therapy
   - "healthcare" - medical services
   - "childcare" - childcare services
   - "financial-skills" - financial education
   - "citizenship" - citizenship preparation

   post_type - category of listing:
   - "service" - a service provided
   - "business" - a business to support
   - "event" - an event, class, or workshop
   - "fundraiser" - a fundraising event, gala, or benefit
   - "opportunity" - volunteer/donation opportunity

   org_leadership - who runs it:
   - "immigrant-owned" - immigrant-owned business
   - "refugee-owned" - refugee-owned business
   - "woman-owned" - woman-owned business
   - "nonprofit" - nonprofit organization

   service_area - geographic coverage:
   - "twin-cities" - Minneapolis/St. Paul metro
   - "st-cloud" - St. Cloud area
   - "rochester" - Rochester area
   - "statewide" - all of Minnesota

7. source_urls: List ALL page URLs that contributed information to this listing

RULES:
- Create SEPARATE listings for each distinct program/service (e.g., Food Shelf, Clothing Closet, Emergency Assistance)
- Do NOT create just one generic listing for the whole organization
- Each program with its own name, hours, or services should be its own listing
- Include ALL relevant tags - be thorough
- Every listing must have at least one audience_role and one service_offered tag
- source_urls should include every page that mentions this program

EXAMPLE: A nonprofit website might have:
- "Valley Outreach Food Shelf" (food distribution, Mon-Fri hours)
- "StyleXchange Clothing Program" (free clothing, different hours)
- "Emergency Assistance Program" (financial help, by appointment)
These should be 3 SEPARATE listings, not combined into one."#;

const SYNTHESIS_SCHEMA: &str = r#"Return a JSON array of posts:
[{
  "title": "string - organization/program name",
  "tldr": "string - 1-2 sentence summary",
  "description": "string - full details",
  "contact": {
    "phone": "string or null",
    "email": "string or null",
    "website": "string or null"
  },
  "location": "string or null - physical address or service area",
  "tags": [{
    "kind": "audience_role|population|community_served|service_offered|post_type|org_leadership|service_area",
    "value": "string - the tag value",
    "display_name": "string or null - human readable name"
  }],
  "source_urls": ["string - page URLs"]
}]"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesis_prompt_not_empty() {
        assert!(!SYNTHESIS_SYSTEM_PROMPT.is_empty());
        assert!(!SYNTHESIS_SCHEMA.is_empty());
    }
}
