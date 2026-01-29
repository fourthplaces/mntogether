use anyhow::Result;
use juniper::FieldResult;
use sqlx::PgPool;

use crate::common::ListingId;
use crate::domains::listings::data::ListingData;
use crate::domains::listings::models::Listing;

/// Get a listing by ID
pub async fn query_listing(pool: &PgPool, id: String) -> FieldResult<Option<ListingData>> {
    let listing_id = ListingId::parse(&id)?;

    match Listing::find_by_id(listing_id, pool).await {
        Ok(listing) => Ok(Some(ListingData::from(listing))),
        Err(_) => Ok(None),
    }
}

/// Get listings by type
pub async fn query_listings_by_type(
    pool: &PgPool,
    listing_type: String,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<Vec<ListingData>> {
    let limit = limit.unwrap_or(20).min(100) as i64;
    let offset = offset.unwrap_or(0) as i64;

    let listings = Listing::find_by_type(&listing_type, limit, offset, pool).await?;

    Ok(listings.into_iter().map(ListingData::from).collect())
}

/// Get listings by category
pub async fn query_listings_by_category(
    pool: &PgPool,
    category: String,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<Vec<ListingData>> {
    let limit = limit.unwrap_or(20).min(100) as i64;
    let offset = offset.unwrap_or(0) as i64;

    let listings = Listing::find_by_category(&category, limit, offset, pool).await?;

    Ok(listings.into_iter().map(ListingData::from).collect())
}

/// Get listings by status
pub async fn query_listings_by_status(
    pool: &PgPool,
    status: String,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<Vec<ListingData>> {
    let limit = limit.unwrap_or(20).min(100) as i64;
    let offset = offset.unwrap_or(0) as i64;

    let listings = Listing::find_by_status(&status, limit, offset, pool).await?;

    Ok(listings.into_iter().map(ListingData::from).collect())
}

/// Search listings by combining filters
pub async fn search_listings(
    pool: &PgPool,
    listing_type: Option<String>,
    category: Option<String>,
    capacity_status: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<Vec<ListingData>> {
    let limit = limit.unwrap_or(20).min(100) as i64;
    let offset = offset.unwrap_or(0) as i64;

    // Build dynamic query based on filters
    let mut query = "SELECT * FROM listings WHERE status = 'active'".to_string();
    let mut bind_values: Vec<String> = vec![];

    if let Some(lt) = listing_type {
        query.push_str(&format!(" AND listing_type = ${}", bind_values.len() + 1));
        bind_values.push(lt);
    }

    if let Some(cat) = category {
        query.push_str(&format!(" AND category = ${}", bind_values.len() + 1));
        bind_values.push(cat);
    }

    if let Some(cap) = capacity_status {
        query.push_str(&format!(" AND capacity_status = ${}", bind_values.len() + 1));
        bind_values.push(cap);
    }

    query.push_str(&format!(
        " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        bind_values.len() + 1,
        bind_values.len() + 2
    ));

    let mut query_builder = sqlx::query_as::<_, Listing>(&query);

    for value in bind_values {
        query_builder = query_builder.bind(value);
    }

    query_builder = query_builder.bind(limit).bind(offset);

    let listings = query_builder.fetch_all(pool).await?;

    Ok(listings.into_iter().map(ListingData::from).collect())
}
