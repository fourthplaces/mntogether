use anyhow::Result;
use juniper::FieldResult;
use seesaw_core::{dispatch_request, EnvelopeMatch};
use tracing::{error, info};
use uuid::Uuid;

use crate::domains::member::{data::MemberData, events::MemberEvent, models::member::Member};
use crate::server::graphql::context::GraphQLContext;

/// Register a new member
///
/// Takes city/state input, geocodes to lat/lng, creates member record
pub async fn register_member(
    expo_push_token: String,
    searchable_text: String,
    city: String,
    state: String,
    ctx: &GraphQLContext,
) -> FieldResult<MemberData> {
    info!(
        "register_member mutation called for city: {}, {}",
        city, state
    );

    // Dispatch request event and await response
    let member_id = dispatch_request(
        MemberEvent::RegisterMemberRequested {
            expo_push_token: expo_push_token.clone(),
            searchable_text,
            city,
            state,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &MemberEvent| match e {
                MemberEvent::MemberRegistered { member_id, .. } => Some(Ok(*member_id)),
                MemberEvent::RegistrationFailed { reason, .. } => {
                    error!(reason = %reason, "Member registration failed");
                    Some(Err(anyhow::anyhow!("Registration failed: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to register member");
        e
    })?;

    // Query the created member from database
    let member = Member::find_by_id(member_id, &ctx.db_pool).await?;

    Ok(MemberData::from(member))
}

/// Update member status (activate/deactivate)
pub async fn update_member_status(
    member_id: String,
    active: bool,
    ctx: &GraphQLContext,
) -> FieldResult<MemberData> {
    let member_id = Uuid::parse_str(&member_id)?;

    info!(
        "update_member_status mutation called: {} -> {}",
        member_id, active
    );

    // Dispatch request event
    dispatch_request(
        MemberEvent::UpdateMemberStatusRequested { member_id, active },
        &ctx.bus,
        |m| {
            m.try_match(|e: &MemberEvent| match e {
                MemberEvent::MemberStatusUpdated {
                    member_id: updated_id,
                    ..
                } if updated_id == &member_id => Some(Ok(())),
                MemberEvent::MemberNotFound {
                    member_id: not_found_id,
                } if not_found_id == &member_id => Some(Err(anyhow::anyhow!("Member not found"))),
                _ => None,
            })
            .result()
        },
    )
    .await?;

    // Query updated member
    let member = Member::find_by_id(member_id, &ctx.db_pool).await?;

    Ok(MemberData::from(member))
}
