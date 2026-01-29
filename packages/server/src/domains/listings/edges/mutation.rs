use juniper::FieldResult;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{ListingId, TagId};
use crate::domains::listings::data::ListingData;
use crate::domains::listings::models::Listing;
use crate::kernel::tag::{Tag, Taggable};

/// Input for creating a new listing
#[derive(Debug, Clone, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct CreateListingInput {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub tldr: Option<String>,
    pub listing_type: String, // 'service', 'opportunity', 'business'
    pub category: String,
    pub capacity_status: Option<String>,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub source_language: Option<String>,
}

/// Input for adding tags to a listing
#[derive(Debug, Clone, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct AddListingTagsInput {
    pub listing_id: String,
    pub tags: Vec<TagInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct TagInput {
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
}

/// Create a new listing
pub async fn create_listing(
    pool: &PgPool,
    input: CreateListingInput,
) -> FieldResult<ListingData> {
    let listing = Listing::create(
        input.organization_name,
        input.title,
        input.description,
        input.tldr,
        input.listing_type,
        input.category,
        input.capacity_status,
        input.urgency,
        input.location,
        "pending_approval".to_string(), // Default status
        None,                            // content_hash
        input.source_language.unwrap_or_else(|| "en".to_string()),
        Some("admin".to_string()), // submission_type
        None,                      // submitted_by_admin_id
        None,                      // domain_id
        None,                      // source_url
        None,                      // organization_id
        pool,
    )
    .await?;

    Ok(ListingData::from(listing))
}

/// Update listing status
pub async fn update_listing_status(
    pool: &PgPool,
    listing_id: String,
    status: String,
) -> FieldResult<ListingData> {
    let id = ListingId::parse(&listing_id)?;
    let listing = Listing::update_status(id, &status, pool).await?;

    Ok(ListingData::from(listing))
}

/// Update listing capacity status
pub async fn update_listing_capacity(
    pool: &PgPool,
    listing_id: String,
    capacity_status: String,
) -> FieldResult<ListingData> {
    let id = ListingId::parse(&listing_id)?;
    let listing = Listing::update_capacity_status(id, &capacity_status, pool).await?;

    Ok(ListingData::from(listing))
}

/// Mark listing as verified
pub async fn verify_listing(pool: &PgPool, listing_id: String) -> FieldResult<ListingData> {
    let id = ListingId::parse(&listing_id)?;
    let listing = Listing::mark_verified(id, pool).await?;

    Ok(ListingData::from(listing))
}

/// Add tags to a listing
pub async fn add_listing_tags(
    pool: &PgPool,
    input: AddListingTagsInput,
) -> FieldResult<ListingData> {
    let listing_id = ListingId::parse(&input.listing_id)?;

    for tag_input in input.tags {
        let tag = Tag::find_or_create(
            &tag_input.kind,
            &tag_input.value,
            tag_input.display_name,
            pool,
        )
        .await?;

        let _ = Taggable::create_listing_tag(listing_id, tag.id, pool).await;
    }

    let listing = Listing::find_by_id(listing_id, pool).await?;
    Ok(ListingData::from(listing))
}

/// Delete a listing
pub async fn delete_listing(pool: &PgPool, listing_id: String) -> FieldResult<bool> {
    let id = ListingId::parse(&listing_id)?;
    Listing::delete(id, pool).await?;

    Ok(true)
}
