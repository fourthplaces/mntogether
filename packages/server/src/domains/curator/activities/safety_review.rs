use std::collections::HashMap;

use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::domains::curator::models::{CuratorAction, PageBriefExtraction};
use crate::kernel::{ServerDeps, GPT_5_MINI};

const SAFETY_REVIEW_PROMPT: &str = r#"
You are a safety reviewer for a community platform serving immigrant communities in Minnesota.

Your ONLY job: check whether each post omits eligibility restrictions that exist in the
source material. This is critical because our users include undocumented immigrants, refugees,
and people with uncertain legal status. If a post says "no questions asked" but the source
says "US citizens or legal residents only", someone could register their personal information
and be turned away — or worse.

## What to Check

For each post, compare it against ITS OWN source material (listed directly below the post)
and check:

1. **Citizenship or residency requirements** — Does the source mention "US citizens",
   "legal residents", "must have papers", or similar? If yes, does the post state this?
2. **ID requirements** — Does the source require ID, driver's license, insurance, or
   proof of status? If yes, does the post state this?
3. **Age restrictions** — Does the source say "18+", "adults only", or similar?
4. **Geographic restrictions** — Does the source limit service to a specific area?
5. **Registration requirements** — Does the source require registration but the post
   says "no questions asked" or "just show up"?
6. **False safety claims** — Does the post say "no paperwork", "no questions", or
   "no ID needed" when the source material says otherwise?

## What to Return

For each post, return:
- **verdict**: "safe", "fix", or "blocked"
  - "safe" — no issues found
  - "fix" — restriction is missing but can be added; include a corrected description
  - "blocked" — post is fundamentally misleading and should not be published
- **issue** — what's wrong (empty string if safe)
- **fixed_description** — corrected description with the restriction clearly stated
  (only for "fix" verdicts; null otherwise)
- **fixed_summary** — corrected summary if needed (null if unchanged)

## Rules

- ONLY flag real eligibility restrictions from the source material. Don't invent restrictions.
- If the source says "no ID required", that's fine — don't flag it.
- If two access methods have different eligibility (e.g., pickup is open to all but delivery
  requires citizenship), the post must clearly distinguish them or be split.
- When fixing, keep the same voice and structure. Just add the missing restriction clearly.
- Be concise. Don't rewrite the whole post — splice in the restriction where it belongs.
"#;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SafetyReviewResponse {
    pub reviews: Vec<PostReview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostReview {
    /// Index matching the input post
    pub index: usize,
    /// "safe", "fix", or "blocked"
    pub verdict: String,
    /// What's wrong (empty if safe)
    pub issue: String,
    /// Corrected description (null if safe or blocked)
    pub fixed_description: Option<String>,
    /// Corrected summary (null if unchanged)
    pub fixed_summary: Option<String>,
}

const MAX_SAFETY_ATTEMPTS: usize = 3;

/// Run safety review on all create_post / update_post actions.
/// Loops up to 3 times: review → fix → re-review until all pass or are blocked.
/// Posts that pass ("safe") are done — only fixed posts get re-reviewed.
/// After 3 failed fix attempts, the post is blocked.
pub async fn review_and_fix_actions(
    actions: &mut Vec<CuratorAction>,
    briefs: &[(String, PageBriefExtraction)],
    deps: &ServerDeps,
) -> Result<SafetyOutcome> {
    let mut blocked_indices: Vec<usize> = Vec::new();
    let mut passed_indices: Vec<usize> = Vec::new();
    let mut total_fixes = 0;
    let mut fix_attempts: HashMap<usize, usize> = HashMap::new();

    for attempt in 0..MAX_SAFETY_ATTEMPTS {
        // Only review posts that haven't passed or been blocked yet
        let posts_for_review: Vec<(usize, &CuratorAction)> = actions
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                (a.action_type == "create_post" || a.action_type == "update_post")
                    && !blocked_indices.contains(i)
                    && !passed_indices.contains(i)
            })
            .collect();

        if posts_for_review.is_empty() {
            break;
        }

        // Build prompt with per-post source material
        let user_prompt = build_review_prompt(&posts_for_review, briefs);

        info!(
            prompt_len = user_prompt.len(),
            posts = posts_for_review.len(),
            "\n--- SAFETY REVIEW PROMPT START ---\n{}\n--- SAFETY REVIEW PROMPT END ---",
            user_prompt
        );

        let reviews = deps
            .ai
            .extract::<SafetyReviewResponse>(GPT_5_MINI, SAFETY_REVIEW_PROMPT, &user_prompt)
            .await
            .map_err(|e| anyhow::anyhow!("Safety review failed: {}", e))?;

        let mut any_fixes_this_round = false;

        for review in &reviews.reviews {
            let idx = review.index;
            match review.verdict.as_str() {
                "safe" => {
                    passed_indices.push(idx);
                    let title = actions.get(idx)
                        .and_then(|a| a.title.as_deref())
                        .unwrap_or("?");
                    info!(idx = idx, title = title, "Safety review: safe");
                }
                "fix" => {
                    let attempts = fix_attempts.entry(idx).or_insert(0);
                    *attempts += 1;

                    if *attempts >= MAX_SAFETY_ATTEMPTS {
                        blocked_indices.push(idx);
                        info!(
                            idx = idx,
                            attempts = *attempts,
                            issue = review.issue.as_str(),
                            "Safety review: blocked after max fix attempts"
                        );
                    } else if let Some(action) = actions.get_mut(idx) {
                        if let Some(fixed_desc) = &review.fixed_description {
                            action.description = Some(fixed_desc.clone());
                            action.description_markdown = Some(fixed_desc.clone());
                        }
                        if let Some(fixed_summary) = &review.fixed_summary {
                            action.summary = Some(fixed_summary.clone());
                        }
                        total_fixes += 1;
                        any_fixes_this_round = true;
                        info!(
                            idx = idx,
                            attempt = *attempts,
                            issue = review.issue.as_str(),
                            "Safety review: fixed, will re-check"
                        );
                    }
                }
                "blocked" => {
                    blocked_indices.push(idx);
                    info!(
                        idx = idx,
                        issue = review.issue.as_str(),
                        "Safety review: blocked"
                    );
                }
                _ => {}
            }
        }

        if !any_fixes_this_round {
            break;
        }

        info!(
            attempt = attempt + 1,
            fixes = total_fixes,
            "Safety review round complete, re-checking fixes"
        );
    }

    // Remove blocked actions (iterate in reverse to preserve indices)
    let blocked_count = blocked_indices.len();
    blocked_indices.sort_unstable();
    for &idx in blocked_indices.iter().rev() {
        let title = actions[idx].title.clone().unwrap_or_default();
        info!(idx = idx, title = title.as_str(), "Removing blocked action");
        actions.remove(idx);
    }

    Ok(SafetyOutcome {
        fixes_applied: total_fixes,
        posts_blocked: blocked_count,
    })
}

pub struct SafetyOutcome {
    pub fixes_applied: usize,
    pub posts_blocked: usize,
}

/// Build the review prompt with each post's source material inline.
/// Each post gets only the briefs that match its source_urls.
fn build_review_prompt(
    posts: &[(usize, &CuratorAction)],
    briefs: &[(String, PageBriefExtraction)],
) -> String {
    let mut prompt = String::new();

    for (idx, action) in posts {
        let title = action.title.as_deref().unwrap_or("(untitled)");
        let summary = action.summary.as_deref().unwrap_or("");
        let description = action.description.as_deref().unwrap_or("");

        prompt.push_str(&format!("## Post {} — {}\n", idx, title));
        prompt.push_str(&format!("**Summary:** {}\n", summary));
        prompt.push_str(&format!("**Description:**\n{}\n\n", description));

        // Find matching briefs by source_urls
        let matching_briefs: Vec<_> = briefs
            .iter()
            .filter(|(url, _)| {
                action.source_urls.iter().any(|src| urls_match(src, url))
            })
            .collect();

        if matching_briefs.is_empty() {
            prompt.push_str("**Source Material:** No matching sources found.\n\n");
        } else {
            prompt.push_str("**Source Material for this post:**\n\n");
            for (url, brief) in &matching_briefs {
                prompt.push_str(&format!("### {}\n", url));
                prompt.push_str(&format!("{}\n", brief.summary));
                if let Some(critical) = &brief.critical_info {
                    prompt.push_str(&format!("- Critical: {}\n", critical));
                }
                if let Some(capacity) = &brief.capacity_info {
                    prompt.push_str(&format!("- Capacity: {}\n", capacity));
                }
                if !brief.calls_to_action.is_empty() {
                    prompt.push_str(&format!(
                        "- Actions: {}\n",
                        brief.calls_to_action.join("; ")
                    ));
                }
                if !brief.services.is_empty() {
                    prompt.push_str(&format!("- Services: {}\n", brief.services.join(", ")));
                }
                prompt.push('\n');
            }
        }

        prompt.push_str("---\n\n");
    }

    prompt
}

/// Check if two URLs refer to the same page, handling trailing slashes and scheme differences.
fn urls_match(a: &str, b: &str) -> bool {
    let normalize = |u: &str| {
        u.trim_end_matches('/')
            .replace("http://", "https://")
            .replace("www.", "")
    };
    normalize(a) == normalize(b)
}
