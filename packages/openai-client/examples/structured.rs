//! Structured output example with JSON schema

use openai_client::{OpenAIClient, StructuredRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::from_env()?;

    // Define a JSON schema for structured output
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The person's name"
            },
            "age": {
                "type": "integer",
                "description": "The person's age"
            },
            "occupation": {
                "type": "string",
                "description": "The person's job"
            }
        },
        "required": ["name", "age", "occupation"],
        "additionalProperties": false
    });

    let system = "Extract person information from text.";
    let user = "John Smith is a 35 year old software engineer.";

    let response = client
        .structured_output(StructuredRequest::new("gpt-4o", system, user, schema))
        .await?;

    println!("Structured output: {}", response);

    // Parse the JSON
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    println!("\nParsed:");
    println!("  Name: {}", parsed["name"]);
    println!("  Age: {}", parsed["age"]);
    println!("  Occupation: {}", parsed["occupation"]);

    Ok(())
}
