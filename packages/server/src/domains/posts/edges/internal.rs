//! Internal edges for the Posts domain
//!
//! These functions react to fact events and emit new request events,
//! creating event chains. Following seesaw 0.3.0 architecture:
//!   Request Event → Effect → Fact Event → Internal Edge → Request Event → ...

use crate::domains::posts::events::PostEvent;

/// React to SourceScraped - trigger AI extraction
pub fn on_source_scraped(event: &PostEvent) -> Option<PostEvent> {
    match event {
        PostEvent::SourceScraped {
            source_id,
            job_id,
            organization_name,
            content,
            ..
        } => Some(PostEvent::ExtractPostsRequested {
            source_id: *source_id,
            job_id: *job_id,
            organization_name: organization_name.clone(),
            content: content.clone(),
        }),
        _ => None,
    }
}

/// React to PostsExtracted - trigger database sync
pub fn on_posts_extracted(event: &PostEvent) -> Option<PostEvent> {
    match event {
        PostEvent::PostsExtracted {
            source_id,
            job_id,
            posts,
        } => Some(PostEvent::SyncPostsRequested {
            source_id: *source_id,
            job_id: *job_id,
            posts: posts.clone(),
        }),
        _ => None,
    }
}

/// React to ResourceLinkScraped - trigger AI extraction for resource link
pub fn on_resource_link_scraped(event: &PostEvent) -> Option<PostEvent> {
    match event {
        PostEvent::ResourceLinkScraped {
            job_id,
            url,
            content,
            context,
            submitter_contact,
            ..
        } => Some(PostEvent::ExtractPostsFromResourceLinkRequested {
            job_id: *job_id,
            url: url.clone(),
            content: content.clone(),
            context: context.clone(),
            submitter_contact: submitter_contact.clone(),
        }),
        _ => None,
    }
}

/// React to ResourceLinkPostsExtracted - create posts from extracted data
pub fn on_resource_link_posts_extracted(event: &PostEvent) -> Option<PostEvent> {
    match event {
        PostEvent::ResourceLinkPostsExtracted {
            job_id,
            url,
            posts,
            context,
            submitter_contact,
        } => Some(PostEvent::CreatePostsFromResourceLinkRequested {
            job_id: *job_id,
            url: url.clone(),
            posts: posts.clone(),
            context: context.clone(),
            submitter_contact: submitter_contact.clone(),
        }),
        _ => None,
    }
}

/// React to WebsiteCreatedFromLink - trigger resource link scraping
pub fn on_website_created_from_link(event: &PostEvent) -> Option<PostEvent> {
    match event {
        PostEvent::WebsiteCreatedFromLink {
            job_id,
            url,
            submitter_contact,
            ..
        } => Some(PostEvent::ScrapeResourceLinkRequested {
            job_id: *job_id,
            url: url.clone(),
            context: None,
            submitter_contact: submitter_contact.clone(),
        }),
        _ => None,
    }
}
