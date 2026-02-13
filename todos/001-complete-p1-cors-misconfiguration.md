---
status: pending
priority: p1
issue_id: "001"
tags: [code-review, security, cors, csrf]
dependencies: []
---

# CORS Misconfiguration - Wide Open to All Origins

## Problem Statement

The CORS (Cross-Origin Resource Sharing) configuration in the server allows requests from ANY origin, making the application vulnerable to Cross-Site Request Forgery (CSRF) attacks. This is a **CRITICAL** security vulnerability that must be fixed before production deployment.

## Findings

**Location**: `/packages/server/src/server/app.rs:141-145`

**Current Code**:
```rust
let cors = CorsLayer::new()
    .allow_origin(Any)  // ⚠️ CRITICAL: Allows ANY origin
    .allow_methods(Any)
    .allow_headers(Any);
```

**Security Impact**:
- CSRF attacks possible from malicious websites
- Credential theft via third-party sites
- Unauthorized API access from any domain
- Exposure of sensitive volunteer and organization data

**From Security Sentinel Agent**: "This allows malicious websites to make authenticated requests on behalf of users, leading to CSRF attacks."

## Proposed Solutions

### Option 1: Whitelist Specific Origins (Recommended)
**Pros**: Most secure, explicit control over allowed domains
**Cons**: Requires maintaining origin list
**Effort**: Small (15 minutes)
**Risk**: Low

```rust
use tower_http::cors::{CorsLayer, AllowOrigin};

let cors = CorsLayer::new()
    .allow_origin([
        "https://yourdomain.com".parse().unwrap(),
        "https://admin.yourdomain.com".parse().unwrap(),
        "exp://192.168.1.0/24".parse().unwrap(), // Expo development
    ])
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    .allow_credentials(true);
```

### Option 2: Environment-Based Configuration
**Pros**: Different settings for dev/prod
**Cons**: More complex configuration
**Effort**: Medium (30 minutes)
**Risk**: Low

```rust
let allowed_origins = if cfg!(debug_assertions) {
    vec![Any]  // Allow all in development
} else {
    vec![
        "https://production.com".parse().unwrap(),
    ]
};

let cors = CorsLayer::new()
    .allow_origin(allowed_origins)
    // ...
```

### Option 3: Dynamic Origin Validation
**Pros**: Flexible, can validate against database
**Cons**: More complex, performance overhead
**Effort**: Large (2 hours)
**Risk**: Medium

## Recommended Action

**Option 1** - Whitelist specific origins immediately. This provides the best security with minimal complexity.

## Technical Details

**Affected Files**:
- `/packages/server/src/server/app.rs` (Line 141-145)

**Components**:
- Tower HTTP middleware layer
- GraphQL endpoint
- Authentication endpoints

**Testing Required**:
- Verify legitimate origins can access API
- Confirm blocked origins receive CORS errors
- Test Expo mobile app connectivity
- Test admin dashboard connectivity

## Acceptance Criteria

- [ ] CORS configured with specific allowed origins
- [ ] Development and production origins properly separated
- [ ] Mobile app (Expo) can still connect
- [ ] Admin dashboard can still connect
- [ ] Unauthorized origins receive CORS errors
- [ ] Credentials (JWT tokens) only sent to allowed origins
- [ ] Documentation updated with CORS configuration

## Work Log

*Empty - work not started*

## Resources

- **PR/Issue**: N/A - Found in code review
- **Related Code**:
  - `/packages/server/src/server/auth/edges.rs` (authentication endpoints)
  - `/packages/server/src/server/graphql/schema.rs` (GraphQL endpoint)
- **Documentation**:
  - [Tower HTTP CORS docs](https://docs.rs/tower-http/latest/tower_http/cors/)
  - [OWASP CORS Guide](https://owasp.org/www-community/attacks/csrf)
- **Similar Patterns**: N/A - new security fix
