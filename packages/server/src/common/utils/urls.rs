//! URL / domain normalisation helpers for organisation dedup (spec §7.1).

/// Normalise a website URL to a domain suitable for dedup lookup: lowercase,
/// strip scheme, strip leading `www.`, strip trailing slash and path. Returns
/// `None` for inputs that don't contain a recognisable host.
pub fn normalise_domain(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Strip scheme if present.
    let without_scheme = match trimmed.find("://") {
        Some(idx) => &trimmed[idx + 3..],
        None => trimmed,
    };

    // Drop path, query, fragment.
    let host_only = without_scheme
        .split(|c: char| c == '/' || c == '?' || c == '#')
        .next()
        .unwrap_or(without_scheme);

    // Drop userinfo (`user:pass@`).
    let host_only = match host_only.rfind('@') {
        Some(idx) => &host_only[idx + 1..],
        None => host_only,
    };

    // Drop port.
    let host_only = host_only.split(':').next().unwrap_or(host_only);

    let lower = host_only.to_lowercase();
    let stripped = lower.strip_prefix("www.").unwrap_or(&lower);
    if stripped.is_empty() || !stripped.contains('.') {
        None
    } else {
        Some(stripped.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_scheme_and_www() {
        assert_eq!(
            normalise_domain("https://www.Example.com/"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn strips_path_and_trailing_slash() {
        assert_eq!(
            normalise_domain("https://example.com/foo/bar/?q=1"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn handles_no_scheme() {
        assert_eq!(
            normalise_domain("example.com"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn drops_port() {
        assert_eq!(
            normalise_domain("http://example.com:8080/"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn rejects_invalid() {
        assert_eq!(normalise_domain(""), None);
        assert_eq!(normalise_domain("  "), None);
        assert_eq!(normalise_domain("/just-a-path"), None);
    }

    #[test]
    fn drops_userinfo() {
        assert_eq!(
            normalise_domain("https://user:pass@example.com/"),
            Some("example.com".to_string())
        );
    }
}
