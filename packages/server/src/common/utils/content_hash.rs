use sha2::{Digest, Sha256};

/// Generate a content hash for duplicate detection
///
/// Uses SHA256 of normalized text to detect when content has changed.
/// Normalization rules:
/// - Convert to lowercase
/// - Remove all non-alphanumeric characters (except spaces)
/// - Collapse multiple spaces into single spaces
/// - Trim leading/trailing whitespace
///
/// This makes the hash robust against minor formatting changes while
/// still detecting meaningful content changes.
pub fn generate_content_hash(text: &str) -> String {
    // Normalize text
    let normalized = text
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Generate SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_text_same_hash() {
        let text1 = "We need Spanish-speaking volunteers!";
        let text2 = "We need Spanish-speaking volunteers!";

        assert_eq!(
            generate_content_hash(text1),
            generate_content_hash(text2)
        );
    }

    #[test]
    fn test_case_insensitive() {
        let text1 = "We need Spanish-speaking volunteers!";
        let text2 = "WE NEED SPANISH-SPEAKING VOLUNTEERS!";
        let text3 = "we need spanish speaking volunteers";

        let hash1 = generate_content_hash(text1);
        let hash2 = generate_content_hash(text2);
        let hash3 = generate_content_hash(text3);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_punctuation_ignored() {
        let text1 = "We need Spanish-speaking volunteers!";
        let text2 = "We need Spanish speaking volunteers";
        let text3 = "We need Spanish-speaking volunteers!!!";

        let hash1 = generate_content_hash(text1);
        let hash2 = generate_content_hash(text2);
        let hash3 = generate_content_hash(text3);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_whitespace_normalized() {
        let text1 = "We need Spanish-speaking volunteers";
        let text2 = "We    need    Spanish-speaking    volunteers";
        let text3 = "  We need Spanish-speaking volunteers  ";

        let hash1 = generate_content_hash(text1);
        let hash2 = generate_content_hash(text2);
        let hash3 = generate_content_hash(text3);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_different_content_different_hash() {
        let text1 = "We need Spanish-speaking volunteers";
        let text2 = "We need French-speaking volunteers";

        assert_ne!(
            generate_content_hash(text1),
            generate_content_hash(text2)
        );
    }

    #[test]
    fn test_word_order_matters() {
        let text1 = "Volunteers needed for food distribution";
        let text2 = "Food distribution volunteers needed";

        // Word order DOES matter - these should have different hashes
        assert_ne!(
            generate_content_hash(text1),
            generate_content_hash(text2)
        );
    }

    #[test]
    fn test_hash_format() {
        let text = "Test content";
        let hash = generate_content_hash(text);

        // SHA256 hash should be 64 hex characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_empty_string() {
        let hash = generate_content_hash("");
        assert_eq!(hash.len(), 64); // Still produces valid hash
    }

    #[test]
    fn test_special_characters_only() {
        let text1 = "!!!???...";
        let text2 = "---***===";

        // Both should normalize to empty string
        assert_eq!(
            generate_content_hash(text1),
            generate_content_hash(text2)
        );
    }
}
