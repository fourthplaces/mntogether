use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::curator::models::{OrgDocument, PageBriefExtraction};
use crate::domains::contacts::models::Contact;
use crate::domains::notes::models::Note;
use crate::domains::posts::models::Post;
use crate::domains::schedules::models::Schedule;
use crate::domains::tag::models::Tag;

/// Maximum character budget for the org document (~50k tokens at ~4 chars/token).
const MAX_ORG_DOCUMENT_CHARS: usize = 200_000;

/// Compile the org document from all available data.
/// Priority: website briefs -> recent social briefs -> existing posts -> notes
pub async fn compile_org_document(
    org_id: Uuid,
    org_name: &str,
    briefs: &[(String, PageBriefExtraction)],
    pool: &PgPool,
) -> Result<OrgDocument> {
    let mut doc = String::new();
    let mut budget = MAX_ORG_DOCUMENT_CHARS;

    // Header
    let header = format!("# Organization: {}\n\n", org_name);
    doc.push_str(&header);
    budget = budget.saturating_sub(header.len());

    // Section 1: Website briefs (highest priority — anything without social indicators)
    doc.push_str("## Website Content\n\n");
    let mut briefs_count = 0;
    for (url, brief) in briefs {
        if is_social_url(url) {
            continue;
        }
        let section = format_brief(url, brief);
        if section.len() > budget {
            break;
        }
        doc.push_str(&section);
        budget = budget.saturating_sub(section.len());
        briefs_count += 1;
    }

    // Section 2: Social media briefs (recent first)
    doc.push_str("\n## Social Media\n\n");
    for (url, brief) in briefs {
        if !is_social_url(url) {
            continue;
        }
        let section = format_brief(url, brief);
        if section.len() > budget {
            break;
        }
        doc.push_str(&section);
        budget = budget.saturating_sub(section.len());
        briefs_count += 1;
    }

    // Section 3: Existing posts in the system
    let existing_posts = Post::find_by_organization_id(org_id, pool).await?;
    doc.push_str("\n## Existing Posts in System\n\n");
    let mut posts_count = 0;
    for post in &existing_posts {
        let post_uuid: Uuid = post.id.into();
        let contacts = Contact::find_by_entity("post", post_uuid, pool)
            .await
            .unwrap_or_default();
        let schedules = Schedule::find_for_post(post_uuid, pool)
            .await
            .unwrap_or_default();
        let tags = Tag::find_for_post(post.id, pool).await.unwrap_or_default();

        let section = format_existing_post(post, &contacts, &schedules, &tags);
        if section.len() > budget {
            break;
        }
        doc.push_str(&section);
        budget = budget.saturating_sub(section.len());
        posts_count += 1;
    }

    // Section 4: Existing notes linked to this org's posts
    let mut notes_count = 0;
    if !existing_posts.is_empty() {
        doc.push_str("\n## Existing Notes\n\n");
        let post_uuids: Vec<Uuid> = existing_posts.iter().map(|p| p.id.into()).collect();
        for post_uuid in &post_uuids {
            let notes = Note::find_active_for_entity("post", *post_uuid, pool)
                .await
                .unwrap_or_default();
            for note in &notes {
                let section = format_note(note);
                if section.len() > budget {
                    break;
                }
                doc.push_str(&section);
                budget = budget.saturating_sub(section.len());
                notes_count += 1;
            }
        }
    }

    Ok(OrgDocument {
        token_estimate: doc.len() / 4,
        content: doc,
        briefs_included: briefs_count,
        posts_included: posts_count,
        notes_included: notes_count,
    })
}

fn is_social_url(url: &str) -> bool {
    url.contains("instagram.com")
        || url.contains("facebook.com")
        || url.contains("twitter.com")
        || url.contains("x.com")
        || url.contains("tiktok.com")
}

fn format_brief(url: &str, brief: &PageBriefExtraction) -> String {
    let mut s = format!("### {}\n", url);
    s.push_str(&format!("{}\n", brief.summary));
    if !brief.locations.is_empty() {
        s.push_str(&format!("- Locations: {}\n", brief.locations.join(", ")));
    }
    if !brief.calls_to_action.is_empty() {
        s.push_str(&format!(
            "- Calls to action: {}\n",
            brief.calls_to_action.join("; ")
        ));
    }
    if let Some(info) = &brief.critical_info {
        s.push_str(&format!("- Critical: {}\n", info));
    }
    if !brief.services.is_empty() {
        s.push_str(&format!("- Services: {}\n", brief.services.join(", ")));
    }
    if !brief.contacts.is_empty() {
        let contact_strs: Vec<_> = brief
            .contacts
            .iter()
            .map(|c| {
                let label = c.label.as_deref().unwrap_or(&c.contact_type);
                format!("{}: {}", label, c.value)
            })
            .collect();
        s.push_str(&format!("- Contacts: {}\n", contact_strs.join(", ")));
    }
    if !brief.schedules.is_empty() {
        for sched in &brief.schedules {
            let mut line = format!(
                "- Schedule ({}): {}",
                sched.schedule_type, sched.description
            );
            if let Some(exc) = &sched.exceptions {
                line.push_str(&format!(" [{}]", exc));
            }
            if let Some(seasonal) = &sched.seasonal_notes {
                line.push_str(&format!(" ({})", seasonal));
            }
            s.push_str(&format!("{}\n", line));
        }
    }
    if !brief.languages_mentioned.is_empty() {
        s.push_str(&format!(
            "- Languages: {}\n",
            brief.languages_mentioned.join(", ")
        ));
    }
    if !brief.populations_mentioned.is_empty() {
        s.push_str(&format!(
            "- Populations: {}\n",
            brief.populations_mentioned.join(", ")
        ));
    }
    if let Some(cap) = &brief.capacity_info {
        s.push_str(&format!("- Capacity: {}\n", cap));
    }
    s.push('\n');
    s
}

fn format_existing_post(
    post: &Post,
    contacts: &[Contact],
    schedules: &[Schedule],
    tags: &[crate::domains::tag::models::Tag],
) -> String {
    let mut s = format!(
        "### [POST-{}] {} (status: {}, type: {})\n{}\n",
        Uuid::from(post.id),
        post.title,
        post.status,
        post.post_type,
        post.summary.as_deref().unwrap_or("No summary"),
    );
    if let Some(loc) = &post.location {
        s.push_str(&format!("- Location: {}\n", loc));
    }
    if let Some(urgency) = &post.urgency {
        s.push_str(&format!("- Urgency: {}\n", urgency));
    }
    if !contacts.is_empty() {
        let contact_strs: Vec<_> = contacts
            .iter()
            .map(|c| format!("{}: {}", c.contact_type, c.contact_value))
            .collect();
        s.push_str(&format!("- Contacts: {}\n", contact_strs.join(", ")));
    }
    if !schedules.is_empty() {
        let sched_strs: Vec<_> = schedules.iter().map(format_schedule_brief).collect();
        s.push_str(&format!("- Schedule: {}\n", sched_strs.join("; ")));
    }
    if !tags.is_empty() {
        let tag_strs: Vec<_> = tags.iter().map(|t| format!("{}:{}", t.kind, t.value)).collect();
        s.push_str(&format!("- Tags: {}\n", tag_strs.join(", ")));
    }
    s.push('\n');
    s
}

fn format_schedule_brief(sched: &Schedule) -> String {
    if let Some(ref rrule) = sched.rrule {
        if let (Some(opens), Some(closes)) = (sched.opens_at, sched.closes_at) {
            format!("{} {}-{}", rrule, opens, closes)
        } else {
            rrule.clone()
        }
    } else if let (Some(start), Some(end)) = (sched.dtstart, sched.dtend) {
        format!("{} to {}", start.format("%Y-%m-%d %H:%M"), end.format("%H:%M"))
    } else if let Some(start) = sched.dtstart {
        format!("{}", start.format("%Y-%m-%d %H:%M"))
    } else {
        "Unscheduled".to_string()
    }
}

fn format_note(note: &Note) -> String {
    format!(
        "### [NOTE-{}] (severity: {})\n{}\n\n",
        Uuid::from(note.id),
        note.severity,
        note.content
    )
}
