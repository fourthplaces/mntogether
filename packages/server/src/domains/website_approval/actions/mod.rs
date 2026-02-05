//! Website Approval actions
//!
//! Entry-point actions are called directly from GraphQL mutations via `process()`.
//! They do the work synchronously and return values.
//!
//! Actions are self-contained: they take raw Uuid types, handle conversions,
//! and return simple values.
//!
//! Flow:
//! - If fresh research exists → generate assessment directly (synchronous)
//! - If research is stale/missing → create research, trigger search cascade (async)

use crate::common::{JobId, MemberId, WebsiteId};
use crate::domains::website::models::{
    CreateTavilySearchQuery, CreateWebsiteAssessment, TavilySearchQuery, TavilySearchResult,
    Website, WebsiteAssessment, WebsiteResearch, WebsiteResearchHomepage,
};
use crate::domains::website_approval::events::WebsiteApprovalEvent;
use crate::kernel::{CompletionExt, FirecrawlIngestor, HttpIngestor, ServerDeps, ValidatedIngestor};
use anyhow::{Context, Result};
use tracing::info;
use uuid::Uuid;

/// Result of a website assessment operation (for GraphQL return)
#[derive(Debug, Clone)]
pub struct AssessmentResult {
    pub job_id: Uuid,
    pub website_id: Uuid,
    pub assessment_id: Option<Uuid>,
    pub status: String,
    pub message: Option<String>,
}

impl AssessmentResult {
    /// Create from a WebsiteApprovalEvent
    pub fn from_event(event: &WebsiteApprovalEvent) -> Self {
        match event {
            WebsiteApprovalEvent::WebsiteResearchCreated {
                job_id,
                website_id,
                ..
            } => Self {
                job_id: job_id.into_uuid(),
                website_id: website_id.into_uuid(),
                assessment_id: None,
                status: "processing".to_string(),
                message: Some("Research created, running web searches...".to_string()),
            },
            WebsiteApprovalEvent::WebsiteAssessmentCompleted {
                job_id,
                website_id,
                assessment_id,
                ..
            } => Self {
                job_id: job_id.into_uuid(),
                website_id: website_id.into_uuid(),
                assessment_id: Some(*assessment_id),
                status: "completed".to_string(),
                message: Some("Assessment completed".to_string()),
            },
            _ => Self {
                job_id: Uuid::nil(),
                website_id: Uuid::nil(),
                assessment_id: None,
                status: "unknown".to_string(),
                message: None,
            },
        }
    }
}

/// Result of conducting searches
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub total_queries: usize,
    pub total_results: usize,
}

// ============================================================================
// Entry Point: Assess Website
// ============================================================================

/// Assess a website by fetching/creating research and generating an assessment.
///
/// Returns an event:
/// - `WebsiteAssessmentCompleted` if fresh research exists (sync completion)
/// - `WebsiteResearchCreated` if new research created (triggers async cascade)
pub async fn assess_website(
    website_id: Uuid,
    member_id: Uuid,
    deps: &ServerDeps,
) -> Result<WebsiteApprovalEvent> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(
        website_id = %website_id,
        job_id = %job_id,
        "Starting website assessment"
    );

    // Step 1: Fetch website to ensure it exists
    let website = Website::find_by_id(website_id_typed.into(), &deps.db_pool)
        .await
        .context(format!("Website not found: {}", website_id))?;

    info!(
        website_id = %website_id,
        website_domain = %website.domain,
        "Website found"
    );

    // Step 2: Check for existing fresh research (<7 days old)
    let existing =
        WebsiteResearch::find_latest_by_website_id(website_id_typed.into(), &deps.db_pool)
            .await?;

    if let Some(research) = existing {
        let age_days = (chrono::Utc::now() - research.created_at).num_days();

        info!(
            research_id = %research.id,
            age_days = age_days,
            "Found existing research"
        );

        if age_days < 7 {
            // Fresh research exists - generate assessment synchronously
            info!(
                research_id = %research.id,
                "Research is fresh, generating assessment directly"
            );

            let assessment =
                generate_assessment(research.id, website_id_typed, job_id, requested_by, deps)
                    .await?;

            return Ok(WebsiteApprovalEvent::WebsiteAssessmentCompleted {
                website_id: website_id_typed,
                job_id,
                assessment_id: assessment.id,
                recommendation: assessment.recommendation.clone(),
                confidence_score: assessment.confidence_score,
                organization_name: assessment.organization_name.clone(),
            });
        }

        info!(research_id = %research.id, "Research is stale, creating fresh research");
    }

    // Step 3: Create fresh research - fetch homepage using extraction library
    info!(website_domain = %website.domain, "Fetching homepage via extraction library");

    let homepage_url = format!("https://{}", &website.domain);
    let urls = vec![homepage_url.clone()];

    // Get extraction service (required for homepage fetching)
    let extraction = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

    // Use extraction library to ingest the homepage
    let ingest_result = match FirecrawlIngestor::from_env() {
        Ok(firecrawl) => {
            let ingestor = ValidatedIngestor::new(firecrawl);
            extraction.ingest_urls(&urls, &ingestor).await
        }
        Err(_) => {
            let http = HttpIngestor::new();
            let ingestor = ValidatedIngestor::new(http);
            extraction.ingest_urls(&urls, &ingestor).await
        }
    };

    let homepage_content = match ingest_result {
        Ok(result) if result.pages_summarized > 0 => {
            info!(
                website_domain = %website.domain,
                pages_summarized = result.pages_summarized,
                "Homepage fetched and ingested successfully"
            );
            // Get the content from extraction index
            match extraction
                .extract_one("homepage content", Some(&homepage_url))
                .await
            {
                Ok(extraction) => Some(extraction.content),
                Err(_) => None,
            }
        }
        Ok(_) => {
            tracing::warn!(
                website_domain = %website.domain,
                "Homepage ingested but no content summarized"
            );
            None
        }
        Err(e) => {
            tracing::warn!(
                website_domain = %website.domain,
                error = %e,
                "Failed to fetch homepage, continuing with search-based research"
            );
            None
        }
    };

    // Step 4: Create research record
    let research = WebsiteResearch::create(
        website_id_typed.into(),
        website.domain.clone(),
        Some(requested_by.into()),
        &deps.db_pool,
    )
    .await
    .context("Failed to create research record")?;

    info!(research_id = %research.id, "Research record created");

    // Step 5: Store homepage content (if available)
    if let Some(content) = homepage_content {
        WebsiteResearchHomepage::create(
            research.id,
            Some(content.clone()),
            Some(content),
            &deps.db_pool,
        )
        .await
        .context("Failed to store homepage content")?;

        info!(research_id = %research.id, "Homepage content stored");
    }

    // Step 6: Return event to trigger async search cascade
    // Flow: WebsiteResearchCreated → conduct_searches → generate_assessment → WebsiteAssessmentCompleted
    Ok(WebsiteApprovalEvent::WebsiteResearchCreated {
        research_id: research.id,
        website_id: website_id_typed,
        job_id,
        homepage_url: website.domain.clone(),
        requested_by,
    })
}

// ============================================================================
// Core Actions - Called by both entry points and effect handlers
// ============================================================================

/// Generate an AI assessment from research data.
///
/// This is the core logic - can be called directly from actions or via effects.
/// Returns the created WebsiteAssessment.
pub async fn generate_assessment(
    research_id: Uuid,
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<WebsiteAssessment> {
    info!(
        research_id = %research_id,
        website_id = %website_id,
        job_id = %job_id,
        "Generating assessment from research"
    );

    // Step 1: Load website
    let website = Website::find_by_id(website_id.into(), &deps.db_pool)
        .await
        .context(format!("Website not found: {}", website_id))?;

    // Step 2: Load research data
    let homepage = WebsiteResearchHomepage::find_by_research_id(research_id, &deps.db_pool)
        .await
        .context("Failed to load homepage")?;

    let queries = TavilySearchQuery::find_by_research_id(research_id, &deps.db_pool)
        .await
        .context("Failed to load search queries")?;

    let mut all_results = Vec::new();
    for query in &queries {
        let results = TavilySearchResult::find_by_query_id(query.id, &deps.db_pool)
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
    let assessment_markdown = deps
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
        CreateWebsiteAssessment::builder()
            .website_id(website_id.into_uuid())
            .assessment_markdown(assessment_markdown.clone())
            .recommendation(recommendation)
            .model_used("claude-sonnet-4-5")
            .website_research_id(Some(research_id))
            .confidence_score(confidence)
            .organization_name(org_name)
            .founded_year(founded_year)
            .generated_by(Some(requested_by.into_uuid()))
            .build(),
        &deps.db_pool,
    )
    .await
    .context("Failed to store assessment")?;

    info!(
        website_id = %website_id,
        assessment_id = %assessment.id,
        "Assessment stored successfully"
    );

    // Step 7: Generate and store embedding for semantic search (non-fatal)
    match deps
        .embedding_service
        .generate(&assessment_markdown)
        .await
    {
        Ok(embedding) => {
            if let Err(e) =
                WebsiteAssessment::update_embedding(assessment.id, &embedding, &deps.db_pool)
                    .await
            {
                tracing::warn!(
                    assessment_id = %assessment.id,
                    error = %e,
                    "Failed to store assessment embedding (non-fatal)"
                );
            } else {
                info!(
                    assessment_id = %assessment.id,
                    embedding_dim = embedding.len(),
                    "Assessment embedding stored"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                assessment_id = %assessment.id,
                error = %e,
                "Failed to generate assessment embedding (non-fatal)"
            );
        }
    }

    Ok(assessment)
}

/// Conduct research searches for a website.
///
/// Executes Tavily searches and stores results. Returns search statistics.
pub async fn conduct_searches(
    research_id: Uuid,
    website_id: WebsiteId,
    deps: &ServerDeps,
) -> Result<SearchResult> {
    info!(
        research_id = %research_id,
        website_id = %website_id,
        "Conducting research searches"
    );

    // Step 1: Load research to get website URL
    let research =
        WebsiteResearch::find_latest_by_website_id(website_id.into(), &deps.db_pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Research not found: {}", research_id))?;

    // Step 2: Extract domain name from URL
    let domain_name = extract_domain_name(&research.homepage_url);

    info!(
        research_id = %research_id,
        domain_name = %domain_name,
        "Extracted domain name"
    );

    // Step 3: Define research queries
    let queries = vec![
        format!("{} organization background mission", domain_name),
        format!("{} reviews complaints problems", domain_name),
        format!("{} founded history about", domain_name),
    ];

    let mut total_results = 0;

    // Step 4: Execute each search and store results
    for query_text in &queries {
        info!(query = %query_text, "Executing Tavily search");

        // Execute search
        let results = deps
            .web_searcher
            .search_with_limit(query_text, 5)
            .await
            .context(format!("Failed to execute search: {}", query_text))?;

        info!(
            query = %query_text,
            result_count = results.len(),
            "Tavily search completed"
        );

        // Store query record
        let query_record = TavilySearchQuery::create(
            CreateTavilySearchQuery::builder()
                .website_research_id(research.id)
                .query(query_text.clone())
                .search_depth(Some("basic".to_string()))
                .max_results(Some(5))
                .build(),
            &deps.db_pool,
        )
        .await
        .context("Failed to store query record")?;

        // Store results
        if !results.is_empty() {
            let result_tuples: Vec<_> = results
                .into_iter()
                .map(|r| {
                    (
                        r.title.unwrap_or_default(),
                        r.url.to_string(),
                        r.snippet.unwrap_or_default(),
                        r.score.unwrap_or(0.0) as f64,
                        None::<String>, // published_date not available in extraction SearchResult
                    )
                })
                .collect();

            total_results += result_tuples.len();

            TavilySearchResult::create_batch(query_record.id, result_tuples, &deps.db_pool)
                .await
                .context("Failed to store search results")?;

            info!(
                query_id = %query_record.id,
                result_count = total_results,
                "Search results stored"
            );
        }
    }

    // Step 5: Mark research as complete
    research
        .mark_tavily_complete(&deps.db_pool)
        .await
        .context("Failed to mark research complete")?;

    info!(
        research_id = %research_id,
        total_queries = queries.len(),
        total_results = total_results,
        "All research searches completed"
    );

    Ok(SearchResult {
        total_queries: queries.len(),
        total_results,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build the assessment prompt from website and research data.
pub fn build_assessment_prompt(
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
        website.domain
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

/// Parse assessment metadata from the generated markdown.
pub fn parse_assessment_metadata(
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

/// Extract domain name from a URL.
pub fn extract_domain_name(url: &str) -> String {
    url.trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}
