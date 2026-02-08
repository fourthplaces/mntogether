//! AI pre-filter evaluation for agent pipelines.
//!
//! Evaluates discovered websites against an agent's plain-text filter rules using an LLM.
//! Batches all results from a single query into one AI call for efficiency.

use anyhow::Result;
use tracing::{info, warn};

use crate::domains::agents::models::AgentFilterRule;
use crate::kernel::llm_request::CompletionExt;
use openai_client::OpenAIClient;

/// A website to evaluate.
#[derive(Debug, Clone)]
pub struct WebsiteCandidate {
    pub domain: String,
    pub url: String,
    pub title: String,
    pub snippet: String,
}

/// Result of AI filter evaluation for a single website.
#[derive(Debug, Clone)]
pub struct FilterEvaluation {
    pub domain: String,
    pub passed: bool,
    pub reason: String,
}

/// Evaluate a batch of discovered websites against an agent's filter rules.
///
/// Uses a single AI call per batch for efficiency.
/// Returns pass/fail with reason for each website.
pub async fn evaluate_websites_against_filters(
    websites: &[WebsiteCandidate],
    rules: &[AgentFilterRule],
    purpose: &str,
    ai: &OpenAIClient,
) -> Result<Vec<FilterEvaluation>> {
    if websites.is_empty() {
        return Ok(vec![]);
    }

    // If no filter rules, everything passes
    if rules.is_empty() {
        return Ok(websites
            .iter()
            .map(|w| FilterEvaluation {
                domain: w.domain.clone(),
                passed: true,
                reason: "No filter rules configured".to_string(),
            })
            .collect());
    }

    let prompt = build_filter_prompt(websites, rules, purpose);

    info!(
        websites_count = websites.len(),
        rules_count = rules.len(),
        "Evaluating websites against agent filter rules"
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
    rules: &[AgentFilterRule],
    purpose: &str,
) -> String {
    let mut prompt = format!(
        "You are evaluating websites discovered by a search query.\n\
         The agent's purpose: {}\n\n\
         For each website, determine if it should PASS (enter the review queue) or FAIL (be filtered out).\n\n",
        purpose
    );

    prompt.push_str("## Filter Rules\n\n");
    for rule in rules {
        prompt.push_str(&format!("- {}\n", rule.rule_text));
    }
    prompt.push('\n');

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
        let mut end = max_len;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    }
}
