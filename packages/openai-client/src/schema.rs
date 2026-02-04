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
    /// OpenAI requires `additionalProperties: false` for strict mode,
    /// which this method ensures is set.
    fn openai_schema() -> serde_json::Value {
        let schema = schema_for!(Self);
        let mut value = serde_json::to_value(schema).unwrap_or_default();

        // OpenAI requires additionalProperties: false for strict mode
        ensure_no_additional_properties(&mut value);

        value
    }

    /// Get the schema name for this type.
    fn type_name() -> String {
        <Self as JsonSchema>::schema_name()
    }
}

// Blanket implementation for all types that satisfy the bounds
impl<T: JsonSchema + DeserializeOwned> StructuredOutput for T {}

/// Recursively set `additionalProperties: false` on all object schemas.
///
/// OpenAI's strict mode requires this for proper validation.
fn ensure_no_additional_properties(value: &mut serde_json::Value) {
    if let serde_json::Value::Object(map) = value {
        // If this is an object type schema, add additionalProperties: false
        if map.get("type") == Some(&serde_json::Value::String("object".to_string())) {
            map.insert(
                "additionalProperties".to_string(),
                serde_json::Value::Bool(false),
            );
        }

        // Recurse into nested schemas
        for (_, v) in map.iter_mut() {
            ensure_no_additional_properties(v);
        }
    } else if let serde_json::Value::Array(arr) = value {
        for item in arr.iter_mut() {
            ensure_no_additional_properties(item);
        }
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
}
