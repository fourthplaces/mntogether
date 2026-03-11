//! Contact management activities for posts.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::contacts::{Contact, ContactType};

/// Add a single contact to a post.
pub async fn add_post_contact(
    post_id: Uuid,
    contact_type: &str,
    contact_value: String,
    contact_label: Option<String>,
    pool: &PgPool,
) -> Result<Contact> {
    let ct: ContactType = contact_type.parse()?;
    Contact::create(
        "post",
        post_id,
        ct,
        contact_value,
        contact_label,
        None,
        pool,
    )
    .await
}

/// Remove a single contact by its ID.
pub async fn remove_post_contact(contact_id: Uuid, pool: &PgPool) -> Result<()> {
    Contact::delete_by_id(contact_id, pool).await
}
