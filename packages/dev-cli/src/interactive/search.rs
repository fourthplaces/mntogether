//! Fuzzy search for menu items

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use super::types::InteractiveMenuItem;

/// Result of a fuzzy search match
#[derive(Debug)]
pub struct SearchResult {
    /// The matched item
    pub item: InteractiveMenuItem,
    /// Match score (higher = better match)
    pub score: i64,
}

/// Fuzzy search across menu items
pub struct MenuSearch {
    matcher: SkimMatcherV2,
}

impl Default for MenuSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuSearch {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Search items with a query, returning matches sorted by score
    pub fn search(&self, items: &[InteractiveMenuItem], query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return items
                .iter()
                .map(|item| SearchResult {
                    item: item.clone(),
                    score: 0,
                })
                .collect();
        }

        let query_lower = query.to_lowercase();
        let mut results: Vec<SearchResult> = items
            .iter()
            .filter_map(|item| {
                let searchable = item.searchable_text().to_lowercase();
                self.matcher
                    .fuzzy_match(&searchable, &query_lower)
                    .map(|score| SearchResult {
                        item: item.clone(),
                        score,
                    })
            })
            .collect();

        // Sort by score descending (best matches first)
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    /// Check if a single item matches the query
    pub fn matches(&self, item: &InteractiveMenuItem, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let searchable = item.searchable_text().to_lowercase();
        let query_lower = query.to_lowercase();
        self.matcher
            .fuzzy_match(&searchable, &query_lower)
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactive::types::WorkflowGroup;
    use crate::menu::MenuAction;

    fn make_item(id: &str, label: &str, keywords: &[&str]) -> InteractiveMenuItem {
        InteractiveMenuItem::new(id, label, MenuAction::Status, WorkflowGroup::Bootstrap)
            .with_keywords(keywords)
    }

    #[test]
    fn test_fuzzy_search() {
        let search = MenuSearch::new();
        let items = vec![
            make_item("docker:up", "Start containers", &["docker", "up"]),
            make_item("docker:stop", "Stop containers", &["docker", "down"]),
            make_item("dev:start", "Start dev environment", &["development"]),
        ];

        // Exact match
        let results = search.search(&items, "docker");
        assert_eq!(results.len(), 2);

        // Fuzzy match
        let results = search.search(&items, "strt");
        assert!(!results.is_empty());

        // No match
        let results = search.search(&items, "xyz123");
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_query() {
        let search = MenuSearch::new();
        let items = vec![make_item("a", "Alpha", &[]), make_item("b", "Beta", &[])];

        let results = search.search(&items, "");
        assert_eq!(results.len(), 2);
    }
}
