//! Extraction pipeline - the core of the library.
//!
//! The pipeline orchestrates:
//! - Strategy selection (Collection/Singular/Narrative)
//! - Recall (hybrid semantic + keyword search)
//! - Partition (for Collection strategy)
//! - Extraction with evidence grounding
//! - Conflict detection
//! - Ingest flow (crawl → summarize → store)

pub mod extract;
pub mod grounding;
pub mod index;
pub mod ingest;
pub mod partition;
pub mod prompts;
pub mod recall;
pub mod strategy;

pub use extract::{
    parse_extraction_response, transform_extraction, transform_narrative_response,
    transform_single_response, AIExtractionResponse, AINarrativeResponse, AISingleResponse,
    ExtractionTransformConfig,
};
pub use grounding::{calculate_grounding, Claim, ClaimGrounding, Evidence, GroundingConfig};
pub use index::Index;
pub use ingest::{ingest_urls_with_ingestor, ingest_with_ingestor, IngestResult, IngestorConfig};
pub use partition::{
    default_partition, merge_similar_partitions, parse_partition_response, split_large_partition,
    validate_partitions,
};
pub use prompts::{
    format_extract_prompt, format_partition_prompt, format_summarize_prompt, summarize_prompt_hash,
    EXTRACT_PROMPT, PARTITION_PROMPT, SUMMARIZE_PROMPT,
};
pub use recall::{hybrid_recall, RecallConfig};
pub use strategy::{classify_query, QueryAnalysis};
