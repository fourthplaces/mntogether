use chrono::{DateTime, Utc};
use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};

use crate::domains::contacts::models::Contact;

/// Contact GraphQL data type
#[derive(Debug, Clone, Serialize, Deserialize, GraphQLObject)]
#[graphql(description = "Contact information for an entity (organization, listing, provider)")]
pub struct ContactData {
    /// Unique identifier
    pub id: String,

    /// Type of contact (phone, email, website, address, booking_url, social)
    pub contact_type: String,

    /// The contact value (e.g., phone number, email address, URL)
    pub contact_value: String,

    /// Optional label (e.g., 'Office', 'Mobile', 'LinkedIn')
    pub contact_label: Option<String>,

    /// Whether this contact is publicly visible
    pub is_public: bool,

    /// Display order for sorting
    pub display_order: i32,

    /// When the contact was created
    pub created_at: DateTime<Utc>,
}

impl From<Contact> for ContactData {
    fn from(contact: Contact) -> Self {
        Self {
            id: contact.id.to_string(),
            contact_type: contact.contact_type,
            contact_value: contact.contact_value,
            contact_label: contact.contact_label,
            is_public: contact.is_public,
            display_order: contact.display_order,
            created_at: contact.created_at,
        }
    }
}
