//! SSRF guards for the Root Signal media ingest pipeline.
//!
//! Root Signal hands us arbitrary URLs via `source_image_url`. If we
//! resolved and fetched whatever host they named, a compromised or
//! coerced upstream could make the Editorial server reach into the
//! deployment's own private network (metadata services, MinIO admin,
//! Postgres, etc.). The guard below enforces:
//!
//!   * HTTPS only (rejects `http`, `file`, `gopher`, `ftp`, …).
//!   * Reject URLs whose host resolves to any loopback, private,
//!     link-local, multicast, documentation, unspecified, or CG-NAT
//!     address — v4 and v6.
//!   * Reject hostnames that are textual loopback aliases
//!     (`localhost`, `localhost.localdomain`, `ip6-localhost`, …).
//!
//! Validation happens **twice**: once on the submitted URL's hostname
//! (cheap fast-path for the common cases) and once on every IP address
//! returned by DNS resolution (defence against DNS rebinding / public
//! hostnames that resolve to private ranges). The DNS check runs inside
//! [`validate_resolved_ips`] and is called by [`fetch`] after resolving
//! the host.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

/// Hard-fail reasons surfaced to the ingest handler. The variant name
/// is what ends up in the structured error code Root Signal sees per
/// handoff §9.3.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SsrfError {
    #[error("URL is not valid: {0}")]
    InvalidUrl(String),
    #[error("scheme must be https (got {0:?})")]
    UnsupportedScheme(String),
    #[error("URL has no host")]
    MissingHost,
    #[error("host {0:?} is a disallowed loopback alias")]
    LoopbackHostname(String),
    #[error("resolved IP {0} is in a disallowed range")]
    DisallowedAddress(IpAddr),
}

/// Parse + scheme-check + hostname-check. Does not resolve DNS (callers
/// do that inside [`validate_resolved_ips`] after they have the DNS
/// answer from reqwest / a resolver).
pub fn validate_url(raw: &str) -> Result<Url, SsrfError> {
    let url = Url::parse(raw).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?;

    if url.scheme() != "https" {
        return Err(SsrfError::UnsupportedScheme(url.scheme().to_string()));
    }

    // url::Url::host() returns a typed enum that pre-classifies
    // literal IPs. host_str() would give us "[::1]" (brackets
    // included) for IPv6 literals, which is awkward to re-parse.
    match url.host().ok_or(SsrfError::MissingHost)? {
        Host::Domain(name) => {
            if is_loopback_hostname(name) {
                return Err(SsrfError::LoopbackHostname(name.to_string()));
            }
        }
        Host::Ipv4(v4) => {
            let ip = IpAddr::V4(v4);
            if is_disallowed_ip(&ip) {
                return Err(SsrfError::DisallowedAddress(ip));
            }
        }
        Host::Ipv6(v6) => {
            let ip = IpAddr::V6(v6);
            if is_disallowed_ip(&ip) {
                return Err(SsrfError::DisallowedAddress(ip));
            }
        }
    }

    Ok(url)
}

/// Check every DNS-resolved address for the target host. Any one
/// disallowed address fails the whole request — a multi-A-record host
/// that mixes public and private answers is still a rebinding vector.
pub fn validate_resolved_ips(ips: &[IpAddr]) -> Result<(), SsrfError> {
    for ip in ips {
        if is_disallowed_ip(ip) {
            return Err(SsrfError::DisallowedAddress(*ip));
        }
    }
    Ok(())
}

fn is_loopback_hostname(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "localhost.localdomain" | "ip6-localhost" | "ip6-loopback"
    )
}

/// True for every v4/v6 address space that is *not* reachable on the
/// public internet (or is otherwise dangerous for server-side fetch).
///
/// The nightly `IpAddr` helpers (`is_shared`, `is_benchmarking`,
/// `is_documentation`) aren't stable yet, so the CG-NAT and doc ranges
/// are spelled out inline. Keep the list in sync with
/// [IANA IPv4 Special-Purpose Registry][iana-v4] and
/// [IANA IPv6 Special-Purpose Registry][iana-v6].
///
/// [iana-v4]: https://www.iana.org/assignments/iana-ipv4-special-registry/
/// [iana-v6]: https://www.iana.org/assignments/iana-ipv6-special-registry/
fn is_disallowed_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_disallowed_v4(v4),
        IpAddr::V6(v6) => is_disallowed_v6(v6),
    }
}

fn is_disallowed_v4(ip: &Ipv4Addr) -> bool {
    if ip.is_loopback() || ip.is_private() || ip.is_link_local() {
        return true;
    }
    if ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast() {
        return true;
    }
    let o = ip.octets();
    // Documentation ranges (RFC 5737) — 192.0.2.0/24, 198.51.100.0/24,
    // 203.0.113.0/24. `Ipv4Addr::is_documentation` was unstable until
    // Rust 1.82; inline the check to keep this module portable.
    if (o[0] == 192 && o[1] == 0 && o[2] == 2)
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
    {
        return true;
    }
    // 100.64.0.0/10 — Carrier-grade NAT (RFC 6598).
    if o[0] == 100 && (o[1] & 0xC0) == 64 {
        return true;
    }
    // 169.254.0.0/16 is link-local (handled above); also the AWS /
    // GCP / Azure IMDS lives at 169.254.169.254 — belt and braces.
    if o[0] == 169 && o[1] == 254 {
        return true;
    }
    // 192.0.0.0/24 — IETF Protocol Assignments.
    if o[0] == 192 && o[1] == 0 && o[2] == 0 {
        return true;
    }
    // 198.18.0.0/15 — Network Interconnect Device Benchmark Testing.
    if o[0] == 198 && (o[1] & 0xFE) == 18 {
        return true;
    }
    false
}

fn is_disallowed_v6(ip: &Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
        return true;
    }
    let segs = ip.segments();
    // fe80::/10 — link-local.
    if (segs[0] & 0xFFC0) == 0xFE80 {
        return true;
    }
    // fc00::/7 — unique-local.
    if (segs[0] & 0xFE00) == 0xFC00 {
        return true;
    }
    // ::ffff:0:0/96 — IPv4-mapped; re-check the embedded v4.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_disallowed_v4(&v4);
    }
    // 64:ff9b::/96 — NAT64 well-known prefix. Conservatively reject;
    // the embedded v4 may or may not be public and we don't need it.
    if segs[0] == 0x0064 && segs[1] == 0xFF9B && segs[2] == 0 && segs[3] == 0 {
        return true;
    }
    // 2001:db8::/32 — documentation.
    if segs[0] == 0x2001 && segs[1] == 0x0DB8 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v4(s: &str) -> IpAddr {
        IpAddr::V4(s.parse().unwrap())
    }
    fn v6(s: &str) -> IpAddr {
        IpAddr::V6(s.parse().unwrap())
    }

    #[test]
    fn accepts_https_public_url() {
        let url = validate_url("https://example.org/image.jpg").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.org"));
    }

    #[test]
    fn rejects_http_scheme() {
        assert!(matches!(
            validate_url("http://example.org/image.jpg"),
            Err(SsrfError::UnsupportedScheme(s)) if s == "http"
        ));
    }

    #[test]
    fn rejects_file_scheme() {
        assert!(matches!(
            validate_url("file:///etc/passwd"),
            // url::Url parses file:/// with an empty host string — the
            // scheme check fires before the host check, so the error
            // variant is UnsupportedScheme regardless of host presence.
            Err(SsrfError::UnsupportedScheme(_))
        ));
    }

    #[test]
    fn rejects_gopher_and_ftp() {
        assert!(matches!(
            validate_url("gopher://example.org/"),
            Err(SsrfError::UnsupportedScheme(_))
        ));
        assert!(matches!(
            validate_url("ftp://example.org/"),
            Err(SsrfError::UnsupportedScheme(_))
        ));
    }

    #[test]
    fn rejects_literal_localhost_by_host() {
        assert!(matches!(
            validate_url("https://localhost/image.jpg"),
            Err(SsrfError::LoopbackHostname(h)) if h == "localhost"
        ));
        assert!(matches!(
            validate_url("https://LOCALHOST/image.jpg"),
            Err(SsrfError::LoopbackHostname(_))
        ));
    }

    #[test]
    fn rejects_literal_loopback_v4() {
        assert!(matches!(
            validate_url("https://127.0.0.1/image.jpg"),
            Err(SsrfError::DisallowedAddress(IpAddr::V4(_)))
        ));
        assert!(matches!(
            validate_url("https://127.255.255.254/image.jpg"),
            Err(SsrfError::DisallowedAddress(IpAddr::V4(_)))
        ));
    }

    #[test]
    fn rejects_literal_loopback_v6() {
        assert!(matches!(
            validate_url("https://[::1]/image.jpg"),
            Err(SsrfError::DisallowedAddress(IpAddr::V6(_)))
        ));
    }

    #[test]
    fn rejects_literal_private_v4() {
        for raw in [
            "https://10.0.0.1/image.jpg",
            "https://10.255.255.254/image.jpg",
            "https://192.168.1.1/image.jpg",
            "https://172.16.0.1/image.jpg",
            "https://172.31.255.254/image.jpg",
        ] {
            assert!(matches!(
                validate_url(raw),
                Err(SsrfError::DisallowedAddress(_))
            ), "expected {} to be rejected", raw);
        }
    }

    #[test]
    fn rejects_literal_link_local_v4() {
        assert!(matches!(
            validate_url("https://169.254.169.254/latest/meta-data"),
            Err(SsrfError::DisallowedAddress(_))
        ));
    }

    #[test]
    fn rejects_literal_link_local_v6() {
        assert!(matches!(
            validate_url("https://[fe80::1]/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
    }

    #[test]
    fn rejects_literal_unique_local_v6() {
        assert!(matches!(
            validate_url("https://[fc00::1]/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
        assert!(matches!(
            validate_url("https://[fd00::1]/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
    }

    #[test]
    fn rejects_cgnat_range() {
        assert!(matches!(
            validate_url("https://100.64.0.1/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
        assert!(matches!(
            validate_url("https://100.127.255.254/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
    }

    #[test]
    fn rejects_v4_multicast_and_broadcast() {
        assert!(matches!(
            validate_url("https://224.0.0.1/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
        assert!(matches!(
            validate_url("https://255.255.255.255/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
    }

    #[test]
    fn rejects_v4_unspecified() {
        assert!(matches!(
            validate_url("https://0.0.0.0/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
    }

    #[test]
    fn rejects_v6_mapped_private() {
        assert!(matches!(
            validate_url("https://[::ffff:10.0.0.1]/"),
            Err(SsrfError::DisallowedAddress(_))
        ));
    }

    #[test]
    fn validate_resolved_ips_rejects_mixed_answers() {
        let ips = vec![v4("8.8.8.8"), v4("10.0.0.1")];
        assert!(matches!(
            validate_resolved_ips(&ips),
            Err(SsrfError::DisallowedAddress(_))
        ));
    }

    #[test]
    fn validate_resolved_ips_accepts_all_public() {
        let ips = vec![v4("8.8.8.8"), v6("2001:4860:4860::8888")];
        validate_resolved_ips(&ips).unwrap();
    }
}
