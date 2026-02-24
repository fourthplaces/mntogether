---
title: Replace Next.js with Dioxus Fullstack SSR
type: feat
date: 2026-02-04
---

# Replace Next.js with Dioxus Fullstack SSR

## Overview

Replace the Next.js frontend (`packages/web-next`) with a Dioxus fullstack SSR application integrated into the Rust server. The server will support two modes:
- `server --web` - Runs API + web frontend (single binary serving both)
- `server` - Headless API only (for cloud/scaling scenarios)

**Motivation:**
- Single language (Rust everywhere) - reduces context switching
- Deployment simplification - single binary instead of managing Node.js + Rust services
- Hybrid deployment support - web served locally, API can be remote

## Problem Statement

The current architecture requires:
1. Two separate services (Rust API + Node.js Next.js frontend)
2. Two different languages/ecosystems (Rust + TypeScript)
3. Complex Docker orchestration for deployment
4. Separate build pipelines and dependency management

This creates operational overhead and cognitive load when the entire application could be a single Rust binary.

## Proposed Solution

Integrate Dioxus fullstack SSR into the existing Rust server package, with conditional compilation to support both modes.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    server binary                         │
├─────────────────────────────────────────────────────────┤
│  Mode: --web                    │  Mode: headless        │
│  ┌─────────────────────────┐    │                        │
│  │   Dioxus SSR Router     │    │                        │
│  │   ┌─────────────────┐   │    │                        │
│  │   │ Public Pages    │   │    │                        │
│  │   │ Admin Pages     │   │    │                        │
│  │   │ Static Assets   │   │    │                        │
│  │   └─────────────────┘   │    │                        │
│  └─────────────────────────┘    │                        │
├─────────────────────────────────┼────────────────────────┤
│              Axum Router (shared)                        │
│  ┌─────────────────────────────────────────────────┐    │
│  │  /graphql - GraphQL API                         │    │
│  │  /health  - Health check                        │    │
│  │  Seesaw Engine + Domain Effects                 │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Technical Approach

### Phase 1: Foundation

#### 1.1 Add Dioxus Dependencies

**File: `packages/server/Cargo.toml`**

```toml
[dependencies]
# Existing deps...

# Dioxus fullstack (feature-gated)
dioxus = { version = "0.7", features = ["fullstack"], optional = true }

[features]
default = []
web = ["dep:dioxus"]
```

#### 1.2 Create Dioxus App Structure

```
packages/server/src/
├── web/                      # New Dioxus frontend module
│   ├── mod.rs               # Module exports + feature gate
│   ├── app.rs               # Root App component
│   ├── routes.rs            # Route definitions
│   ├── components/          # Shared UI components
│   │   ├── mod.rs
│   │   ├── post_card.rs
│   │   ├── pagination.rs
│   │   ├── loading.rs
│   │   └── layout.rs
│   ├── pages/               # Page components
│   │   ├── mod.rs
│   │   ├── public/
│   │   │   ├── home.rs
│   │   │   ├── search.rs
│   │   │   └── submit.rs
│   │   └── admin/
│   │       ├── login.rs
│   │       ├── dashboard.rs
│   │       ├── posts.rs
│   │       ├── websites.rs
│   │       ├── organizations.rs
│   │       ├── resources.rs
│   │       └── extraction.rs
│   ├── graphql/             # GraphQL client
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── queries.rs
│   │   └── mutations.rs
│   ├── auth/                # Authentication
│   │   ├── mod.rs
│   │   ├── context.rs
│   │   └── guards.rs
│   └── state/               # Client state management
│       ├── mod.rs
│       └── signals.rs
├── server/
│   ├── app.rs               # Modified to conditionally include Dioxus
│   └── ...
└── main.rs                  # Modified for --web flag
```

#### 1.3 Modify Main Entry Point

**File: `packages/server/src/main.rs`**

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "server")]
struct Args {
    /// Run with web frontend (SSR)
    #[arg(long)]
    web: bool,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Build base app (GraphQL, health, etc.)
    let (router, engine, deps) = build_app(...).await;

    // Conditionally add Dioxus routes
    #[cfg(feature = "web")]
    let router = if args.web {
        crate::web::attach_dioxus_routes(router)
    } else {
        router
    };

    // Start server...
}
```

### Phase 2: Core Implementation

#### 2.1 GraphQL Client for Dioxus

Use `cynic` for type-safe GraphQL queries (recommended over `graphql-client` for better Rust ergonomics).

**File: `packages/server/src/web/graphql/client.rs`**

```rust
use cynic::http::ReqwestExt;

pub struct GraphQLClient {
    client: reqwest::Client,
    endpoint: String,
}

impl GraphQLClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.into(),
        }
    }

    pub async fn query<Q>(&self, query: Q) -> Result<Q::ResponseData, GraphQLError>
    where
        Q: cynic::QueryBuilder + cynic::Operation,
    {
        let response = self.client
            .post(&self.endpoint)
            .run_graphql(query)
            .await?;

        if let Some(errors) = response.errors {
            return Err(GraphQLError::from(errors));
        }

        response.data.ok_or(GraphQLError::NoData)
    }
}
```

#### 2.2 Authentication Flow

**File: `packages/server/src/web/auth/context.rs`**

```rust
use dioxus::prelude::*;
use tower_sessions::Session;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AuthUser {
    pub member_id: uuid::Uuid,
    pub phone_number: String,
    pub is_admin: bool,
}

#[server]
pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    let session: Session = extract().await?;
    session.get("user").await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn send_verification_code(identifier: String) -> Result<bool, ServerFnError> {
    // Call existing GraphQL mutation via internal client
    let client = get_internal_graphql_client();
    let result = client.mutate(SendVerificationCode { identifier }).await?;
    Ok(result.success)
}

#[server]
pub async fn verify_code(identifier: String, code: String) -> Result<Option<String>, ServerFnError> {
    let client = get_internal_graphql_client();
    let result = client.mutate(VerifyCode { identifier, code }).await?;

    if let Some(token) = result.token {
        // Set session
        let session: Session = extract().await?;
        let user = decode_jwt_claims(&token)?;
        session.insert("user", user).await?;
        Ok(Some(token))
    } else {
        Ok(None)
    }
}
```

#### 2.3 Route Definitions

**File: `packages/server/src/web/routes.rs`**

```rust
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Routable)]
pub enum Route {
    // Public routes
    #[route("/")]
    Home {},

    #[route("/search")]
    Search {},

    #[route("/submit")]
    Submit {},

    // Admin routes (protected)
    #[nest("/admin")]
        #[route("/login")]
        AdminLogin {},

        #[layout(AdminLayout)]
            #[route("/dashboard")]
            AdminDashboard {},

            #[route("/posts")]
            AdminPosts {},

            #[route("/posts/:id")]
            AdminPostDetail { id: String },

            #[route("/websites")]
            AdminWebsites {},

            #[route("/websites/:id")]
            AdminWebsiteDetail { id: String },

            #[route("/organizations")]
            AdminOrganizations {},

            #[route("/organizations/:id")]
            AdminOrganizationDetail { id: String },

            #[route("/resources")]
            AdminResources {},

            #[route("/resources/:id")]
            AdminResourceDetail { id: String },

            #[route("/extraction")]
            AdminExtraction {},
        #[end_layout]
    #[end_nest]
}
```

#### 2.4 Admin Route Protection

**File: `packages/server/src/web/auth/guards.rs`**

```rust
use dioxus::prelude::*;

#[component]
pub fn AdminLayout() -> Element {
    let auth = use_context::<AuthContext>();
    let user = auth.user.read();

    match &*user {
        Some(Ok(Some(user))) if user.is_admin => {
            rsx! {
                div { class: "min-h-screen bg-gray-100",
                    AdminNav {}
                    main { class: "p-6",
                        Outlet::<Route> {}
                    }
                    ChatPanel {}
                }
            }
        }
        Some(Ok(Some(_))) => {
            // Authenticated but not admin
            rsx! { Redirect { to: Route::Home {} } }
        }
        Some(Ok(None)) | Some(Err(_)) => {
            // Not authenticated
            rsx! { Redirect { to: Route::AdminLogin {} } }
        }
        None => {
            // Loading
            rsx! { LoadingSpinner {} }
        }
    }
}
```

### Phase 3: Page Implementation

#### 3.1 Home Page

**File: `packages/server/src/web/pages/public/home.rs`**

```rust
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let posts = use_server_future(|| get_published_posts())?;
    let mut filter = use_signal(|| PostFilter::All);
    let mut search = use_signal(String::new);

    let filtered_posts = use_memo(move || {
        posts.iter()
            .filter(|p| filter().matches(p))
            .filter(|p| search().is_empty() || p.matches_search(&search()))
            .collect::<Vec<_>>()
    });

    rsx! {
        div { class: "container mx-auto px-4 py-8",
            // Search bar
            input {
                class: "w-full p-3 border rounded-lg mb-6",
                placeholder: "Search posts...",
                value: "{search}",
                oninput: move |e| search.set(e.value()),
            }

            // Filter tabs
            div { class: "flex gap-2 mb-6",
                for variant in PostFilter::variants() {
                    button {
                        class: if filter() == variant { "btn-primary" } else { "btn-secondary" },
                        onclick: move |_| filter.set(variant),
                        "{variant.label()}"
                    }
                }
            }

            // Post grid
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                for post in filtered_posts() {
                    PostCard { post: post.clone() }
                }
            }
        }
    }
}

#[server]
async fn get_published_posts() -> Result<Vec<Post>, ServerFnError> {
    let client = get_graphql_client();
    let result = client.query(GetPublishedPosts::build(())).await?;
    Ok(result.published_posts)
}
```

#### 3.2 Admin Login Page

**File: `packages/server/src/web/pages/admin/login.rs`**

```rust
use dioxus::prelude::*;

#[component]
pub fn AdminLogin() -> Element {
    let mut identifier = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut step = use_signal(|| LoginStep::EnterIdentifier);
    let mut error = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);

    let send_code = move |_| async move {
        loading.set(true);
        error.set(None);

        match send_verification_code(identifier()).await {
            Ok(true) => step.set(LoginStep::EnterCode),
            Ok(false) => error.set(Some("Failed to send code".into())),
            Err(e) => error.set(Some(e.to_string())),
        }

        loading.set(false);
    };

    let verify = move |_| async move {
        loading.set(true);
        error.set(None);

        match verify_code(identifier(), code()).await {
            Ok(Some(_token)) => {
                // Redirect to dashboard
                navigator().push(Route::AdminDashboard {});
            }
            Ok(None) => error.set(Some("Invalid code".into())),
            Err(e) => error.set(Some(e.to_string())),
        }

        loading.set(false);
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-100",
            div { class: "bg-white p-8 rounded-lg shadow-md w-full max-w-md",
                h1 { class: "text-2xl font-bold mb-6", "Admin Login" }

                if let Some(err) = error() {
                    div { class: "bg-red-100 text-red-700 p-3 rounded mb-4", "{err}" }
                }

                match step() {
                    LoginStep::EnterIdentifier => rsx! {
                        form { onsubmit: send_code,
                            input {
                                class: "w-full p-3 border rounded mb-4",
                                placeholder: "Phone number or email",
                                value: "{identifier}",
                                oninput: move |e| identifier.set(e.value()),
                            }
                            button {
                                class: "w-full bg-blue-600 text-white p-3 rounded",
                                disabled: loading(),
                                if loading() { "Sending..." } else { "Send Code" }
                            }
                        }
                    },
                    LoginStep::EnterCode => rsx! {
                        form { onsubmit: verify,
                            p { class: "text-gray-600 mb-4",
                                "Enter the 6-digit code sent to {identifier}"
                            }
                            input {
                                class: "w-full p-3 border rounded mb-4 text-center text-2xl tracking-widest",
                                placeholder: "000000",
                                maxlength: "6",
                                value: "{code}",
                                oninput: move |e| code.set(e.value()),
                            }
                            button {
                                class: "w-full bg-blue-600 text-white p-3 rounded",
                                disabled: loading() || code().len() != 6,
                                if loading() { "Verifying..." } else { "Verify" }
                            }
                        }
                    },
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum LoginStep {
    EnterIdentifier,
    EnterCode,
}
```

### Phase 4: Integration

#### 4.1 Attach Dioxus to Axum Router

**File: `packages/server/src/web/mod.rs`**

```rust
#[cfg(feature = "web")]
use axum::Router;
use dioxus::prelude::*;

#[cfg(feature = "web")]
pub fn attach_dioxus_routes(router: Router) -> Router {
    use tower_sessions::{MemoryStore, SessionManagerLayer};

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax);

    // Create Dioxus SSR router
    let dioxus_router = dioxus::server::router(App);

    // Merge with existing Axum router
    // Dioxus handles: /, /search, /submit, /admin/*
    // Existing handles: /graphql, /health
    router
        .merge(dioxus_router)
        .layer(session_layer)
}
```

#### 4.2 Shared GraphQL Types

Generate Rust types from GraphQL schema for use in both server and Dioxus components.

**File: `packages/server/src/web/graphql/schema.rs`**

```rust
// Generated from GraphQL schema using cynic
#[cynic::schema("mntogether")]
mod schema {}

#[derive(cynic::QueryFragment, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Post {
    pub id: cynic::Id,
    pub title: String,
    pub tldr: Option<String>,
    pub description: Option<String>,
    pub post_type: PostType,
    pub status: PostStatus,
    pub location: Option<String>,
    pub organization: Option<Organization>,
}

#[derive(cynic::QueryFragment, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Organization {
    pub id: cynic::Id,
    pub name: String,
    pub website: Option<String>,
}

// ... more types
```

## Acceptance Criteria

### Functional Requirements

- [ ] `server --web` serves both GraphQL API and web UI on single port
- [ ] `server` (no flag) serves only GraphQL API (current behavior)
- [ ] All public pages render with SSR (home, search, submit)
- [ ] All admin pages work with authentication (dashboard, posts, websites, etc.)
- [ ] OTP authentication flow works (send code, verify, set session)
- [ ] Admin route protection redirects unauthenticated users
- [ ] Chat panel works with polling for messages
- [ ] Post approval/rejection workflow works
- [ ] Website management (view, approve, crawl) works
- [ ] Search page with semantic search works
- [ ] Submit resource page works

### Non-Functional Requirements

- [ ] SSR response time < 200ms for public pages
- [ ] Hydration completes without visible flicker
- [ ] Single binary size < 50MB (release build)
- [ ] Memory usage < 256MB under normal load
- [ ] Tailwind CSS styling parity with current design

### Quality Gates

- [ ] All existing E2E tests pass (or equivalent new tests)
- [ ] No console errors in browser
- [ ] Lighthouse performance score > 80
- [ ] WCAG 2.1 AA accessibility compliance

## Dependencies & Prerequisites

### Before Starting

1. **Dioxus 0.7 stable release** - Currently latest, verify stability
2. **cynic crate** - For GraphQL client code generation
3. **tower-sessions** - For session management
4. **Tailwind CSS setup** - Dioxus-compatible build pipeline

### Technical Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| dioxus | 0.7 | Fullstack SSR framework |
| cynic | 3.x | GraphQL client |
| tower-sessions | 0.13 | Session management |
| clap | 4.x | CLI argument parsing (already in use) |

### External Dependencies

- Existing GraphQL schema remains unchanged
- Twilio Verify integration unchanged
- PostgreSQL database unchanged

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Dioxus SSR hydration issues | Medium | High | Build POC first, test hydration edge cases |
| GraphQL client type mismatch | Low | Medium | Use code generation from schema |
| Session/cookie handling differences | Medium | High | Test auth flow thoroughly before full migration |
| Tailwind build pipeline issues | Low | Medium | Use proven Dioxus + Tailwind examples |
| Performance regression | Medium | Medium | Benchmark SSR latency, set performance budget |
| Big bang migration failure | Low | High | Keep Next.js deployable until Dioxus verified |

## Implementation Notes

### GraphQL Client Strategy

Use **cynic** with code generation:
1. Export GraphQL schema from server: `cargo run -- export-schema > schema.graphql`
2. Generate Rust types: `cynic querygen`
3. Import into `web/graphql/schema.rs`

### Tailwind CSS Integration

```toml
# Dioxus.toml
[web.resource]
style = ["public/tailwind.css"]

[web.watcher]
watch_path = ["src", "public", "tailwind.config.js"]
```

Build pipeline:
```bash
npx tailwindcss -i ./input.css -o ./public/tailwind.css --watch
```

### Development Workflow

```bash
# Terminal 1: Tailwind watcher
npx tailwindcss -i ./input.css -o ./public/tailwind.css --watch

# Terminal 2: Dioxus dev server
dx serve --features web --hot-reload
```

### Hybrid Deployment (Local Web, Remote API)

For scenarios where web runs locally but API is remote:

```rust
// In Dioxus components, API URL is configurable
fn get_graphql_url() -> String {
    std::env::var("API_URL")
        .unwrap_or_else(|_| "/graphql".to_string())
}
```

Environment configuration:
```bash
# Local web + remote API
API_URL=https://api.example.com/graphql server --web

# All local (default)
server --web
```

## ERD: No Database Changes

This feature does not modify the database schema. All data access goes through the existing GraphQL API.

## References

### Internal References

- Current server architecture: `packages/server/src/server/app.rs`
- GraphQL schema: `packages/server/src/server/graphql/`
- Authentication: `packages/server/src/domains/auth/`
- Current Next.js pages: `packages/web-next/app/`
- Current GraphQL queries: `packages/web-next/lib/graphql/queries.ts`

### External References

- [Dioxus 0.7 Fullstack Documentation](https://dioxuslabs.com/learn/0.7/essentials/fullstack/)
- [Dioxus SSR Guide](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr)
- [Cynic GraphQL Client](https://cynic-rs.dev/)
- [tower-sessions](https://github.com/maxcountryman/tower-sessions)

### Related Work

- Current branch: `refactor/seesaw-0.7.2-upgrade`
- Seesaw upgrade already in progress
