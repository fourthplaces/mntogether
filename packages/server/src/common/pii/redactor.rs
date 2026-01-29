use super::detector::{PiiFindings, PiiMatch, PiiType};

/// Strategy for redacting PII
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionStrategy {
    /// Completely remove PII, replace with [REDACTED] token
    FullRemoval,
    /// Partially mask PII while preserving some readability
    /// Example: john@example.com -> j***@example.com
    PartialMask,
    /// Replace with typed tokens
    /// Example: john@example.com -> [EMAIL], (555) 123-4567 -> [PHONE]
    TokenReplacement,
}

/// Redact PII from text based on findings and strategy
pub fn redact_pii(text: &str, findings: &PiiFindings, strategy: RedactionStrategy) -> String {
    if findings.is_empty() {
        return text.to_string();
    }

    // Sort matches by position (reverse order so we can replace without offset issues)
    let mut sorted_matches: Vec<&PiiMatch> = findings.matches.iter().collect();
    sorted_matches.sort_by(|a, b| b.start.cmp(&a.start));

    let mut result = text.to_string();

    for pii_match in sorted_matches {
        let replacement = match strategy {
            RedactionStrategy::FullRemoval => "[REDACTED]".to_string(),
            RedactionStrategy::PartialMask => mask_value(&pii_match.value, &pii_match.pii_type),
            RedactionStrategy::TokenReplacement => {
                format!("[{}]", type_to_token(&pii_match.pii_type))
            }
        };

        result.replace_range(pii_match.start..pii_match.end, &replacement);
    }

    result
}

/// Convert PII type to token string
fn type_to_token(pii_type: &PiiType) -> &'static str {
    match pii_type {
        PiiType::Email => "EMAIL",
        PiiType::Phone => "PHONE",
        PiiType::Ssn => "SSN",
        PiiType::CreditCard => "CREDIT_CARD",
        PiiType::IpAddress => "IP_ADDRESS",
    }
}

/// Partially mask a value while preserving some readability
fn mask_value(value: &str, pii_type: &PiiType) -> String {
    match pii_type {
        PiiType::Email => mask_email(value),
        PiiType::Phone => mask_phone(value),
        PiiType::Ssn => mask_ssn(value),
        PiiType::CreditCard => mask_credit_card(value),
        PiiType::IpAddress => mask_ip(value),
    }
}

/// Mask email: john.doe@example.com -> j***@example.com
fn mask_email(email: &str) -> String {
    if let Some(at_pos) = email.find('@') {
        let (local, domain) = email.split_at(at_pos);
        if !local.is_empty() {
            let first_char = local.chars().next().unwrap();
            format!("{}***{}", first_char, domain)
        } else {
            format!("***{}", domain)
        }
    } else {
        "***@***.***".to_string()
    }
}

/// Mask phone: (555) 123-4567 -> (555) 123-****
fn mask_phone(phone: &str) -> String {
    // Find the last group of digits (typically last 4 digits)
    let chars: Vec<char> = phone.chars().collect();
    let mut last_digit_group_start = None;
    let mut in_digit_group = false;

    for (i, ch) in chars.iter().enumerate().rev() {
        if ch.is_ascii_digit() {
            if !in_digit_group {
                in_digit_group = true;
                last_digit_group_start = Some(i);
            }
        } else if in_digit_group {
            // Found the start of the last digit group
            if let Some(start) = last_digit_group_start {
                if start - i >= 3 {
                    // At least 4 digits in this group
                    let mut result = phone.to_string();
                    result.replace_range((i + 1)..=start, &"*".repeat(start - i));
                    return result;
                }
            }
            break;
        }
    }

    // Fallback: mask last 4 characters
    if phone.len() > 4 {
        let visible = &phone[..phone.len() - 4];
        format!("{}****", visible)
    } else {
        "***-****".to_string()
    }
}

/// Mask SSN: 123-45-6789 -> ***-**-6789
fn mask_ssn(ssn: &str) -> String {
    if ssn.len() >= 11 && ssn.contains('-') {
        // XXX-XX-XXXX format
        let parts: Vec<&str> = ssn.split('-').collect();
        if parts.len() == 3 {
            return format!("***-**-{}", parts[2]);
        }
    }

    // Fallback
    "***-**-****".to_string()
}

/// Mask credit card: 4532-1488-0343-6467 -> ****-****-****-6467
fn mask_credit_card(card: &str) -> String {
    let digits: String = card.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() >= 4 {
        let last_four = &digits[digits.len() - 4..];

        // Preserve original formatting if possible
        if card.contains('-') {
            "****-****-****-".to_string() + last_four
        } else if card.contains(' ') {
            "**** **** **** ".to_string() + last_four
        } else {
            "************".to_string() + last_four
        }
    } else {
        "****-****-****-****".to_string()
    }
}

/// Mask IP address: 192.168.1.100 -> 192.168.*.*
fn mask_ip(ip: &str) -> String {
    if ip.contains(':') {
        // IPv6 - mask last 4 groups
        let parts: Vec<&str> = ip.split(':').collect();
        if parts.len() >= 4 {
            let visible = &parts[..parts.len() - 4];
            return format!("{}:*:*:*:*", visible.join(":"));
        }
        "*:*:*:*:*:*:*:*".to_string()
    } else {
        // IPv4 - mask last 2 octets
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() == 4 {
            format!("{}.{}.*.*", parts[0], parts[1])
        } else {
            "*.*.*.*".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pii::detector::{detect_structured_pii, PiiType};

    #[test]
    fn test_full_removal_strategy() {
        let text = "Contact john@example.com or (555) 123-4567";
        let findings = detect_structured_pii(text);

        let result = redact_pii(text, &findings, RedactionStrategy::FullRemoval);

        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("john@example.com"));
        assert!(!result.contains("(555) 123-4567"));
    }

    #[test]
    fn test_partial_mask_strategy() {
        let text = "Email: john@example.com";
        let findings = detect_structured_pii(text);

        let result = redact_pii(text, &findings, RedactionStrategy::PartialMask);

        assert!(result.contains("j***@example.com"));
        assert!(!result.contains("john@example.com"));
    }

    #[test]
    fn test_token_replacement_strategy() {
        let text = "Contact john@example.com or (555) 123-4567";
        let findings = detect_structured_pii(text);

        let result = redact_pii(text, &findings, RedactionStrategy::TokenReplacement);

        assert!(result.contains("[EMAIL]"));
        assert!(result.contains("[PHONE]"));
        assert!(!result.contains("john@example.com"));
    }

    #[test]
    fn test_mask_email() {
        assert_eq!(mask_email("john@example.com"), "j***@example.com");
        assert_eq!(mask_email("a@test.org"), "a***@test.org");
        assert_eq!(mask_email("long.email.address@domain.com"), "l***@domain.com");
    }

    #[test]
    fn test_mask_phone() {
        assert!(mask_phone("(555) 123-4567").ends_with("****"));
        assert!(mask_phone("555-123-4567").ends_with("****"));
        assert!(mask_phone("+1-555-123-4567").ends_with("****"));
    }

    #[test]
    fn test_mask_ssn() {
        assert_eq!(mask_ssn("123-45-6789"), "***-**-6789");
    }

    #[test]
    fn test_mask_credit_card() {
        let masked = mask_credit_card("4532-1488-0343-6467");
        assert!(masked.ends_with("6467"));
        assert!(masked.contains("****"));
    }

    #[test]
    fn test_mask_ip() {
        assert_eq!(mask_ip("192.168.1.100"), "192.168.*.*");
        assert_eq!(mask_ip("10.0.0.5"), "10.0.*.*");
    }

    #[test]
    fn test_multiple_pii_types() {
        let text = "Email john@example.com, phone 555-123-4567, IP 192.168.1.1";
        let findings = detect_structured_pii(text);

        let result = redact_pii(text, &findings, RedactionStrategy::PartialMask);

        assert!(result.contains("j***@example.com"));
        assert!(result.contains("192.168.*.*"));
        assert!(!result.contains("john@example.com"));
    }

    #[test]
    fn test_preserve_formatting() {
        let text = "Contact Info:\n  Email: admin@test.com\n  Phone: (555) 123-4567";
        let findings = detect_structured_pii(text);

        let result = redact_pii(text, &findings, RedactionStrategy::PartialMask);

        // Should preserve the structure
        assert!(result.contains("Contact Info:"));
        assert!(result.contains("\n  Email:"));
        assert!(result.contains("\n  Phone:"));
    }

    #[test]
    fn test_no_pii() {
        let text = "This is a normal message with no PII";
        let findings = detect_structured_pii(text);

        let result = redact_pii(text, &findings, RedactionStrategy::PartialMask);

        assert_eq!(result, text);
    }

    #[test]
    fn test_overlapping_protection() {
        // Ensure we handle overlapping matches correctly by processing in reverse order
        let text = "a@b.com and c@d.com";
        let findings = detect_structured_pii(text);

        let result = redact_pii(text, &findings, RedactionStrategy::TokenReplacement);

        assert_eq!(result.matches("[EMAIL]").count(), 2);
    }
}
