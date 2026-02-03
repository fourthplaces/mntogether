//! AI implementations for the extraction library.
//!
//! This module provides reference implementations of the `AI` trait.
//! Users can use these directly or implement their own.

#[cfg(feature = "openai")]
mod openai;

#[cfg(feature = "openai")]
pub use openai::OpenAI;
