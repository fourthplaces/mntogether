use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{MemberId, PostId, SyncProposalId};
use crate::domains::curator::models::CuratorAction;
use crate::domains::sync::models::{ProposalComment, SyncProposal};
use crate::kernel::{ServerDeps, GPT_5_MINI};

const MAX_REVISIONS: i32 = 3;

const REFINEMENT_PROMPT: &str = r#"
You are revising a proposed action based on admin feedback.

The admin is a volunteer reviewer who knows the community. Their feedback should be
taken seriously and incorporated precisely.

Return the revised action with the same structure. Only change what the feedback asks for.
If the feedback contradicts the source material, note that in your reasoning but still
make the requested change — the admin has final say.
"#;

pub enum RefineResult {
    Revised,
    MaxRevisionsReached,
}

/// Refine a proposal based on an admin comment. Returns whether revision happened.
pub async fn refine_proposal_from_comment(
    proposal_id: Uuid,
    comment: &str,
    author_id: MemberId,
    deps: &ServerDeps,
) -> Result<RefineResult> {
    let pool = &deps.db_pool;
    let proposal_typed_id = SyncProposalId::from(proposal_id);

    let proposal = SyncProposal::find_by_id(proposal_typed_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Proposal not found: {}", proposal_id))?;

    if proposal.revision_count >= MAX_REVISIONS {
        // Save the comment but don't revise
        ProposalComment::create(
            proposal_typed_id,
            author_id,
            comment,
            proposal.revision_count,
            false,
            pool,
        )
        .await?;
        return Ok(RefineResult::MaxRevisionsReached);
    }

    // Build context for the LLM
    let comments = ProposalComment::find_by_proposal(proposal_typed_id, pool).await?;
    let draft_summary = load_draft_summary(&proposal, pool).await?;

    let user_prompt = format!(
        "## Original Proposal\nOperation: {}\nEntity type: {}\nReasoning: {}\n\n## Current Draft\n{}\n\n## Comment History\n{}\n\n## Latest Comment (respond to this)\n{}",
        proposal.operation,
        proposal.entity_type,
        proposal.consultant_reasoning.as_deref().unwrap_or(""),
        draft_summary,
        format_comment_history(&comments),
        comment,
    );

    let revised: CuratorAction = deps
        .ai
        .extract::<CuratorAction>(GPT_5_MINI, REFINEMENT_PROMPT, &user_prompt)
        .await
        .map_err(|e| anyhow::anyhow!("Refinement LLM call failed: {}", e))?;

    // Apply the revised content to the draft entity
    apply_revision_to_draft(&proposal, &revised, pool).await?;

    // Increment revision count
    SyncProposal::increment_revision(proposal_typed_id, pool).await?;

    // Save the comment with revision metadata
    ProposalComment::create(
        proposal_typed_id,
        author_id,
        comment,
        proposal.revision_count + 1,
        true, // ai_revised
        pool,
    )
    .await?;

    Ok(RefineResult::Revised)
}

async fn load_draft_summary(proposal: &SyncProposal, pool: &PgPool) -> Result<String> {
    if let Some(draft_id) = proposal.draft_entity_id {
        match proposal.entity_type.as_str() {
            "post" => {
                if let Ok(Some(post)) =
                    crate::domains::posts::models::Post::find_by_id(draft_id.into(), pool).await
                {
                    return Ok(format!(
                        "Title: {}\nSummary: {}\nDescription: {}\nType: {}\nCategory: {}",
                        post.title,
                        post.summary.as_deref().unwrap_or(""),
                        truncate_safe(&post.description, 500),
                        post.post_type,
                        post.category,
                    ));
                }
            }
            "note" => {
                if let Ok(note) =
                    crate::domains::notes::models::Note::find_by_id(draft_id.into(), pool).await
                {
                    return Ok(format!(
                        "Content: {}\nSeverity: {}",
                        note.content, note.severity,
                    ));
                }
            }
            _ => {}
        }
    }
    Ok("(no draft available)".to_string())
}

async fn apply_revision_to_draft(
    proposal: &SyncProposal,
    revised: &CuratorAction,
    pool: &PgPool,
) -> Result<()> {
    if let Some(draft_id) = proposal.draft_entity_id {
        match proposal.entity_type.as_str() {
            "post" => {
                use crate::domains::posts::models::{Post, UpdatePostContent};
                let update = UpdatePostContent::builder()
                    .id(PostId::from(draft_id))
                    .title(revised.title.clone())
                    .description(revised.description.clone())
                    .summary(revised.summary.clone())
                    .category(revised.category.clone())
                    .urgency(revised.urgency.clone())
                    .build();
                Post::update_content(update, pool).await?;
            }
            "note" => {
                if let Some(content) = &revised.note_content {
                    let severity = revised.note_severity.as_deref().unwrap_or("info");
                    crate::domains::notes::models::Note::update(
                        draft_id.into(),
                        content,
                        severity,
                        true,
                        None,
                        pool,
                    )
                    .await?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

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

fn format_comment_history(comments: &[ProposalComment]) -> String {
    if comments.is_empty() {
        return "(no previous comments)".to_string();
    }
    comments
        .iter()
        .map(|c| {
            format!(
                "[Revision {}{}] {}",
                c.revision_number,
                if c.ai_revised { " (AI revised)" } else { "" },
                c.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
