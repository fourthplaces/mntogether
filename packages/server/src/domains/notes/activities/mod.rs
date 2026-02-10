pub mod attachment;
pub mod extraction;

pub use attachment::{attach_notes_to_org_posts, attach_org_notes_to_post};
pub use extraction::{extract_and_create_notes, generate_notes_for_organization, SourceContent};
