# Security Status

## Known Vulnerabilities (as of 2026-01-29)

### Transitive Dependencies (No Fix Available)

1. **rsa 0.9.10** - Marvin Attack (RUSTSEC-2023-0071)
   - **Severity**: Medium (5.9/10)
   - **Source**: sqlx-mysql (transitive dependency)
   - **Impact**: Potential key recovery through timing sidechannels
   - **Mitigation**: We don't use MySQL, only PostgreSQL. This is a transitive dependency that can be ignored.
   - **Status**: Monitoring for updates

2. **tokio-tar 0.3.1** - PAX Header Parsing (RUSTSEC-2025-0111)
   - **Severity**: Critical
   - **Source**: testcontainers (dev/test dependency only)
   - **Impact**: File smuggling in PAX extended headers
   - **Mitigation**: Only used in development/test environments, not in production runtime
   - **Status**: Monitoring for updates

### Unmaintained Dependencies (Warning)

1. **rustls-pemfile** (versions 1.0.4 and 2.2.0)
   - **Status**: Unmaintained
   - **Source**: reqwest and testcontainers (transitive)
   - **Impact**: Low - functionality still works
   - **Action**: Monitor for maintained alternatives

## Fixed Vulnerabilities

### ✅ ring 0.16.20 - AES Panic (RUSTSEC-2025-0009)
- **Fixed**: Upgraded jsonwebtoken to v9, which uses ring 0.17+
- **Status**: Resolved

## Security Practices

- All API keys and secrets are in `.env` (gitignored)
- JWT tokens use HMAC-SHA256 with 24-hour expiration
- SQL injection prevented via sqlx parameterized queries
- Admin authorization enforced via fluent API in effect layer
- Privacy-first: no PII storage beyond hashed identifiers

## Reporting Security Issues

If you discover a security vulnerability, please email security@mndigitalaid.org with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

**Please do not open public GitHub issues for security vulnerabilities.**
