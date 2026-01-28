/// Pure utility functions for content manipulation
///
/// These functions contain NO side effects - they take inputs and return outputs
/// without touching databases, making API calls, or performing I/O.
/// This makes them easy to test and reason about.
use crate::common::utils::generate_content_hash as hash_impl;

/// Generate a TLDR (Too Long; Didn't Read) summary from a description
///
/// If the description is longer than max_length, it truncates to (max_length - 3)
/// and appends "..." for a total of max_length characters.
///
/// If shorter than max_length, returns the description unchanged.
///
/// # Examples
/// ```
/// let short = "Short text";
/// assert_eq!(generate_tldr(short, 100), "Short text");
///
/// let long = "a".repeat(150);
/// let tldr = generate_tldr(&long, 100);
/// assert_eq!(tldr.len(), 100);
/// assert!(tldr.ends_with("..."));
/// ```
pub fn generate_tldr(description: &str, max_length: usize) -> String {
    if description.len() > max_length {
        let truncate_at = max_length.saturating_sub(3);
        format!("{}...", &description[..truncate_at])
    } else {
        description.to_string()
    }
}

/// Generate a content hash for a need (title + description + organization)
///
/// Uses the common hash implementation which normalizes text before hashing.
/// This is used for duplicate detection and change detection.
///
/// The hash combines:
/// - Title
/// - Description
/// - Organization name
///
/// All in a single string with space separators.
///
/// # Examples
/// ```
/// let hash1 = generate_need_content_hash("Help Needed", "We need volunteers", "Org Name");
/// let hash2 = generate_need_content_hash("Help Needed", "We need volunteers", "Org Name");
/// assert_eq!(hash1, hash2); // Same content = same hash
///
/// let hash3 = generate_need_content_hash("Different", "We need volunteers", "Org Name");
/// assert_ne!(hash1, hash3); // Different content = different hash
/// ```
pub fn generate_need_content_hash(title: &str, description: &str, org_name: &str) -> String {
    hash_impl(&format!("{} {} {}", title, description, org_name))
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

    #[test]
    fn test_generate_need_content_hash_consistency() {
        let hash1 = generate_need_content_hash("Help Needed", "We need volunteers", "Test Org");
        let hash2 = generate_need_content_hash("Help Needed", "We need volunteers", "Test Org");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_need_content_hash_different_title() {
        let hash1 = generate_need_content_hash("Help Needed", "We need volunteers", "Test Org");
        let hash2 = generate_need_content_hash("Different Title", "We need volunteers", "Test Org");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_need_content_hash_different_description() {
        let hash1 = generate_need_content_hash("Help Needed", "We need volunteers", "Test Org");
        let hash2 = generate_need_content_hash("Help Needed", "Different description", "Test Org");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_need_content_hash_different_org() {
        let hash1 = generate_need_content_hash("Help Needed", "We need volunteers", "Test Org");
        let hash2 =
            generate_need_content_hash("Help Needed", "We need volunteers", "Different Org");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_need_content_hash_format() {
        let hash = generate_need_content_hash("Test", "Description", "Org");
        // SHA256 produces 64 hex characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_need_content_hash_case_insensitive() {
        // The underlying hash implementation normalizes case
        let hash1 = generate_need_content_hash("Help Needed", "We need volunteers", "Test Org");
        let hash2 = generate_need_content_hash("HELP NEEDED", "WE NEED VOLUNTEERS", "TEST ORG");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_need_content_hash_punctuation_normalized() {
        // The underlying hash implementation normalizes punctuation
        let hash1 = generate_need_content_hash("Help Needed!", "We need volunteers.", "Test Org");
        let hash2 = generate_need_content_hash("Help Needed", "We need volunteers", "Test Org");
        assert_eq!(hash1, hash2);
    }
}
