use crate::common::{WebsiteId, JobId, MemberId};
use crate::domains::domain_approval::commands::DomainApprovalCommand;
use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::domains::listings::effects::deps::ServerDeps;
use crate::domains::scraping::models::{
    Website, WebsiteAssessment, WebsiteResearch, WebsiteResearchHomepage, TavilySearchQuery,
    TavilySearchResult,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::info;
use uuid::Uuid;

/// Assessment Effect - Handles generating AI assessments from research data
///
/// This effect is a thin orchestration layer that dispatches to handler functions.
pub struct AssessmentEffect;

#[async_trait]
impl Effect<DomainApprovalCommand, ServerDeps> for AssessmentEffect {
    type Event = DomainApprovalEvent;

    async fn execute(
        &self,
        cmd: DomainApprovalCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<DomainApprovalEvent> {
        match cmd {
            DomainApprovalCommand::GenerateAssessmentFromResearch {
                research_id,
                website_id,
                job_id,
                requested_by,
            } => {
                handle_generate_assessment_from_research(
                    research_id,
                    website_id,
                    job_id,
                    requested_by,
                    &ctx,
                )
                .await
            }
            _ => anyhow::bail!("AssessmentEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Handler Functions (Business Logic)
// ============================================================================

async fn handle_generate_assessment_from_research(
    research_id: Uuid,
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<DomainApprovalEvent> {
    info!(
        research_id = %research_id,
        website_id = %website_id,
        job_id = %job_id,
        "Generating assessment from research"
    );

    // Step 1: Load website
    let website = Website::find_by_id(website_id.into(), &ctx.deps().db_pool)
        .await
        .context(format!("Website not found: {}", website_id))?;

    // Step 2: Load research data
    let homepage = WebsiteResearchHomepage::find_by_research_id(research_id, &ctx.deps().db_pool)
        .await
        .context("Failed to load homepage")?;

    let queries = TavilySearchQuery::find_by_research_id(research_id, &ctx.deps().db_pool)
        .await
        .context("Failed to load search queries")?;

    let mut all_results = Vec::new();
    for query in &queries {
        let results = TavilySearchResult::find_by_query_id(query.id, &ctx.deps().db_pool)
            .await
            .context("Failed to load search results")?;
        all_results.push((query.clone(), results));
    }

    info!(
        research_id = %research_id,
        query_count = queries.len(),
        total_results = all_results.iter().map(|(_, r)| r.len()).sum::<usize>(),
        "Research data loaded"
    );

    // Step 3: Build assessment prompt
    let prompt = build_assessment_prompt(&website, homepage.as_ref(), &all_results);

    info!(
        website_id = %website_id,
        prompt_length = prompt.len(),
        "Generating AI assessment"
    );

    // Step 4: Generate AI assessment
    let assessment_markdown = ctx
        .deps()
        .ai
        .complete(&prompt)
        .await
        .context("Failed to generate AI assessment")?;

    info!(
        website_id = %website_id,
        assessment_length = assessment_markdown.len(),
        "AI assessment generated"
    );

    // Step 5: Parse metadata from assessment
    let (recommendation, confidence, org_name, founded_year) =
        parse_assessment_metadata(&assessment_markdown);

    info!(
        website_id = %website_id,
        recommendation = %recommendation,
        confidence = ?confidence,
        org_name = ?org_name,
        "Assessment metadata parsed"
    );

    // Step 6: Store assessment
    let assessment = WebsiteAssessment::create(
        website_id.into(),
        Some(research_id),
        assessment_markdown,
        recommendation.clone(),
        confidence,
        org_name.clone(),
        founded_year,
        Some(requested_by.into()),
        "claude-sonnet-4-5".to_string(),
        &ctx.deps().db_pool,
    )
    .await
    .context("Failed to store assessment")?;

    info!(
        website_id = %website_id,
        assessment_id = %assessment.id,
        "Assessment stored successfully"
    );

    // Step 7: Emit success event
    Ok(DomainApprovalEvent::WebsiteAssessmentCompleted {
        website_id,
        job_id,
        assessment_id: assessment.id,
        recommendation,
        confidence_score: confidence,
        organization_name: org_name,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn build_assessment_prompt(
    website: &Website,
    homepage: Option<&WebsiteResearchHomepage>,
    search_results: &[(TavilySearchQuery, Vec<TavilySearchResult>)],
) -> String {
    let mut prompt = format!(
        r#"# Website Assessment Task

You are evaluating whether the website "{}" should be approved for our community resource platform.

## Your Task

Generate a comprehensive "background check" style assessment report in markdown format. This report will help administrators make an informed approval decision.

## Report Structure

Your report MUST start with these YAML-style metadata fields (on separate lines):
RECOMMENDATION: [approve|reject|needs_review]
CONFIDENCE: [0.0-1.0]
ORGANIZATION_NAME: [extracted organization name or "Unknown"]
FOUNDED_YEAR: [year as integer or leave blank]

Then provide:

1. **Executive Summary** (2-3 sentences)
   - What this organization does
   - Key takeaway for approval decision

2. **Organization Background**
   - When founded (if known)
   - Mission and purpose
   - Size/scope of operations

3. **Assessment Findings**
   - Positive indicators (credibility, legitimacy, community value)
   - Concerns or red flags (if any)
   - Notable mentions in search results

4. **Why They Might Be a Good Fit**
   - How they align with community needs
   - What value they provide

5. **Recommendation**
   - Clear approve/reject/needs_review recommendation
   - Specific reasoning

## Source Data

"#,
        website.url
    );

    // Add homepage content
    if let Some(hp) = homepage {
        if let Some(md) = &hp.markdown {
            prompt.push_str(&format!(
                r#"
### Homepage Content

```
{}
```

"#,
                md.chars().take(5000).collect::<String>()
            ));
        }
    }

    // Add search results
    prompt.push_str("\n### Web Research Findings\n\n");
    for (query, results) in search_results {
        prompt.push_str(&format!("**Search Query:** {}\n\n", query.query));
        for result in results {
            prompt.push_str(&format!(
                "- **{}** (score: {:.2})\n  URL: {}\n  {}\n\n",
                result.title,
                result.score,
                result.url,
                result.content.chars().take(300).collect::<String>()
            ));
        }
    }

    prompt.push_str(
        r#"
## Guidelines

- Be objective and evidence-based
- Highlight both positives and concerns
- Consider: legitimacy, transparency, community value, potential risks
- Use clear, professional language
- Focus on facts from the source data

Generate the assessment report now:
"#,
    );

    prompt
}

fn parse_assessment_metadata(
    markdown: &str,
) -> (String, Option<f64>, Option<String>, Option<i32>) {
    let mut recommendation = "needs_review".to_string();
    let mut confidence: Option<f64> = None;
    let mut org_name: Option<String> = None;
    let mut founded_year: Option<i32> = None;

    // Parse first few lines for metadata
    for line in markdown.lines().take(10) {
        if line.starts_with("RECOMMENDATION:") {
            let value = line
                .trim_start_matches("RECOMMENDATION:")
                .trim()
                .to_lowercase();
            if ["approve", "reject", "needs_review"].contains(&value.as_str()) {
                recommendation = value;
            }
        } else if line.starts_with("CONFIDENCE:") {
            let value = line.trim_start_matches("CONFIDENCE:").trim();
            if let Ok(conf) = value.parse::<f64>() {
                confidence = Some(conf.clamp(0.0, 1.0));
            }
        } else if line.starts_with("ORGANIZATION_NAME:") {
            let value = line.trim_start_matches("ORGANIZATION_NAME:").trim();
            if !value.is_empty() && value.to_lowercase() != "unknown" {
                org_name = Some(value.to_string());
            }
        } else if line.starts_with("FOUNDED_YEAR:") {
            let value = line.trim_start_matches("FOUNDED_YEAR:").trim();
            if let Ok(year) = value.parse::<i32>() {
                founded_year = Some(year);
            }
        }
    }

    (recommendation, confidence, org_name, founded_year)
}
