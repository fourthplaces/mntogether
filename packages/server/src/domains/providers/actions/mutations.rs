//! Provider mutation actions
//!
//! All provider write operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they handle ID parsing, auth checks, and return final models.
//! They emit events for cascade operations (e.g., ProviderDeleted triggers cleanup).

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

use crate::common::{AppState, ContactId, MemberId, ProviderId, TagId};
use crate::domains::contacts::Contact;
use crate::domains::providers::data::{SubmitProviderInput, UpdateProviderInput};
use crate::domains::providers::events::ProviderEvent;
use crate::domains::providers::models::{CreateProvider, Provider, UpdateProvider};
use crate::domains::tag::{Tag, Taggable};
use crate::kernel::ServerDeps;

/// Submit a new provider (goes to pending_review)
/// Returns the created Provider directly.
pub async fn submit_provider(
    input: SubmitProviderInput,
    member_id: Option<Uuid>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Provider> {
    info!(name = %input.name, "Submitting new provider");

    let member_id_typed = member_id.map(MemberId::from_uuid);

    let create_input = CreateProvider {
        name: input.name,
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

    let provider = Provider::create(create_input, &ctx.deps().db_pool).await?;

    // Emit event for observability (no cascade needed for create)
    ctx.emit(ProviderEvent::ProviderCreated {
        provider_id: provider.id,
        name: provider.name.clone(),
        submitted_by: member_id_typed,
    });

    info!(provider_id = %provider.id, "Provider submitted successfully");

    Ok(provider)
}

/// Update a provider (admin only)
/// Returns the updated Provider directly.
pub async fn update_provider(
    provider_id: String,
    input: UpdateProviderInput,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Provider> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

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

    let provider = Provider::update(id, update_input, &ctx.deps().db_pool).await?;

    Ok(provider)
}

/// Approve a provider (admin only)
/// Returns the updated Provider directly.
pub async fn approve_provider(
    provider_id: String,
    reviewed_by_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Provider> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    info!(provider_id = %id, reviewed_by = %reviewed_by, "Approving provider");

    let provider = Provider::approve(id, reviewed_by, &ctx.deps().db_pool).await?;

    // Emit event for observability (could trigger welcome email cascade later)
    ctx.emit(ProviderEvent::ProviderApproved {
        provider_id: id,
        reviewed_by,
    });

    Ok(provider)
}

/// Reject a provider (admin only)
/// Returns the updated Provider directly.
pub async fn reject_provider(
    provider_id: String,
    reason: String,
    reviewed_by_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Provider> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    info!(provider_id = %id, reason = %reason, "Rejecting provider");

    let provider = Provider::reject(id, reviewed_by, &reason, &ctx.deps().db_pool).await?;

    // Emit event for observability (could trigger notification cascade later)
    ctx.emit(ProviderEvent::ProviderRejected {
        provider_id: id,
        reviewed_by,
        reason,
    });

    Ok(provider)
}

/// Suspend a provider (admin only)
/// Returns the updated Provider directly.
pub async fn suspend_provider(
    provider_id: String,
    reason: String,
    reviewed_by_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Provider> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;
    let reviewed_by = MemberId::from_uuid(reviewed_by_id);

    info!(provider_id = %id, reason = %reason, "Suspending provider");

    let provider = Provider::suspend(id, reviewed_by, &reason, &ctx.deps().db_pool).await?;

    // Emit event for observability (could trigger notification cascade later)
    ctx.emit(ProviderEvent::ProviderSuspended {
        provider_id: id,
        reviewed_by,
        reason,
    });

    Ok(provider)
}

/// Add a tag to a provider (admin only)
pub async fn add_provider_tag(
    provider_id: String,
    tag_kind: String,
    tag_value: String,
    display_name: Option<String>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Tag> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, tag_kind = %tag_kind, tag_value = %tag_value, "Adding provider tag");

    let tag = Tag::find_or_create(&tag_kind, &tag_value, display_name, &ctx.deps().db_pool).await?;
    Taggable::create_provider_tag(id, tag.id, &ctx.deps().db_pool).await?;

    Ok(tag)
}

/// Remove a tag from a provider (admin only)
pub async fn remove_provider_tag(
    provider_id: String,
    tag_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let provider_id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;
    let tag_id = TagId::parse(&tag_id).context("Invalid tag ID")?;

    info!(provider_id = %provider_id, tag_id = %tag_id, "Removing provider tag");

    Taggable::delete_provider_tag(provider_id, tag_id, &ctx.deps().db_pool).await?;

    Ok(true)
}

/// Add a contact to a provider (admin only)
pub async fn add_provider_contact(
    provider_id: String,
    contact_type: String,
    contact_value: String,
    contact_label: Option<String>,
    is_public: Option<bool>,
    display_order: Option<i32>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Contact> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, contact_type = %contact_type, "Adding provider contact");

    let contact = Contact::create_for_provider(
        id,
        &contact_type,
        &contact_value,
        contact_label,
        is_public.unwrap_or(true),
        display_order.unwrap_or(0),
        &ctx.deps().db_pool,
    )
    .await?;

    Ok(contact)
}

/// Remove a contact from a provider (admin only)
pub async fn remove_provider_contact(
    contact_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ContactId::parse(&contact_id).context("Invalid contact ID")?;

    info!(contact_id = %id, "Removing provider contact");

    Contact::delete(id, &ctx.deps().db_pool).await?;

    Ok(true)
}

/// Delete a provider (admin only)
///
/// Emits ProviderDeleted event which triggers cascade cleanup of contacts and tags
/// via the provider effect handler. This keeps delete_provider focused on ONE thing
/// (deleting the provider record) while the effect handles cascading cleanup.
pub async fn delete_provider(
    provider_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, "Deleting provider");

    // Delete the provider record
    Provider::delete(id, &ctx.deps().db_pool).await?;

    // Emit event - effect will handle cascade cleanup (contacts, tags)
    ctx.emit(ProviderEvent::ProviderDeleted { provider_id: id });

    Ok(true)
}
