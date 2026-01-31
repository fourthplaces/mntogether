use juniper::FieldResult;
use tracing::info;

use crate::common::ProviderId;
use crate::domains::providers::data::ProviderData;
use crate::domains::providers::models::Provider;
use crate::server::graphql::context::GraphQLContext;

/// Get a single provider by ID
pub async fn get_provider(
    ctx: &GraphQLContext,
    id: String,
) -> FieldResult<Option<ProviderData>> {
    info!("get_provider query called: {}", id);

    let provider_id = ProviderId::parse(&id)?;
    let provider = Provider::find_by_id_optional(provider_id, &ctx.db_pool).await?;

    Ok(provider.map(ProviderData::from))
}

/// Get all providers with optional filters
pub async fn get_providers(
    ctx: &GraphQLContext,
    status: Option<String>,
    accepting_clients: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<Vec<ProviderData>> {
    info!(
        "get_providers query called: status={:?}, accepting_clients={:?}",
        status, accepting_clients
    );

    let providers = Provider::find_with_filters(
        status.as_deref(),
        accepting_clients,
        limit,
        offset,
        &ctx.db_pool,
    )
    .await?;

    Ok(providers.into_iter().map(ProviderData::from).collect())
}

/// Get all pending providers (for admin approval queue)
pub async fn get_pending_providers(ctx: &GraphQLContext) -> FieldResult<Vec<ProviderData>> {
    info!("get_pending_providers query called");

    let providers = Provider::find_pending(&ctx.db_pool).await?;

    Ok(providers.into_iter().map(ProviderData::from).collect())
}

/// Get all approved providers
pub async fn get_approved_providers(ctx: &GraphQLContext) -> FieldResult<Vec<ProviderData>> {
    info!("get_approved_providers query called");

    let providers = Provider::find_approved(&ctx.db_pool).await?;

    Ok(providers.into_iter().map(ProviderData::from).collect())
}
