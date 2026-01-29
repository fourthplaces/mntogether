// Type-specific extraction schemas for AI-extracted listings
//
// These structs define what fields the AI should extract for each listing type.
// The AI returns JSON that deserializes into these structures.

use serde::{Deserialize, Serialize};

/// Envelope containing the listing type and type-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "listing_type", rename_all = "snake_case")]
pub enum ExtractedListingEnvelope {
    Service(ExtractedService),
    Opportunity(ExtractedOpportunity),
    Business(ExtractedBusiness),
}

/// Core fields shared across all listing types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedListingCore {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tldr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_info: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency: Option<String>, // "low" | "medium" | "high" | "urgent"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>, // "high" | "medium" | "low"
}

/// Service listing extraction (legal aid, healthcare, social services)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedService {
    #[serde(flatten)]
    pub core: ExtractedListingCore,

    // Accessibility & Requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_identification: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_appointment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walk_ins_accepted: Option<bool>,

    // Service Delivery
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_person_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_visits_available: Option<bool>,

    // Accessibility Features
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wheelchair_accessible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation_available: Option<bool>,

    // Cost Model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_service: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sliding_scale_fees: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepts_insurance: Option<bool>,

    // Hours
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evening_hours: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekend_hours: Option<bool>,
}

/// Opportunity listing extraction (volunteer, donation, partnership)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedOpportunity {
    #[serde(flatten)]
    pub core: ExtractedListingCore,

    // Opportunity Type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opportunity_type: Option<String>, // "volunteer" | "donation" | "customer" | "partnership" | "other"

    // Requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_commitment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_background_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_age: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills_needed: Option<Vec<String>>,

    // Logistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_ok: Option<bool>,
}

/// Business listing extraction (cause-driven commerce)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedBusiness {
    #[serde(flatten)]
    pub core: ExtractedListingCore,

    // Proceeds Information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proceeds_percentage: Option<f64>, // 0-100
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proceeds_beneficiary: Option<String>, // Organization name that receives proceeds

    // Support CTAs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub donation_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gift_card_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online_store_url: Option<String>,
}

impl ExtractedListingEnvelope {
    /// Get the core fields regardless of listing type
    pub fn core(&self) -> &ExtractedListingCore {
        match self {
            ExtractedListingEnvelope::Service(s) => &s.core,
            ExtractedListingEnvelope::Opportunity(o) => &o.core,
            ExtractedListingEnvelope::Business(b) => &b.core,
        }
    }

    /// Get the listing type as a string
    pub fn listing_type(&self) -> &'static str {
        match self {
            ExtractedListingEnvelope::Service(_) => "service",
            ExtractedListingEnvelope::Opportunity(_) => "opportunity",
            ExtractedListingEnvelope::Business(_) => "business",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_deserialization() {
        let json = r#"{
            "listing_type": "service",
            "organization_name": "Legal Aid Society",
            "title": "Free Immigration Legal Services",
            "description": "We provide free legal consultation and representation for immigration cases.",
            "tldr": "Free legal help for immigrants",
            "location": "Minneapolis, MN",
            "category": "legal",
            "contact_info": {"phone": "612-555-1234", "email": "help@legalaid.org"},
            "urgency": "high",
            "confidence": "high",
            "free_service": true,
            "requires_appointment": true,
            "remote_available": true,
            "in_person_available": true,
            "interpretation_available": true
        }"#;

        let result: ExtractedListingEnvelope = serde_json::from_str(json).unwrap();
        assert!(matches!(result, ExtractedListingEnvelope::Service(_)));
        assert_eq!(result.core().organization_name, "Legal Aid Society");
    }

    #[test]
    fn test_opportunity_deserialization() {
        let json = r#"{
            "listing_type": "opportunity",
            "organization_name": "Food Bank",
            "title": "Volunteer Food Sorters Needed",
            "description": "Help us sort and pack food donations every Saturday.",
            "tldr": "Weekend food sorting volunteers",
            "location": "St. Paul, MN",
            "category": "volunteer",
            "urgency": "medium",
            "confidence": "high",
            "opportunity_type": "volunteer",
            "time_commitment": "3 hours per week",
            "remote_ok": false,
            "minimum_age": 16
        }"#;

        let result: ExtractedListingEnvelope = serde_json::from_str(json).unwrap();
        assert!(matches!(result, ExtractedListingEnvelope::Opportunity(_)));
    }

    #[test]
    fn test_business_deserialization() {
        let json = r#"{
            "listing_type": "business",
            "organization_name": "Community Coffee Shop",
            "title": "Coffee for a Cause",
            "description": "Local coffee shop donating 10% of proceeds to immigrant support organizations.",
            "tldr": "Coffee shop supporting immigrants",
            "location": "Minneapolis, MN",
            "category": "food",
            "confidence": "medium",
            "proceeds_percentage": 10,
            "proceeds_beneficiary": "Immigrant Law Center",
            "online_store_url": "https://communitycoffee.shop"
        }"#;

        let result: ExtractedListingEnvelope = serde_json::from_str(json).unwrap();
        assert!(matches!(result, ExtractedListingEnvelope::Business(_)));
    }

    #[test]
    fn test_envelope_type_routing() {
        let service_json = r#"{"listing_type": "service", "organization_name": "Test", "title": "Test", "description": "Test"}"#;
        let service: ExtractedListingEnvelope = serde_json::from_str(service_json).unwrap();
        assert_eq!(service.listing_type(), "service");

        let opp_json = r#"{"listing_type": "opportunity", "organization_name": "Test", "title": "Test", "description": "Test"}"#;
        let opp: ExtractedListingEnvelope = serde_json::from_str(opp_json).unwrap();
        assert_eq!(opp.listing_type(), "opportunity");

        let biz_json = r#"{"listing_type": "business", "organization_name": "Test", "title": "Test", "description": "Test"}"#;
        let biz: ExtractedListingEnvelope = serde_json::from_str(biz_json).unwrap();
        assert_eq!(biz.listing_type(), "business");
    }
}
