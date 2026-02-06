/// Pure utility functions for content manipulation
///
/// These functions contain NO side effects - they take inputs and return outputs
/// without touching databases, making API calls, or performing I/O.
/// This makes them easy to test and reason about.

/// Generate a TLDR (Too Long; Didn't Read) summary from a description
///
/// If the description is longer than max_length, it truncates to (max_length - 3)
/// and appends "..." for a total of max_length characters.
///
/// If shorter than max_length, returns the description unchanged.
pub fn generate_tldr(description: &str, max_length: usize) -> String {
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
    fn test_generate_tldr_short_description() {
        let short_desc = "Short description";
        let tldr = generate_tldr(short_desc, 100);
        assert_eq!(tldr, "Short description");
    }

    #[test]
    fn test_generate_tldr_long_description() {
        let long_desc = "a".repeat(150);
        let tldr = generate_tldr(&long_desc, 100);
        assert_eq!(tldr.len(), 100);
        assert!(tldr.ends_with("..."));
        assert_eq!(&tldr[..97], "a".repeat(97).as_str());
    }

    #[test]
    fn test_generate_tldr_exact_length() {
        let exact_desc = "a".repeat(100);
        let tldr = generate_tldr(&exact_desc, 100);
        assert_eq!(tldr.len(), 100);
        assert!(!tldr.ends_with("...")); // Shouldn't truncate if exactly at limit
    }

    #[test]
    fn test_generate_tldr_one_over_length() {
        let one_over = "a".repeat(101);
        let tldr = generate_tldr(&one_over, 100);
        assert_eq!(tldr.len(), 100);
        assert!(tldr.ends_with("..."));
    }
}
