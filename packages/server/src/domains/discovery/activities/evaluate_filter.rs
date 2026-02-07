//! AI pre-filter evaluation
//!
//! Evaluates discovered websites against plain-text filter rules using an LLM.
//! Batches all results from a single query into one AI call for efficiency.

use anyhow::Result;
use tracing::{info, warn};

use crate::domains::discovery::models::DiscoveryFilterRule;
use crate::kernel::llm_request::CompletionExt;
use openai_client::OpenAIClient;

/// A website to evaluate
#[derive(Debug, Clone)]
pub struct WebsiteCandidate {
    pub domain: String,
    pub url: String,
    pub title: String,
    pub snippet: String,
}

/// Result of AI filter evaluation for a single website
#[derive(Debug, Clone)]
pub struct FilterEvaluation {
    pub domain: String,
    pub passed: bool,
    pub reason: String,
}

/// Evaluate a batch of discovered websites against filter rules.
///
/// Uses a single AI call per batch (all results from one query).
/// Returns pass/fail with reason for each website.
pub async fn evaluate_websites_against_filters(
    websites: &[WebsiteCandidate],
    global_rules: &[DiscoveryFilterRule],
    query_rules: &[DiscoveryFilterRule],
    ai: &OpenAIClient,
) -> Result<Vec<FilterEvaluation>> {
    if websites.is_empty() {
        return Ok(vec![]);
    }

    // If no filter rules, everything passes
    if global_rules.is_empty() && query_rules.is_empty() {
        return Ok(websites
            .iter()
            .map(|w| FilterEvaluation {
                domain: w.domain.clone(),
                passed: true,
                reason: "No filter rules configured".to_string(),
            })
            .collect());
    }

    let prompt = build_filter_prompt(websites, global_rules, query_rules);

    info!(
        websites_count = websites.len(),
        global_rules = global_rules.len(),
        query_rules = query_rules.len(),
        "Evaluating websites against filter rules"
    );

    let response = match ai.complete_with_model(&prompt, Some("gpt-4o-mini")).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "AI filter evaluation failed, passing all websites through");
            return Ok(websites
                .iter()
                .map(|w| FilterEvaluation {
                    domain: w.domain.clone(),
                    passed: true,
                    reason: format!("AI evaluation unavailable: {}", e),
                })
                .collect());
        }
    };

    parse_filter_response(&response, websites)
}

fn build_filter_prompt(
    websites: &[WebsiteCandidate],
    global_rules: &[DiscoveryFilterRule],
    query_rules: &[DiscoveryFilterRule],
) -> String {
    let mut prompt = String::from(
        "You are evaluating websites discovered by a search query for community resources.\n\
         For each website, determine if it should PASS (enter the review queue) or FAIL (be filtered out).\n\n",
    );

    prompt.push_str("## Filter Rules\n\n");

    if !global_rules.is_empty() {
        prompt.push_str("### Global Rules (apply to all)\n");
        for rule in global_rules {
            prompt.push_str(&format!("- {}\n", rule.rule_text));
        }
        prompt.push('\n');
    }

    if !query_rules.is_empty() {
        prompt.push_str("### Query-Specific Rules (override global rules if conflicting)\n");
        for rule in query_rules {
            prompt.push_str(&format!("- {}\n", rule.rule_text));
        }
        prompt.push('\n');
    }

    prompt.push_str("## Websites to Evaluate\n\n");
    for (i, w) in websites.iter().enumerate() {
        prompt.push_str(&format!(
            "{}. Domain: {} | Title: \"{}\" | Snippet: \"{}\"\n",
            i + 1,
            w.domain,
            w.title,
            truncate(&w.snippet, 200),
        ));
    }

    prompt.push_str(
        "\n## Response Format\n\n\
         For each website (by number), respond with exactly one line:\n\
         NUMBER. PASS|FAIL - reason\n\n\
         Example:\n\
         1. PASS - Community food shelf providing direct services\n\
         2. FAIL - Government agency (.gov domain)\n",
    );

    prompt
}

fn parse_filter_response(
    response: &str,
    websites: &[WebsiteCandidate],
) -> Result<Vec<FilterEvaluation>> {
    let mut evaluations = Vec::with_capacity(websites.len());

    for (i, website) in websites.iter().enumerate() {
        let line_prefix = format!("{}.", i + 1);

        let evaluation = response
            .lines()
            .find(|line| line.trim().starts_with(&line_prefix))
            .map(|line| {
                let after_num = line.trim().trim_start_matches(&line_prefix).trim();
                let passed = after_num.starts_with("PASS");
                let reason = after_num
                    .trim_start_matches("PASS")
                    .trim_start_matches("FAIL")
                    .trim_start_matches(" - ")
                    .trim_start_matches(" -")
                    .trim()
                    .to_string();

                FilterEvaluation {
                    domain: website.domain.clone(),
                    passed,
                    reason,
                }
            })
            .unwrap_or_else(|| {
                // If we can't parse the response for this website, pass it through
                warn!(
                    domain = %website.domain,
                    index = i + 1,
                    "Could not parse AI filter response, passing through"
                );
                FilterEvaluation {
                    domain: website.domain.clone(),
                    passed: true,
                    reason: "Could not parse AI filter response".to_string(),
                }
            });

        evaluations.push(evaluation);
    }

    Ok(evaluations)
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        // Find a safe char boundary
        let mut end = max_len;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    }
}
