//! Static service configuration — ports, container names, layer groupings.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    Infra,
    Backend,
    Frontend,
}

impl Layer {
    pub const ALL: &[Layer] = &[Layer::Infra, Layer::Backend, Layer::Frontend];

    pub fn label(self) -> &'static str {
        match self {
            Layer::Infra => "Infrastructure",
            Layer::Backend => "Backend",
            Layer::Frontend => "Frontend",
        }
    }

    pub fn key_hint(self) -> char {
        match self {
            Layer::Infra => 'i',
            Layer::Backend => 'b',
            Layer::Frontend => 'f',
        }
    }

    pub fn has_rebuild(self) -> bool {
        !matches!(self, Layer::Infra)
    }

    pub fn compose_services(self) -> Vec<&'static str> {
        SERVICES
            .iter()
            .filter(|s| s.layer == self)
            .map(|s| s.compose_name)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceId {
    Postgres,
    Restate,
    Minio,
    Server,
    AdminApp,
    WebApp,
}

pub struct ServiceDef {
    pub id: ServiceId,
    pub layer: Layer,
    pub label: &'static str,
    pub container: &'static str,
    pub compose_name: &'static str,
    /// Port used for health/liveness checks (the actual listening port).
    pub port: u16,
    /// Port shown in the dashboard UI (may differ — e.g. Restate UI on 9070, not ingress 8180).
    pub display_port: u16,
    /// URL to open in the browser, if applicable.
    pub url: Option<&'static str>,
}

pub const SERVICES: &[ServiceDef] = &[
    // ── Infrastructure ──────────────────────────────────────
    ServiceDef {
        id: ServiceId::Postgres,
        layer: Layer::Infra,
        label: "PostgreSQL",
        container: "rooteditorial_postgres",
        compose_name: "postgres",
        port: 5432,
        display_port: 5432,
        url: None,
    },
    ServiceDef {
        id: ServiceId::Restate,
        layer: Layer::Infra,
        label: "Restate",
        container: "rooteditorial_restate",
        compose_name: "restate",
        port: 8180,
        display_port: 9070,
        url: Some("http://localhost:9070"),
    },
    ServiceDef {
        id: ServiceId::Minio,
        layer: Layer::Infra,
        label: "MinIO (S3)",
        container: "rooteditorial_minio",
        compose_name: "minio",
        port: 9000,
        display_port: 9001,
        url: Some("http://localhost:9001"),
    },
    // ── Backend ─────────────────────────────────────────────
    ServiceDef {
        id: ServiceId::Server,
        layer: Layer::Backend,
        label: "Rust Server",
        container: "rooteditorial_server",
        compose_name: "server",
        port: 9080,
        display_port: 9080,
        url: None,
    },
    // ── Frontend ────────────────────────────────────────────
    ServiceDef {
        id: ServiceId::AdminApp,
        layer: Layer::Frontend,
        label: "Admin App (CMS)",
        container: "rooteditorial_admin_app",
        compose_name: "admin-app",
        port: 3000,
        display_port: 3000,
        url: Some("http://localhost:3000"),
    },
    ServiceDef {
        id: ServiceId::WebApp,
        layer: Layer::Frontend,
        label: "Web App",
        container: "rooteditorial_web_app",
        compose_name: "web-app",
        port: 3001,
        display_port: 3001,
        url: Some("http://localhost:3001"),
    },
];

/// Services with browser-openable URLs, sorted by display port.
/// Used to assign number keys (1, 2, 3...) in the dashboard.
pub fn url_services() -> Vec<&'static ServiceDef> {
    let mut svcs: Vec<_> = SERVICES.iter().filter(|s| s.url.is_some()).collect();
    svcs.sort_by_key(|s| s.display_port);
    svcs
}

/// All compose service names for a given menu target.
pub fn compose_names_for_target(target: &super::app::MenuTarget) -> Vec<&'static str> {
    use super::app::MenuTarget;
    match target {
        MenuTarget::Layer(layer) => layer.compose_services(),
        MenuTarget::All => SERVICES.iter().map(|s| s.compose_name).collect(),
    }
}
