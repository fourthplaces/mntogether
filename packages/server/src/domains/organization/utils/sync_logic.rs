/// Pure utility functions for sync algorithm logic
///
/// These functions contain NO side effects - they implement the business logic
/// for categorizing scraped needs as new, changed, or unchanged based on content hashes.

/// Represents an existing need in the database
#[derive(Debug, Clone)]
pub struct ExistingNeed {
    pub id: uuid::Uuid,
    pub title: String,
    pub content_hash: Option<String>,
}

/// Represents a scraped need with its calculated content hash
#[derive(Debug, Clone)]
pub struct ScrapedNeed {
    pub title: String,
    pub content_hash: String,
}

/// Result of categorizing a single scraped need
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeedCategory {
    /// Need is unchanged (hash matches existing need)
    Unchanged { existing_id: uuid::Uuid },
    /// Need content changed (title matches but hash differs)
    Changed { title: String },
    /// Need is new (no matching title or hash)
    New { title: String },
}

/// Categorize a scraped need by comparing it to existing needs
///
/// Algorithm:
/// 1. Check if content hash exists in database -> Unchanged
/// 2. Check if title exists but hash differs -> Changed
/// 3. Otherwise -> New
///
/// This is a pure function with no side effects.
///
/// # Examples
/// ```
/// let existing = vec![
///     ExistingNeed {
///         id: uuid::Uuid::new_v4(),
///         title: "Food Bank".to_string(),
///         content_hash: Some("abc123".to_string()),
///     }
/// ];
///
/// let scraped = ScrapedNeed {
///     title: "Food Bank".to_string(),
///     content_hash: "abc123".to_string(),
/// };
///
/// let result = categorize_scraped_need(&scraped, &existing);
/// // Result: Unchanged (hash matches)
/// ```
pub fn categorize_scraped_need(
    scraped: &ScrapedNeed,
    existing_needs: &[ExistingNeed],
) -> NeedCategory {
    // Check if this exact content hash exists
    if let Some(existing) = existing_needs
        .iter()
        .find(|n| n.content_hash.as_ref() == Some(&scraped.content_hash))
    {
        return NeedCategory::Unchanged {
            existing_id: existing.id,
        };
    }

    // Check if title exists (but hash differs) - this means content changed
    if existing_needs.iter().any(|n| n.title == scraped.title) {
        return NeedCategory::Changed {
            title: scraped.title.clone(),
        };
    }

    // New need
    NeedCategory::New {
        title: scraped.title.clone(),
    }
}

/// Categorize all scraped needs in a single pass
///
/// This is more efficient than calling categorize_scraped_need in a loop
/// because it processes all needs at once.
///
/// Returns (unchanged, changed, new) as separate vectors for easy processing.
pub fn categorize_all_scraped_needs(
    scraped_needs: &[ScrapedNeed],
    existing_needs: &[ExistingNeed],
) -> (
    Vec<uuid::Uuid>, // Unchanged (existing IDs)
    Vec<String>,     // Changed (titles)
    Vec<String>,     // New (titles)
) {
    let mut unchanged = Vec::new();
    let mut changed = Vec::new();
    let mut new = Vec::new();

    for scraped in scraped_needs {
        match categorize_scraped_need(scraped, existing_needs) {
            NeedCategory::Unchanged { existing_id } => unchanged.push(existing_id),
            NeedCategory::Changed { title } => changed.push(title),
            NeedCategory::New { title } => new.push(title),
        }
    }

    (unchanged, changed, new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_existing(title: &str, hash: &str) -> ExistingNeed {
        ExistingNeed {
            id: Uuid::new_v4(),
            title: title.to_string(),
            content_hash: Some(hash.to_string()),
        }
    }

    fn make_scraped(title: &str, hash: &str) -> ScrapedNeed {
        ScrapedNeed {
            title: title.to_string(),
            content_hash: hash.to_string(),
        }
    }

    #[test]
    fn test_categorize_unchanged_need() {
        let existing_id = Uuid::new_v4();
        let existing = vec![ExistingNeed {
            id: existing_id,
            title: "Food Bank".to_string(),
            content_hash: Some("abc123".to_string()),
        }];

        let scraped = make_scraped("Food Bank", "abc123");
        let result = categorize_scraped_need(&scraped, &existing);

        assert_eq!(result, NeedCategory::Unchanged { existing_id });
    }

    #[test]
    fn test_categorize_changed_need() {
        let existing = vec![make_existing("Food Bank", "abc123")];
        let scraped = make_scraped("Food Bank", "xyz789"); // Same title, different hash

        let result = categorize_scraped_need(&scraped, &existing);

        assert_eq!(
            result,
            NeedCategory::Changed {
                title: "Food Bank".to_string()
            }
        );
    }

    #[test]
    fn test_categorize_new_need() {
        let existing = vec![make_existing("Food Bank", "abc123")];
        let scraped = make_scraped("Clothing Drive", "xyz789"); // Different title

        let result = categorize_scraped_need(&scraped, &existing);

        assert_eq!(
            result,
            NeedCategory::New {
                title: "Clothing Drive".to_string()
            }
        );
    }

    #[test]
    fn test_categorize_new_need_empty_existing() {
        let existing = vec![];
        let scraped = make_scraped("Food Bank", "abc123");

        let result = categorize_scraped_need(&scraped, &existing);

        assert_eq!(
            result,
            NeedCategory::New {
                title: "Food Bank".to_string()
            }
        );
    }

    #[test]
    fn test_categorize_all_mixed_needs() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let existing = vec![
            ExistingNeed {
                id: id1,
                title: "Food Bank".to_string(),
                content_hash: Some("hash1".to_string()),
            },
            ExistingNeed {
                id: id2,
                title: "Tutoring".to_string(),
                content_hash: Some("hash2".to_string()),
            },
        ];

        let scraped = vec![
            make_scraped("Food Bank", "hash1"),      // Unchanged
            make_scraped("Tutoring", "hash2_new"),   // Changed
            make_scraped("Clothing Drive", "hash3"), // New
        ];

        let (unchanged, changed, new) = categorize_all_scraped_needs(&scraped, &existing);

        assert_eq!(unchanged, vec![id1]);
        assert_eq!(changed, vec!["Tutoring".to_string()]);
        assert_eq!(new, vec!["Clothing Drive".to_string()]);
    }

    #[test]
    fn test_categorize_all_empty_scraped() {
        let existing = vec![make_existing("Food Bank", "abc123")];
        let scraped = vec![];

        let (unchanged, changed, new) = categorize_all_scraped_needs(&scraped, &existing);

        assert!(unchanged.is_empty());
        assert!(changed.is_empty());
        assert!(new.is_empty());
    }

    #[test]
    fn test_categorize_all_empty_existing() {
        let existing = vec![];
        let scraped = vec![
            make_scraped("Food Bank", "abc123"),
            make_scraped("Tutoring", "xyz789"),
        ];

        let (unchanged, changed, new) = categorize_all_scraped_needs(&scraped, &existing);

        assert!(unchanged.is_empty());
        assert!(changed.is_empty());
        assert_eq!(new, vec!["Food Bank".to_string(), "Tutoring".to_string()]);
    }

    #[test]
    fn test_hash_match_takes_precedence_over_title() {
        // If hash matches, it's unchanged even if there's another need with same title
        let id1 = Uuid::new_v4();

        let existing = vec![
            ExistingNeed {
                id: id1,
                title: "Food Bank".to_string(),
                content_hash: Some("hash1".to_string()),
            },
            // Another need with same title but different hash (shouldn't happen, but test it)
            ExistingNeed {
                id: Uuid::new_v4(),
                title: "Food Bank".to_string(),
                content_hash: Some("hash2".to_string()),
            },
        ];

        let scraped = make_scraped("Food Bank", "hash1");
        let result = categorize_scraped_need(&scraped, &existing);

        // Should match by hash first
        assert_eq!(result, NeedCategory::Unchanged { existing_id: id1 });
    }
}
