//! Provider mutation actions
//!
//! Actions return events directly. GraphQL mutations call actions via `process()`
//! and the returned event is dispatched through the engine.

use anyhow::{Context, Result};
use tracing::info;
use uuid::Uuid;

use crate::common::{ContactId, MemberId, ProviderId, TagId};
use crate::domains::contacts::{Contact, CreateContactForProvider};
use crate::domains::providers::data::{SubmitProviderInput, UpdateProviderInput};
use crate::domains::providers::events::ProviderEvent;
use crate::domains::providers::models::{CreateProvider, Provider, UpdateProvider};
use crate::domains::tag::{Tag, Taggable};
use crate::kernel::ServerDeps;

/// Submit a new provider (goes to pending_review)
/// Returns ProviderCreated event with provider_id.
pub async fn submit_provider(
    input: SubmitProviderInput,
    member_id: Option<Uuid>,
    deps: &ServerDeps,
) -> Result<ProviderEvent> {
    info!(name = %input.name, "Submitting new provider");

    let member_id_typed = member_id.map(MemberId::from_uuid);

    let create_input = CreateProvider {
        name: input.name.clone(),
        bio: input.bio,
        why_statement: input.why_statement,
        headline: input.headline,
        profile_image_url: input.profile_image_url,
        member_id: member_id_typed,
        website_id: None,
        location: input.location,
        latitude: input.latitude,
        longitude: input.longitude,
        service_radius_km: input.service_radius_km,
        offers_in_person: input.offers_in_person.unwrap_or(false),
        offers_remote: input.offers_remote.unwrap_or(false),
        accepting_clients: input.accepting_clients.unwrap_or(true),
        submitted_by: member_id_typed,
    };

    let provider = Provider::create(create_input, &deps.db_pool).await?;

    info!(provider_id = %provider.id, "Provider submitted successfully");

    Ok(ProviderEvent::ProviderCreated {
        provider_id: provider.id,
        name: input.name,
        submitted_by: member_id_typed,
    })
}

/// Update a provider (admin only)
/// Returns the updated Provider directly (no event needed for updates).
pub async fn update_provider(
    provider_id: String,
    input: UpdateProviderInput,
    deps: &ServerDeps,
) -> Result<Provider> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, "Updating provider");

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

    let provider = Provider::update(id, update_input, &deps.db_pool).await?;

    Ok(provider)
}

/// Approve a provider (admin only)
/// Returns ProviderApproved event.
pub async fn approve_provider(
    provider_id: String,
    reviewed_by_id: Uuid,
    deps: &ServerDeps,
) -> Result<ProviderEvent> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    info!(provider_id = %id, reviewed_by = %reviewed_by, "Approving provider");

    Provider::approve(id, reviewed_by, &deps.db_pool).await?;

    Ok(ProviderEvent::ProviderApproved {
        provider_id: id,
        reviewed_by,
    })
}

/// Reject a provider (admin only)
/// Returns ProviderRejected event.
pub async fn reject_provider(
    provider_id: String,
    reason: String,
    reviewed_by_id: Uuid,
    deps: &ServerDeps,
) -> Result<ProviderEvent> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    info!(provider_id = %id, reason = %reason, "Rejecting provider");

    Provider::reject(id, reviewed_by, &reason, &deps.db_pool).await?;

    Ok(ProviderEvent::ProviderRejected {
        provider_id: id,
        reviewed_by,
        reason,
    })
}

/// Suspend a provider (admin only)
/// Returns ProviderSuspended event.
pub async fn suspend_provider(
    provider_id: String,
    reason: String,
    reviewed_by_id: Uuid,
    deps: &ServerDeps,
) -> Result<ProviderEvent> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    info!(provider_id = %id, reason = %reason, "Suspending provider");

    Provider::suspend(id, reviewed_by, &reason, &deps.db_pool).await?;

    Ok(ProviderEvent::ProviderSuspended {
        provider_id: id,
        reviewed_by,
        reason,
    })
}

/// Add a tag to a provider (admin only)
/// No event - direct CRUD operation.
pub async fn add_provider_tag(
    provider_id: String,
    tag_kind: String,
    tag_value: String,
    display_name: Option<String>,
    deps: &ServerDeps,
) -> Result<Tag> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, tag_kind = %tag_kind, tag_value = %tag_value, "Adding provider tag");

    let tag = Tag::find_or_create(&tag_kind, &tag_value, display_name, &deps.db_pool).await?;
    Taggable::create_provider_tag(id, tag.id, &deps.db_pool).await?;

    Ok(tag)
}

/// Remove a tag from a provider (admin only)
/// No event - direct CRUD operation.
pub async fn remove_provider_tag(
    provider_id: String,
    tag_id: String,
    deps: &ServerDeps,
) -> Result<bool> {
    let provider_id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;
    let tag_id = TagId::parse(&tag_id).context("Invalid tag ID")?;

    info!(provider_id = %provider_id, tag_id = %tag_id, "Removing provider tag");

    Taggable::delete_provider_tag(provider_id, tag_id, &deps.db_pool).await?;

    Ok(true)
}

/// Add a contact to a provider (admin only)
/// No event - direct CRUD operation.
pub async fn add_provider_contact(
    provider_id: String,
    contact_type: String,
    contact_value: String,
    contact_label: Option<String>,
    is_public: Option<bool>,
    display_order: Option<i32>,
    deps: &ServerDeps,
) -> Result<Contact> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, contact_type = %contact_type, "Adding provider contact");

    let contact = Contact::create_for_provider(
        CreateContactForProvider::builder()
            .provider_id(id)
            .contact_type(contact_type)
            .contact_value(contact_value)
            .contact_label(contact_label)
            .is_public(is_public.unwrap_or(true))
            .display_order(display_order.unwrap_or(0))
            .build(),
        &deps.db_pool,
    )
    .await?;

    Ok(contact)
}

/// Remove a contact from a provider (admin only)
/// No event - direct CRUD operation.
pub async fn remove_provider_contact(contact_id: String, deps: &ServerDeps) -> Result<bool> {
    let id = ContactId::parse(&contact_id).context("Invalid contact ID")?;

    info!(contact_id = %id, "Removing provider contact");

    Contact::delete(id, &deps.db_pool).await?;

    Ok(true)
}

/// Delete a provider (admin only)
///
/// Returns ProviderDeleted event which triggers cascade cleanup of contacts and tags
/// via the provider effect handler.
pub async fn delete_provider(provider_id: String, deps: &ServerDeps) -> Result<ProviderEvent> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, "Deleting provider");

    // Delete the provider record
    Provider::delete(id, &deps.db_pool).await?;

    // Return event - effect will handle cascade cleanup (contacts, tags)
    Ok(ProviderEvent::ProviderDeleted { provider_id: id })
}
