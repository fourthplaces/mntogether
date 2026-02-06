//! Type-safe schema generation for OpenAI structured outputs.
//!
//! Uses the `schemars` crate to automatically generate JSON schemas from Rust types.
//!
//! # Example
//!
//! ```rust,ignore
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//! use openai_client::StructuredOutput;
//!
//! #[derive(Deserialize, JsonSchema)]
//! struct Post {
//!     title: String,
//!     description: String,
//! }
//!
//! #[derive(Deserialize, JsonSchema)]
//! struct ExtractionResponse {
//!     posts: Vec<Post>,
//! }
//!
//! // Get OpenAI-compatible schema
//! let schema = ExtractionResponse::openai_schema();
//! ```

use schemars::{schema_for, JsonSchema};
use serde::de::DeserializeOwned;

/// Trait for types that can be used as OpenAI structured output.
///
/// Automatically implemented for any type that implements `JsonSchema + DeserializeOwned`.
pub trait StructuredOutput: JsonSchema + DeserializeOwned {
    /// Generate an OpenAI-compatible JSON schema for this type.
    ///
    /// OpenAI requires:
    /// 1. `additionalProperties: false` on all object schemas
    /// 2. ALL properties listed in `required`, even nullable ones
    /// 3. Fully inlined schemas (no `$ref` references)
    ///
    /// This method transforms the schemars output to meet these requirements.
    fn openai_schema() -> serde_json::Value {
        let schema = schema_for!(Self);
        let mut value = serde_json::to_value(schema).unwrap_or_default();

        // Step 1: Fix all object schemas in definitions first
        fix_object_schemas(&mut value);

        // Step 2: Inline all $ref references (OpenAI doesn't follow refs properly)
        inline_refs(&mut value);

        // Step 3: Remove definitions section and $schema (OpenAI doesn't need them)
        if let serde_json::Value::Object(map) = &mut value {
            map.remove("definitions");
            map.remove("$schema");
        }

        value
    }

    /// Get the schema name for this type.
    fn type_name() -> String {
        <Self as JsonSchema>::schema_name()
    }
}

// Blanket implementation for all types that satisfy the bounds
impl<T: JsonSchema + DeserializeOwned> StructuredOutput for T {}

/// Fix all object schemas for OpenAI strict mode compatibility.
///
/// Adds `additionalProperties: false` and ensures all properties are in `required`.
fn fix_object_schemas(value: &mut serde_json::Value) {
    if let serde_json::Value::Object(map) = value {
        // If this is an object type schema
        if map.get("type") == Some(&serde_json::Value::String("object".to_string())) {
            // Add additionalProperties: false
            map.insert(
                "additionalProperties".to_string(),
                serde_json::Value::Bool(false),
            );

            // OpenAI requires ALL properties in required array
            if let Some(serde_json::Value::Object(props)) = map.get("properties") {
                let all_keys: Vec<serde_json::Value> = props
                    .keys()
                    .map(|k| serde_json::Value::String(k.clone()))
                    .collect();
                map.insert("required".to_string(), serde_json::Value::Array(all_keys));
            }
        }

        // Recurse into nested schemas
        for (_, v) in map.iter_mut() {
            fix_object_schemas(v);
        }
    } else if let serde_json::Value::Array(arr) = value {
        for item in arr.iter_mut() {
            fix_object_schemas(item);
        }
    }
}

/// Inline all $ref references by replacing them with the actual schema from definitions.
///
/// OpenAI's strict mode validation doesn't properly traverse $ref references,
/// so we need to inline them all.
fn inline_refs(value: &mut serde_json::Value) {
    // First, extract definitions if they exist
    let definitions = if let serde_json::Value::Object(map) = value {
        map.get("definitions").cloned()
    } else {
        None
    };

    if let Some(defs) = definitions {
        inline_refs_recursive(value, &defs);
    }
}

/// Recursively inline $ref references.
fn inline_refs_recursive(value: &mut serde_json::Value, definitions: &serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            // Check if this object has a $ref
            if let Some(serde_json::Value::String(ref_path)) = map.get("$ref").cloned() {
                // Parse ref like "#/definitions/ContactInfo"
                if ref_path.starts_with("#/definitions/") {
                    let type_name = ref_path.trim_start_matches("#/definitions/");
                    if let Some(def) = definitions.get(type_name) {
                        // Replace this object with the inlined definition
                        *value = def.clone();
                        // Recursively inline any nested refs in the inlined schema
                        inline_refs_recursive(value, definitions);
                        return;
                    }
                }
            }

            // Recurse into nested values
            for (_, v) in map.iter_mut() {
                inline_refs_recursive(v, definitions);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                inline_refs_recursive(item, definitions);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(Deserialize, JsonSchema)]
    struct TestPost {
        title: String,
        description: Option<String>,
    }

    #[derive(Deserialize, JsonSchema)]
    struct TestResponse {
        posts: Vec<TestPost>,
    }

    #[test]
    fn test_openai_schema_generation() {
        let schema = TestResponse::openai_schema();

        // Should be an object
        assert!(schema.is_object());

        // Should have required fields for root
        let schema_obj = schema.as_object().unwrap();
        assert!(schema_obj.contains_key("$schema") || schema_obj.contains_key("type"));
    }

    #[test]
    fn test_additional_properties_false() {
        let schema = TestResponse::openai_schema();
        let schema_str = serde_json::to_string(&schema).unwrap();

        // Should contain additionalProperties: false somewhere
        assert!(schema_str.contains("additionalProperties"));
    }

    #[test]
    fn test_nested_schema() {
        let schema = TestResponse::openai_schema();

        // The schema should have definitions for nested types
        assert!(schema.is_object());
    }

    #[test]
    fn test_all_properties_required() {
        // OpenAI requires ALL properties in required, even Option<T> fields
        #[derive(Deserialize, JsonSchema)]
        struct Contact {
            phone: Option<String>,
            email: Option<String>,
            name: String,
        }

        let schema = Contact::openai_schema();
        let schema_obj = schema.as_object().unwrap();

        // Should not have definitions (simple root type)
        assert!(
            !schema_obj.contains_key("definitions"),
            "Simple types should not have definitions"
        );

        // Should have properties at root level
        let properties = schema_obj.get("properties").unwrap().as_object().unwrap();
        assert!(
            properties.contains_key("phone"),
            "phone should be a property"
        );
        assert!(
            properties.contains_key("email"),
            "email should be a property"
        );
        assert!(properties.contains_key("name"), "name should be a property");

        // All three should be in required array at root level
        let required = schema_obj
            .get("required")
            .expect("should have required array")
            .as_array()
            .unwrap();
        let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();

        assert!(required_strs.contains(&"phone"), "phone should be required");
        assert!(required_strs.contains(&"email"), "email should be required");
        assert!(required_strs.contains(&"name"), "name should be required");
    }

    #[test]
    fn test_nested_struct_inlined() {
        // This replicates the actual ExtractedPostInformation/ContactInfo structure
        #[derive(Deserialize, JsonSchema)]
        struct ContactInfo {
            phone: Option<String>,
            email: Option<String>,
            website: Option<String>,
            intake_form_url: Option<String>,
            contact_name: Option<String>,
            other: Vec<String>,
        }

        #[derive(Deserialize, JsonSchema)]
        struct ExtractedPostInformation {
            contact: ContactInfo,
            location: Option<String>,
            urgency: String,
            confidence: String,
            audience_roles: Vec<String>,
        }

        let schema = ExtractedPostInformation::openai_schema();
        let schema_str = serde_json::to_string_pretty(&schema).unwrap();

        println!("Generated schema (inlined):\n{}", schema_str);

        let schema_obj = schema.as_object().unwrap();

        // Should NOT have definitions section (refs are inlined)
        assert!(
            !schema_obj.contains_key("definitions"),
            "Schema should NOT have definitions section - refs should be inlined"
        );

        // Should NOT have $schema (OpenAI doesn't need it)
        assert!(
            !schema_obj.contains_key("$schema"),
            "Schema should NOT have $schema field"
        );

        // Should have root-level properties
        assert!(
            schema_obj.contains_key("properties"),
            "Schema should have properties"
        );

        // The contact field should be inlined, not a $ref
        let properties = schema_obj.get("properties").unwrap().as_object().unwrap();
        let contact = properties.get("contact").unwrap().as_object().unwrap();

        // contact should NOT be a $ref
        assert!(
            !contact.contains_key("$ref"),
            "contact should be inlined, not a $ref"
        );

        // contact should have type: object (inlined ContactInfo)
        assert_eq!(
            contact.get("type"),
            Some(&serde_json::Value::String("object".to_string())),
            "contact should be an object type"
        );

        // contact should have additionalProperties: false
        assert_eq!(
            contact.get("additionalProperties"),
            Some(&serde_json::Value::Bool(false)),
            "contact should have additionalProperties: false"
        );

        // contact should have ALL properties in required
        let contact_props = contact.get("properties").unwrap().as_object().unwrap();
        let contact_required = contact
            .get("required")
            .expect("contact should have required array")
            .as_array()
            .unwrap();
        let required_strs: Vec<&str> = contact_required.iter().filter_map(|v| v.as_str()).collect();

        // All ContactInfo fields should be present
        assert!(
            contact_props.contains_key("contact_name"),
            "contact_name should be a property"
        );
        assert!(
            required_strs.contains(&"contact_name"),
            "contact_name should be in required array. Got: {:?}",
            required_strs
        );
        assert!(
            required_strs.contains(&"phone"),
            "phone should be in required"
        );
        assert!(
            required_strs.contains(&"email"),
            "email should be in required"
        );
        assert!(
            required_strs.contains(&"website"),
            "website should be in required"
        );
        assert!(
            required_strs.contains(&"intake_form_url"),
            "intake_form_url should be in required"
        );
        assert!(
            required_strs.contains(&"other"),
            "other should be in required"
        );
    }
}
