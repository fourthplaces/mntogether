//! Unit tests for content hash generation and sync logic.

use server_core::common::utils::generate_content_hash;

#[test]
fn identical_text_produces_same_hash() {
    let text1 = "We need Spanish-speaking volunteers!";
    let text2 = "We need Spanish-speaking volunteers!";

    assert_eq!(generate_content_hash(text1), generate_content_hash(text2));
}

#[test]
fn case_insensitive_hash() {
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
fn punctuation_ignored() {
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
fn whitespace_normalized() {
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
fn different_content_different_hash() {
    let text1 = "We need Spanish-speaking volunteers";
    let text2 = "We need French-speaking volunteers";

    assert_ne!(generate_content_hash(text1), generate_content_hash(text2));
}

#[test]
fn hash_format_is_valid() {
    let text = "Test content";
    let hash = generate_content_hash(text);

    // SHA256 hash should be 64 hex characters
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}
