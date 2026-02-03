//! Integration tests for the Detective Engine loop.
//!
//! These tests verify the full detective workflow:
//! 1. Extract from initial pages
//! 2. Detect gaps
//! 3. Plan investigation
//! 4. Execute steps
//! 5. Merge results

use extraction::{
    types::extraction::{Extraction, GapQuery},
    Index, MemoryStore, PageCache,
    testing::MockAI,
    types::page::CachedPage,
};

/// Helper to create a test page.
fn test_page(url: &str, site: &str, content: &str) -> CachedPage {
    CachedPage::new(url, site, content)
}

/// Helper to set up a test index with pages.
async fn setup_index_with_pages(pages: Vec<CachedPage>) -> Index<MemoryStore, MockAI> {
    let store = MemoryStore::new();
    let ai = MockAI::new();

    for page in &pages {
        store.store_page(page).await.unwrap();
    }

    Index::new(store, ai)
}

#[tokio::test]
async fn test_plan_investigation_creates_steps_for_gaps() {
    let index = setup_index_with_pages(vec![]).await;

    // Create an extraction with gaps
    let mut extraction = Extraction::new("Board Members:\n- John Smith, CEO".to_string());
    extraction.gaps.push(GapQuery::new(
        "contact email",
        "the email address for John Smith",
    ));
    extraction.gaps.push(GapQuery::new(
        "phone number",
        "the phone number for the main office",
    ));

    // Plan investigation
    let plan = index.plan_investigation(&extraction);

    assert_eq!(plan.len(), 2);
    assert_eq!(plan.steps[0].field, "contact email");
    assert_eq!(plan.steps[1].field, "phone number");
}

#[tokio::test]
async fn test_gap_type_affects_semantic_weight() {
    let index = setup_index_with_pages(vec![]).await;

    // Entity gap (should use FTS-heavy search)
    let mut entity_extraction = Extraction::new("Content".to_string());
    entity_extraction.gaps.push(GapQuery::new(
        "email",
        "john@example.com email address",
    ));

    let entity_plan = index.plan_investigation(&entity_extraction);
    let entity_step = &entity_plan.steps[0];

    // Semantic gap (should use semantic-heavy search)
    let mut semantic_extraction = Extraction::new("Content".to_string());
    semantic_extraction.gaps.push(GapQuery::new(
        "mission",
        "what is their mission and purpose",
    ));

    let semantic_plan = index.plan_investigation(&semantic_extraction);
    let semantic_step = &semantic_plan.steps[0];

    // Verify different weights through the action
    match (&entity_step.recommended_action, &semantic_step.recommended_action) {
        (
            extraction::types::investigation::InvestigationAction::HybridSearch { semantic_weight: entity_weight, .. },
            extraction::types::investigation::InvestigationAction::HybridSearch { semantic_weight: semantic_weight, .. },
        ) => {
            assert!(entity_weight < semantic_weight,
                "Entity gaps should have lower semantic weight than semantic gaps");
        }
        _ => panic!("Expected HybridSearch actions"),
    }
}

#[tokio::test]
async fn test_extraction_merge_deduplicates_sources() {
    use extraction::types::extraction::Source;
    use chrono::Utc;

    let mut base = Extraction::new("Base content".to_string());
    base.sources.push(Source::primary("https://example.com/page1".to_string(), Utc::now()));

    let mut supplement = Extraction::new("Supplement content".to_string());
    // Add duplicate source
    supplement.sources.push(Source::supporting("https://example.com/page1".to_string(), Utc::now()));
    // Add new source
    supplement.sources.push(Source::supporting("https://example.com/page2".to_string(), Utc::now()));

    base.merge(supplement);

    // Should have 2 unique sources, not 3
    assert_eq!(base.sources.len(), 2);
    assert!(base.content.contains("Base content"));
    assert!(base.content.contains("Supplement content"));
}

#[tokio::test]
async fn test_extraction_merge_upgrades_grounding() {
    use extraction::types::extraction::{Source, GroundingGrade};
    use chrono::Utc;

    let mut base = Extraction::new("Base".to_string());
    base.sources.push(Source::primary("https://a.com".to_string(), Utc::now()));
    base.grounding = GroundingGrade::SingleSource;

    let mut supplement = Extraction::new("Supplement".to_string());
    supplement.sources.push(Source::supporting("https://b.com".to_string(), Utc::now()));

    base.merge(supplement);

    // With 2 sources, grounding should upgrade to Verified
    assert_eq!(base.grounding, GroundingGrade::Verified);
}

#[tokio::test]
async fn test_full_detective_loop_pattern() {
    let pages = vec![
        test_page(
            "https://example.org/about",
            "https://example.org",
            "Our organization helps communities.",
        ),
        test_page(
            "https://example.org/contact",
            "https://example.org",
            "Contact us at contact@example.org or call 555-0123.",
        ),
    ];

    let index = setup_index_with_pages(pages).await;

    // Initial extraction with a gap
    let mut extraction = Extraction::new("Organization info here".to_string());
    extraction.gaps.push(GapQuery::new(
        "contact info",
        "contact email and phone number",
    ));

    // Simulate detective loop (simplified)
    let mut iterations = 0;
    let max_iterations = 3;

    while extraction.has_gaps() && iterations < max_iterations {
        iterations += 1;

        let plan = index.plan_investigation(&extraction);
        if plan.is_empty() {
            break;
        }

        // In real usage, we'd execute steps and merge results
        // For this test, we just verify the loop structure works
        assert!(!plan.is_empty());
        break; // Exit after verifying structure
    }

    assert!(iterations > 0);
}

#[tokio::test]
async fn test_step_result_tracks_metadata() {
    use extraction::types::investigation::{InvestigationStep, InvestigationAction, StepResult};
    use uuid::Uuid;

    let step = InvestigationStep::new(
        Uuid::new_v4(),
        "email",
        "contact email",
        InvestigationAction::entity_search("contact email"),
    );

    let result = StepResult::success(step, vec!["https://example.com/contact".to_string()])
        .with_duration(50)
        .with_tokens(100);

    assert!(result.is_success());
    assert!(result.has_content());
    assert_eq!(result.pages_found.len(), 1);
    assert_eq!(result.duration_ms, Some(50));
    assert_eq!(result.tokens_used, Some(100));
}

#[tokio::test]
async fn test_empty_plan_for_no_gaps() {
    let index = setup_index_with_pages(vec![]).await;

    // Extraction with no gaps
    let extraction = Extraction::new("Complete content, no gaps.".to_string());

    let plan = index.plan_investigation(&extraction);

    assert!(plan.is_empty());
}
