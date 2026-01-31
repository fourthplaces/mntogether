use chrono::{DateTime, Utc};
use juniper::{GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};

use crate::common::ProviderId;
use crate::domains::contacts::ContactData;
use crate::domains::providers::models::Provider;
use crate::domains::tag::TagData;
use crate::server::graphql::context::GraphQLContext;

/// Provider GraphQL data type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderData {
    pub id: String,

    // Profile
    pub name: String,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,

    // Links
    pub member_id: Option<String>,
    pub website_id: Option<String>,

    // Location
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,

    // Service modes
    pub offers_in_person: bool,
    pub offers_remote: bool,

    // Availability
    pub accepting_clients: bool,

    // Approval workflow
    pub status: String,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Provider> for ProviderData {
    fn from(provider: Provider) -> Self {
        Self {
            id: provider.id.to_string(),
            name: provider.name,
            bio: provider.bio,
            why_statement: provider.why_statement,
            headline: provider.headline,
            profile_image_url: provider.profile_image_url,
            member_id: provider.member_id.map(|id| id.to_string()),
            website_id: provider.website_id.map(|id| id.to_string()),
            location: provider.location,
            latitude: provider.latitude,
            longitude: provider.longitude,
            service_radius_km: provider.service_radius_km,
            offers_in_person: provider.offers_in_person,
            offers_remote: provider.offers_remote,
            accepting_clients: provider.accepting_clients,
            status: provider.status,
            reviewed_at: provider.reviewed_at,
            rejection_reason: provider.rejection_reason,
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ProviderData {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bio(&self) -> Option<&str> {
        self.bio.as_deref()
    }

    fn why_statement(&self) -> Option<&str> {
        self.why_statement.as_deref()
    }

    fn headline(&self) -> Option<&str> {
        self.headline.as_deref()
    }

    fn profile_image_url(&self) -> Option<&str> {
        self.profile_image_url.as_deref()
    }

    fn member_id(&self) -> Option<&str> {
        self.member_id.as_deref()
    }

    fn website_id(&self) -> Option<&str> {
        self.website_id.as_deref()
    }

    fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    fn latitude(&self) -> Option<f64> {
        self.latitude
    }

    fn longitude(&self) -> Option<f64> {
        self.longitude
    }

    fn service_radius_km(&self) -> Option<i32> {
        self.service_radius_km
    }

    fn offers_in_person(&self) -> bool {
        self.offers_in_person
    }

    fn offers_remote(&self) -> bool {
        self.offers_remote
    }

    fn accepting_clients(&self) -> bool {
        self.accepting_clients
    }

    fn status(&self) -> &str {
        &self.status
    }

    fn reviewed_at(&self) -> Option<DateTime<Utc>> {
        self.reviewed_at
    }

    fn rejection_reason(&self) -> Option<&str> {
        self.rejection_reason.as_deref()
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Get tags for this provider (categories, specialties, languages)
    async fn tags(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<TagData>> {
        use crate::domains::tag::Tag;

        let provider_id = ProviderId::parse(&self.id)?;
        let tags = Tag::find_for_provider(provider_id, &context.db_pool).await?;
        Ok(tags.into_iter().map(TagData::from).collect())
    }

    /// Get contacts for this provider
    async fn contacts(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<ContactData>> {
        use crate::domains::contacts::Contact;

        let provider_id = ProviderId::parse(&self.id)?;
        let contacts = Contact::find_public_for_provider(provider_id, &context.db_pool).await?;
        Ok(contacts.into_iter().map(ContactData::from).collect())
    }
}

/// Input for submitting a new provider
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct SubmitProviderInput {
    pub name: String,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,
    pub offers_in_person: Option<bool>,
    pub offers_remote: Option<bool>,
    pub accepting_clients: Option<bool>,
}

/// Input for updating a provider
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct UpdateProviderInput {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub why_statement: Option<String>,
    pub headline: Option<String>,
    pub profile_image_url: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub service_radius_km: Option<i32>,
    pub offers_in_person: Option<bool>,
    pub offers_remote: Option<bool>,
    pub accepting_clients: Option<bool>,
}

/// Provider status for filtering
#[derive(Debug, Clone, GraphQLObject)]
pub struct ProviderStatusData {
    pub status: String,
    pub count: i32,
}
