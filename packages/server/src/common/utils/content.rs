/// Pure utility functions for content manipulation
///
/// These functions contain NO side effects - they take inputs and return outputs
/// without touching databases, making API calls, or performing I/O.
/// This makes them easy to test and reason about.

/// Generate a summary from a description by truncation (fallback when AI is unavailable)
///
/// If the description is longer than max_length, it truncates to (max_length - 3)
/// and appends "..." for a total of max_length characters.
///
/// If shorter than max_length, returns the description unchanged.
pub fn generate_summary(description: &str, max_length: usize) -> String {
    if description.len() > max_length {
        let truncate_at = max_length.saturating_sub(3);
        format!("{}...", &description[..truncate_at])
    } else {
        description.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_summary_short_description() {
        let short_desc = "Short description";
        let summary = generate_summary(short_desc, 250);
        assert_eq!(summary, "Short description");
    }

    #[test]
    fn test_generate_summary_long_description() {
        let long_desc = "a".repeat(300);
        let summary = generate_summary(&long_desc, 250);
        assert_eq!(summary.len(), 250);
        assert!(summary.ends_with("..."));
        assert_eq!(&summary[..247], "a".repeat(247).as_str());
    }

    #[test]
    fn test_generate_summary_exact_length() {
        let exact_desc = "a".repeat(250);
        let summary = generate_summary(&exact_desc, 250);
        assert_eq!(summary.len(), 250);
        assert!(!summary.ends_with("...")); // Shouldn't truncate if exactly at limit
    }

    #[test]
    fn test_generate_summary_one_over_length() {
        let one_over = "a".repeat(251);
        let summary = generate_summary(&one_over, 250);
        assert_eq!(summary.len(), 250);
        assert!(summary.ends_with("..."));
    }
}
