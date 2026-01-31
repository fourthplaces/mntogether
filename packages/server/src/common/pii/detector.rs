use lazy_static::lazy_static;
use regex::Regex;

/// Type of PII that was detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PiiType {
    Email,
    Phone,
    Ssn,
    CreditCard,
    IpAddress,
}

/// A detected piece of PII with its location
#[derive(Debug, Clone)]
pub struct PiiMatch {
    pub pii_type: PiiType,
    pub value: String,
    pub start: usize,
    pub end: usize,
}

/// Collection of detected PII organized by type
#[derive(Debug, Default, Clone)]
pub struct PiiFindings {
    pub matches: Vec<PiiMatch>,
}

impl PiiFindings {
    pub fn new() -> Self {
        Self {
            matches: Vec::new(),
        }
    }

    pub fn add(&mut self, pii_type: PiiType, value: String, start: usize, end: usize) {
        self.matches.push(PiiMatch {
            pii_type,
            value,
            start,
            end,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn count(&self) -> usize {
        self.matches.len()
    }

    pub fn by_type(&self, pii_type: &PiiType) -> Vec<&PiiMatch> {
        self.matches
            .iter()
            .filter(|m| &m.pii_type == pii_type)
            .collect()
    }
}

lazy_static! {
    // Email pattern - RFC 5322 simplified
    static ref EMAIL_REGEX: Regex = Regex::new(
        r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b"
    ).unwrap();

    // Phone patterns - US and international
    static ref PHONE_REGEX: Regex = Regex::new(
        r"(?:\+?1[-.\s]?)?\(?([0-9]{3})\)?[-.\s]?([0-9]{3})[-.\s]?([0-9]{4})"
    ).unwrap();

    // Social Security Number - XXX-XX-XXXX
    static ref SSN_REGEX: Regex = Regex::new(
        r"\b\d{3}-\d{2}-\d{4}\b"
    ).unwrap();

    // Credit card numbers - various formats (Visa, MC, Amex, Discover)
    static ref CREDIT_CARD_REGEX: Regex = Regex::new(
        r"\b(?:\d{4}[-\s]?){3}\d{4}\b|\b\d{4}[-\s]?\d{6}[-\s]?\d{5}\b"
    ).unwrap();

    // IPv4 addresses
    static ref IPV4_REGEX: Regex = Regex::new(
        r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b"
    ).unwrap();

    // IPv6 addresses (simplified)
    static ref IPV6_REGEX: Regex = Regex::new(
        r"\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\b"
    ).unwrap();
}

/// Context for PII detection - determines what should be considered PII
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionContext {
    /// Personal user input - scrub all PII aggressively
    PersonalMessage,
    /// Public organizational content - preserve public contact info
    PublicContent,
}

/// Detect structured PII in text using regex patterns
///
/// Note: This detects ALL occurrences of structured PII patterns.
/// For context-aware filtering (e.g., preserving public org info),
/// use `detect_pii_contextual` instead.
pub fn detect_structured_pii(text: &str) -> PiiFindings {
    detect_pii_contextual(text, DetectionContext::PersonalMessage)
}

/// Detect PII with context awareness
///
/// - `PersonalMessage`: Scrub all PII (emails, phones, SSNs, etc.)
/// - `PublicContent`: Preserve likely organizational contact info
pub fn detect_pii_contextual(text: &str, context: DetectionContext) -> PiiFindings {
    let mut findings = PiiFindings::new();

    // Detect emails
    for mat in EMAIL_REGEX.find_iter(text) {
        let email = mat.as_str();

        // For public content, skip organizational emails
        if context == DetectionContext::PublicContent && is_likely_organizational_email(email, text)
        {
            continue;
        }

        findings.add(PiiType::Email, email.to_string(), mat.start(), mat.end());
    }

    // Detect phone numbers
    for mat in PHONE_REGEX.find_iter(text) {
        let phone = mat.as_str();

        // For public content, skip organizational phone numbers
        if context == DetectionContext::PublicContent && is_likely_organizational_phone(phone, text)
        {
            continue;
        }

        findings.add(PiiType::Phone, phone.to_string(), mat.start(), mat.end());
    }

    // Detect SSNs
    for mat in SSN_REGEX.find_iter(text) {
        findings.add(
            PiiType::Ssn,
            mat.as_str().to_string(),
            mat.start(),
            mat.end(),
        );
    }

    // Detect credit cards (with Luhn validation)
    for mat in CREDIT_CARD_REGEX.find_iter(text) {
        let card_num = mat.as_str().replace(['-', ' '], "");
        if is_valid_luhn(&card_num) {
            findings.add(
                PiiType::CreditCard,
                mat.as_str().to_string(),
                mat.start(),
                mat.end(),
            );
        }
    }

    // Detect IP addresses
    for mat in IPV4_REGEX.find_iter(text) {
        let ip_str = mat.as_str();
        // Filter out obvious non-IPs like version numbers
        if !is_likely_version_number(ip_str) {
            findings.add(
                PiiType::IpAddress,
                ip_str.to_string(),
                mat.start(),
                mat.end(),
            );
        }
    }

    for mat in IPV6_REGEX.find_iter(text) {
        findings.add(
            PiiType::IpAddress,
            mat.as_str().to_string(),
            mat.start(),
            mat.end(),
        );
    }

    findings
}

/// Luhn algorithm for credit card validation
fn is_valid_luhn(card_number: &str) -> bool {
    let digits: Vec<u32> = card_number.chars().filter_map(|c| c.to_digit(10)).collect();

    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }

    let checksum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(idx, &digit)| {
            if idx % 2 == 1 {
                let doubled = digit * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                digit
            }
        })
        .sum();

    checksum % 10 == 0
}

/// Check if an IP-like string is likely a version number
fn is_likely_version_number(ip_str: &str) -> bool {
    let parts: Vec<&str> = ip_str.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    // Version numbers often have 0 in first or last position
    // or multiple zeros
    let has_leading_zero = parts[0] == "0";
    let has_trailing_zero = parts[3] == "0";
    let zero_count = parts.iter().filter(|&&p| p == "0").count();

    has_leading_zero || has_trailing_zero || zero_count >= 2
}

/// Check if an email is likely organizational (public) vs personal
fn is_likely_organizational_email(email: &str, context_text: &str) -> bool {
    let email_lower = email.to_lowercase();

    // Common organizational email patterns
    let org_patterns = [
        "info@",
        "contact@",
        "admin@",
        "support@",
        "hello@",
        "team@",
        "help@",
        "sales@",
        "office@",
        "press@",
        "media@",
        "service@",
        "volunteer@",
        "donate@",
    ];

    // Check if email starts with organizational prefix
    if org_patterns
        .iter()
        .any(|pattern| email_lower.starts_with(pattern))
    {
        return true;
    }

    // Look for organizational context keywords near the email
    let context_lower = context_text.to_lowercase();
    let org_keywords = [
        "organization",
        "nonprofit",
        "charity",
        "foundation",
        "contact us",
        "reach us",
        "email us",
        "for more information",
        "office",
        "headquarters",
        "main office",
    ];

    // Find the email position and check surrounding text (±100 chars)
    if let Some(email_pos) = context_text.find(email) {
        let start = email_pos.saturating_sub(100);
        let end = (email_pos + email.len() + 100).min(context_text.len());
        let surrounding = &context_lower[start..end];

        if org_keywords
            .iter()
            .any(|keyword| surrounding.contains(keyword))
        {
            return true;
        }
    }

    false
}

/// Check if a phone number is likely organizational (public) vs personal
fn is_likely_organizational_phone(phone: &str, context_text: &str) -> bool {
    let context_lower = context_text.to_lowercase();

    // Look for organizational context keywords near the phone
    let org_keywords = [
        "office",
        "main line",
        "contact",
        "reach us",
        "call us",
        "phone:",
        "tel:",
        "telephone",
        "hotline",
        "helpline",
        "headquarters",
        "main office",
    ];

    // Find the phone position and check surrounding text (±100 chars)
    if let Some(phone_pos) = context_text.find(phone) {
        let start = phone_pos.saturating_sub(100);
        let end = (phone_pos + phone.len() + 100).min(context_text.len());
        let surrounding = &context_lower[start..end];

        if org_keywords
            .iter()
            .any(|keyword| surrounding.contains(keyword))
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_emails() {
        let text = "Contact me at john.doe@example.com or jane@test.org";
        let findings = detect_structured_pii(text);

        let emails = findings.by_type(&PiiType::Email);
        assert_eq!(emails.len(), 2);
        assert_eq!(emails[0].value, "john.doe@example.com");
        assert_eq!(emails[1].value, "jane@test.org");
    }

    #[test]
    fn test_detect_phones() {
        let text = "Call me at (555) 123-4567 or 555-987-6543 or +1-555-111-2222";
        let findings = detect_structured_pii(text);

        let phones = findings.by_type(&PiiType::Phone);
        assert_eq!(phones.len(), 3);
        assert!(phones[0].value.contains("555"));
    }

    #[test]
    fn test_detect_ssn() {
        let text = "My SSN is 123-45-6789 for verification.";
        let findings = detect_structured_pii(text);

        let ssns = findings.by_type(&PiiType::Ssn);
        assert_eq!(ssns.len(), 1);
        assert_eq!(ssns[0].value, "123-45-6789");
    }

    #[test]
    fn test_detect_credit_cards() {
        // Valid Visa test number
        let text = "Card: 4532-1488-0343-6467";
        let findings = detect_structured_pii(text);

        let cards = findings.by_type(&PiiType::CreditCard);
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn test_detect_ip_addresses() {
        let text = "Server at 192.168.1.1 and 10.0.0.5";
        let findings = detect_structured_pii(text);

        let ips = findings.by_type(&PiiType::IpAddress);
        assert_eq!(ips.len(), 2);
    }

    #[test]
    fn test_no_false_positives() {
        let text = "Version 1.2.3.4 released. Contact support@company.com";
        let findings = detect_structured_pii(text);

        // Should detect email but not version number as IP
        let emails = findings.by_type(&PiiType::Email);
        let ips = findings.by_type(&PiiType::IpAddress);

        assert_eq!(emails.len(), 1);
        assert_eq!(ips.len(), 0); // Version number filtered out
    }

    #[test]
    fn test_luhn_validation() {
        // Valid Visa
        assert!(is_valid_luhn("4532148803436467"));
        // Invalid number
        assert!(!is_valid_luhn("1234567890123456"));
    }

    #[test]
    fn test_empty_text() {
        let findings = detect_structured_pii("");
        assert!(findings.is_empty());
        assert_eq!(findings.count(), 0);
    }

    #[test]
    fn test_mixed_pii() {
        let text = "Email john@example.com, phone 555-1234, IP 192.168.1.1";
        let findings = detect_structured_pii(text);

        assert_eq!(findings.by_type(&PiiType::Email).len(), 1);
        assert_eq!(findings.by_type(&PiiType::Phone).len(), 1);
        assert_eq!(findings.by_type(&PiiType::IpAddress).len(), 1);
    }
}
