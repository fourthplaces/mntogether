use juniper::FieldResult;
use tracing::info;
use uuid::Uuid;

use crate::common::{ContactId, MemberId, ProviderId, TagId};
use crate::domains::contacts::{Contact, ContactData};
use crate::domains::providers::data::{ProviderData, SubmitProviderInput, UpdateProviderInput};
use crate::domains::providers::models::{CreateProvider, Provider, UpdateProvider};
use crate::domains::tag::{Tag, TagData, Taggable};
use crate::server::graphql::context::GraphQLContext;

/// Submit a new provider (goes to pending_review)
pub async fn submit_provider(
    ctx: &GraphQLContext,
    input: SubmitProviderInput,
    member_id: Option<Uuid>,
) -> FieldResult<ProviderData> {
    info!("submit_provider mutation called: {}", input.name);

    let create_input = CreateProvider {
        name: input.name,
        bio: input.bio,
        why_statement: input.why_statement,
        headline: input.headline,
        profile_image_url: input.profile_image_url,
        member_id: member_id.map(MemberId::from_uuid),
        website_id: None,
        location: input.location,
        latitude: input.latitude,
        longitude: input.longitude,
        service_radius_km: input.service_radius_km,
        offers_in_person: input.offers_in_person.unwrap_or(false),
        offers_remote: input.offers_remote.unwrap_or(false),
        accepting_clients: input.accepting_clients.unwrap_or(true),
        submitted_by: member_id.map(MemberId::from_uuid),
    };

    let provider = Provider::create(create_input, &ctx.db_pool).await?;

    Ok(ProviderData::from(provider))
}

/// Update a provider (admin only)
pub async fn update_provider(
    ctx: &GraphQLContext,
    provider_id: String,
    input: UpdateProviderInput,
) -> FieldResult<ProviderData> {
    info!("update_provider mutation called: {}", provider_id);

    let id = ProviderId::parse(&provider_id)?;

    let update_input = UpdateProvider {
        name: input.name,
        bio: input.bio,
        why_statement: input.why_statement,
        headline: input.headline,
        profile_image_url: input.profile_image_url,
        location: input.location,
        latitude: input.latitude,
        longitude: input.longitude,
        service_radius_km: input.service_radius_km,
        offers_in_person: input.offers_in_person,
        offers_remote: input.offers_remote,
        accepting_clients: input.accepting_clients,
    };

    let provider = Provider::update(id, update_input, &ctx.db_pool).await?;

    Ok(ProviderData::from(provider))
}

/// Approve a provider (admin only)
pub async fn approve_provider(
    ctx: &GraphQLContext,
    provider_id: String,
    reviewed_by_id: Uuid,
) -> FieldResult<ProviderData> {
    info!("approve_provider mutation called: {}", provider_id);

    let id = ProviderId::parse(&provider_id)?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    let provider = Provider::approve(id, reviewed_by, &ctx.db_pool).await?;

    Ok(ProviderData::from(provider))
}

/// Reject a provider (admin only)
pub async fn reject_provider(
    ctx: &GraphQLContext,
    provider_id: String,
    reason: String,
    reviewed_by_id: Uuid,
) -> FieldResult<ProviderData> {
    info!("reject_provider mutation called: {}", provider_id);

    let id = ProviderId::parse(&provider_id)?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    let provider = Provider::reject(id, reviewed_by, &reason, &ctx.db_pool).await?;

    Ok(ProviderData::from(provider))
}

/// Suspend a provider (admin only)
pub async fn suspend_provider(
    ctx: &GraphQLContext,
    provider_id: String,
    reason: String,
    reviewed_by_id: Uuid,
) -> FieldResult<ProviderData> {
    info!("suspend_provider mutation called: {}", provider_id);

    let id = ProviderId::parse(&provider_id)?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    let provider = Provider::suspend(id, reviewed_by, &reason, &ctx.db_pool).await?;

    Ok(ProviderData::from(provider))
}

/// Add a tag to a provider
pub async fn add_provider_tag(
    ctx: &GraphQLContext,
    provider_id: String,
    tag_kind: String,
    tag_value: String,
    display_name: Option<String>,
) -> FieldResult<TagData> {
    info!(
        "add_provider_tag mutation called: {} - {}:{}",
        provider_id, tag_kind, tag_value
    );

    let id = ProviderId::parse(&provider_id)?;

    // Find or create the tag
    let tag = Tag::find_or_create(&tag_kind, &tag_value, display_name, &ctx.db_pool).await?;

    // Associate with provider
    Taggable::create_provider_tag(id, tag.id, &ctx.db_pool).await?;

    Ok(TagData::from(tag))
}

/// Remove a tag from a provider
pub async fn remove_provider_tag(
    ctx: &GraphQLContext,
    provider_id: String,
    tag_id: String,
) -> FieldResult<bool> {
    info!(
        "remove_provider_tag mutation called: {} - {}",
        provider_id, tag_id
    );

    let provider_uuid = ProviderId::parse(&provider_id)?;
    let tag_uuid = TagId::parse(&tag_id)?;

    Taggable::delete_provider_tag(provider_uuid, tag_uuid, &ctx.db_pool).await?;

    Ok(true)
}

/// Add a contact to a provider
pub async fn add_provider_contact(
    ctx: &GraphQLContext,
    provider_id: String,
    contact_type: String,
    contact_value: String,
    contact_label: Option<String>,
    is_public: Option<bool>,
    display_order: Option<i32>,
) -> FieldResult<ContactData> {
    info!(
        "add_provider_contact mutation called: {} - {}:{}",
        provider_id, contact_type, contact_value
    );

    let id = ProviderId::parse(&provider_id)?;

    let contact = Contact::create_for_provider(
        id,
        &contact_type,
        &contact_value,
        contact_label,
        is_public.unwrap_or(true),
        display_order.unwrap_or(0),
        &ctx.db_pool,
    )
    .await?;

    Ok(ContactData::from(contact))
}

/// Remove a contact from a provider
pub async fn remove_provider_contact(
    ctx: &GraphQLContext,
    contact_id: String,
) -> FieldResult<bool> {
    info!("remove_provider_contact mutation called: {}", contact_id);

    let id = ContactId::parse(&contact_id)?;

    Contact::delete(id, &ctx.db_pool).await?;

    Ok(true)
}

/// Delete a provider (admin only)
pub async fn delete_provider(ctx: &GraphQLContext, provider_id: String) -> FieldResult<bool> {
    info!("delete_provider mutation called: {}", provider_id);

    let id = ProviderId::parse(&provider_id)?;

    // Delete associated contacts first
    Contact::delete_all_for_provider(id, &ctx.db_pool).await?;

    // Delete associated tags
    Taggable::delete_all_for_provider(id, &ctx.db_pool).await?;

    // Delete the provider
    Provider::delete(id, &ctx.db_pool).await?;

    Ok(true)
}
