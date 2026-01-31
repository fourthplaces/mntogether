use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use server_core::config::Config;
use server_core::domains::organization::models::Organization;
use server_core::kernel::tag::{Tag, Taggable};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct SeedData {
    organizations: Vec<OrgInput>,
}

#[derive(Debug, Deserialize)]
struct OrgInput {
    name: String,
    website: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    populations_served: String,
    county: Option<String>,
    #[allow(dead_code)]
    employees: Option<i32>,
    #[allow(dead_code)]
    year_founded: Option<i32>,
    #[allow(dead_code)]
    volunteers_needed: bool,
    #[allow(dead_code)]
    ice_resistance_focus: bool,
    #[allow(dead_code)]
    sources: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractedTags {
    services: Vec<String>,
    languages: Vec<String>,
    communities: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = Config::from_env()?;

    // Connect to database
    let pool = PgPool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    println!("✓ Connected to database");

    // Read seed data
    let json_data = std::fs::read_to_string("data/immigrant_resources_seed.json")
        .context("Failed to read seed data file")?;
    let seed_data: SeedData =
        serde_json::from_str(&json_data).context("Failed to parse seed data")?;

    println!(
        "✓ Loaded {} organizations from JSON",
        seed_data.organizations.len()
    );

    // Initialize OpenAI client for tag extraction
    let openai_api_key = &config.openai_api_key;

    println!("\n🚀 Starting seed process...\n");

    let mut created_count = 0;
    let mut skipped_count = 0;

    for (idx, org_input) in seed_data.organizations.iter().enumerate() {
        println!(
            "[{}/{}] Processing: {}",
            idx + 1,
            seed_data.organizations.len(),
            org_input.name
        );

        // Check if organization already exists
        if let Ok(Some(_)) = Organization::find_by_name(&org_input.name, &pool).await {
            println!("  ⊘ Skipping (already exists)");
            skipped_count += 1;
            continue;
        }

        // Extract city from address
        let city = extract_city(&org_input.address, &org_input.county);

        // Extract tags using AI
        let tags = extract_tags_with_ai(&org_input.populations_served, &openai_api_key)
            .await
            .unwrap_or_else(|e| {
                eprintln!("  ⚠ Failed to extract tags: {}", e);
                ExtractedTags {
                    services: vec!["general".to_string()],
                    languages: vec!["english".to_string()],
                    communities: vec!["general".to_string()],
                }
            });

        println!("  → Services: {:?}", tags.services);
        println!("  → Languages: {:?}", tags.languages);
        println!("  → Communities: {:?}", tags.communities);

        // Create organization
        let primary_address = match (&org_input.address, city.as_ref()) {
            (Some(addr), _) => Some(format!("{}, MN", addr)),
            (None, Some(c)) => Some(format!("{}, MN", c)),
            _ => org_input.county.clone().map(|c| format!("{}, MN", c)),
        };

        let summary = format!(
            "Services: {}. Languages: {}. Communities served: {}",
            tags.services.join(", "),
            tags.languages.join(", "),
            tags.communities.join(", ")
        );

        use server_core::domains::organization::models::CreateOrganization;

        let builder = CreateOrganization::builder()
            .name(org_input.name.clone())
            .description(Some(org_input.populations_served.clone()))
            .summary(Some(summary))
            .website(org_input.website.clone())
            .phone(org_input.phone.clone())
            .primary_address(primary_address)
            .organization_type(Some("nonprofit".to_string()))
            .build();

        let org = Organization::create(builder, &pool)
            .await
            .context("Failed to create organization")?;

        // Create and associate tags
        let mut tag_count = 0;

        // Service tags
        for service in &tags.services {
            if let Ok(tag) = Tag::find_or_create("service", service, None, &pool).await {
                let _ = Taggable::create_organization_tag(org.id, tag.id, &pool).await;
                tag_count += 1;
            }
        }

        // Language tags
        for language in &tags.languages {
            if let Ok(tag) = Tag::find_or_create("language", language, None, &pool).await {
                let _ = Taggable::create_organization_tag(org.id, tag.id, &pool).await;
                tag_count += 1;
            }
        }

        // Community tags
        for community in &tags.communities {
            if let Ok(tag) = Tag::find_or_create("community", community, None, &pool).await {
                let _ = Taggable::create_organization_tag(org.id, tag.id, &pool).await;
                tag_count += 1;
            }
        }

        println!("  ✓ Created organization with {} tags", tag_count);
        created_count += 1;
    }

    println!("\n✨ Seed complete!");
    println!("   Created: {}", created_count);
    println!("   Skipped: {}", skipped_count);
    println!("   Total: {}", seed_data.organizations.len());

    Ok(())
}

fn extract_city(address: &Option<String>, county: &Option<String>) -> Option<String> {
    if let Some(addr) = address {
        // Try to extract city from address
        // Format: "street, city, state zip"
        let parts: Vec<&str> = addr.split(',').collect();
        if parts.len() >= 2 {
            let city_part = parts[1].trim();
            // Remove state and zip
            let city = city_part.split_whitespace().next().unwrap_or(city_part);
            return Some(city.to_string());
        }
    }

    // Fallback to county
    county.clone().map(|c| format!("{} County", c))
}

fn create_contact_info(phone: &Option<String>, website: &Option<String>) -> JsonValue {
    let mut contact = HashMap::new();
    if let Some(p) = phone {
        contact.insert("phone".to_string(), JsonValue::String(p.clone()));
    }
    if let Some(w) = website {
        contact.insert("website".to_string(), JsonValue::String(w.clone()));
    }
    serde_json::to_value(contact).expect(
        "Failed to serialize contact info - this should never fail for HashMap<String, String>",
    )
}

async fn extract_tags_with_ai(description: &str, api_key: &str) -> Result<ExtractedTags> {
    use reqwest::Client;

    let client = Client::new();

    let prompt = format!(
        r#"Extract structured tags from this organization description.

Description: "{}"

Return ONLY a JSON object with these exact fields (arrays of lowercase strings):
- services: service types like "food_assistance", "housing_assistance", "legal_services", "employment_support", "emergency_financial_aid", "shelter", "utility_assistance"
- languages: languages supported like "english", "spanish", "somali", "hmong", "karen", "vietnamese", "arabic"
- communities: communities served like "general", "latino", "somali", "hmong", "karen", "vietnamese", "east_african", "native_american"

Be generous - include all that could be inferred. Use "general" as default for communities if not specified.

OUTPUT FORMAT REQUIREMENTS:
- Return ONLY raw JSON
- NO markdown code blocks (no ```json)
- NO backticks
- NO explanation
- Your entire response must be parseable by JSON.parse()
- Start with {{ and end with }}

Example:
{{"services": ["food_assistance"], "languages": ["english", "spanish"], "communities": ["general"]}}"#,
        description
    );

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "gpt-4-turbo-preview",
            "max_tokens": 500,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a tag extraction assistant. Return only valid JSON."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .context("No content in response")?;

    let tags: ExtractedTags =
        serde_json::from_str(content).context("Failed to parse extracted tags")?;

    Ok(tags)
}
