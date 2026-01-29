//! Configuration system for the dev CLI.
//!
//! Supports a distributed configuration model:
//! - `.dev/config.toml` - Global configuration shared across all packages
//! - `packages/*/dev.toml` - Package-specific configuration (optional)
//!
//! Package names are derived from existing configs:
//! - Rust packages: Cargo.toml `[package] name = "..."`
//! - JS packages: package.json `"name": "..."`
//!
//! Release types are inferred from files:
//! - Cargo.toml present → "server"
//! - app.json present → "expo"
//!
//! Packages declare capabilities via TOML sections:
//! - `[database]` - Package has migrations/seeds
//! - `[mobile]` - Package is a mobile app
//! - `[ai]` - Package has AI tooling

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// =============================================================================
// Global Configuration (.dev/config.toml)
// =============================================================================

/// Global configuration shared across all packages
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct GlobalConfig {
    pub project: ProjectConfig,
    pub workspaces: WorkspacesConfig,
    pub git: GitConfig,
    pub esc: EscConfig,
    pub environments: EnvironmentsConfig,
    pub services: ServicesConfig,
    pub urls: UrlsConfig,
    pub act: ActConfig,
    pub defaults: DefaultsConfig,
    pub test: TestConfig,
    pub ai: GlobalAiConfig,
}

/// Global AI configuration
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct GlobalAiConfig {
    /// Glob pattern for AI task files
    #[serde(default = "default_ai_tasks")]
    pub tasks: String,
}

fn default_ai_tasks() -> String {
    "docs/ai/tasks/*.md".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// Project name (used for ECS service prefix, etc.)
    #[serde(default = "default_project_name")]
    pub name: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: default_project_name(),
        }
    }
}

fn default_project_name() -> String {
    "mntogether".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct WorkspacesConfig {
    /// Glob patterns for package discovery
    #[serde(default = "default_packages_patterns")]
    pub packages: Vec<String>,
    /// Glob patterns for infra package discovery
    #[serde(default = "default_infra_patterns")]
    pub infra: Vec<String>,
    /// Packages to exclude
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for WorkspacesConfig {
    fn default() -> Self {
        Self {
            packages: default_packages_patterns(),
            infra: default_infra_patterns(),
            exclude: Vec::new(),
        }
    }
}

fn default_packages_patterns() -> Vec<String> {
    vec!["packages/*".to_string()]
}

fn default_infra_patterns() -> Vec<String> {
    vec!["infra/packages/*".to_string()]
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GitConfig {
    /// Protected branches that require special handling
    #[serde(default = "default_protected_branches")]
    pub protected_branches: Vec<String>,
    /// Default base branch for PRs
    #[serde(default = "default_pr_base")]
    pub default_pr_base: String,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            protected_branches: default_protected_branches(),
            default_pr_base: default_pr_base(),
        }
    }
}

fn default_protected_branches() -> Vec<String> {
    vec!["main".to_string(), "master".to_string(), "dev".to_string()]
}

fn default_pr_base() -> String {
    "dev".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EscConfig {
    /// Pulumi ESC namespace for secrets
    #[serde(default = "default_esc_namespace")]
    pub namespace: String,
}

impl Default for EscConfig {
    fn default() -> Self {
        Self {
            namespace: default_esc_namespace(),
        }
    }
}

fn default_esc_namespace() -> String {
    "mntogether/service".to_string()
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct EnvironmentsConfig {
    /// Available environments
    #[serde(default = "default_environments")]
    pub available: Vec<String>,
    /// Default environment
    #[serde(default = "default_env")]
    pub default: String,
}

fn default_environments() -> Vec<String> {
    vec!["dev".to_string(), "prod".to_string()]
}

fn default_env() -> String {
    "dev".to_string()
}

/// Services configuration - maps service name to port
/// Example in TOML:
/// [services]
/// api = 8080
/// postgres = 5432
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ServicesConfig {
    /// Service ports keyed by service name
    #[serde(flatten)]
    pub ports: HashMap<String, u16>,
}

impl ServicesConfig {
    /// Get port for a service, with optional default
    pub fn get_port(&self, service: &str, default: u16) -> u16 {
        self.ports.get(service).copied().unwrap_or(default)
    }
}

/// Quick access URLs configuration
/// Example in TOML:
/// [urls.playground]
/// label = "GraphQL Playground"
/// url = "http://localhost:8080/playground"
///
/// [urls.signoz]
/// label = "SigNoz Dashboard"
/// url = "http://localhost:3301"
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct UrlsConfig {
    /// URL entries keyed by identifier
    #[serde(flatten)]
    pub entries: HashMap<String, UrlEntry>,
}

/// A quick access URL entry
#[derive(Debug, Deserialize, Clone)]
pub struct UrlEntry {
    /// Display label for the URL
    pub label: String,
    /// The URL to open
    pub url: String,
}

impl UrlsConfig {
    /// Get a URL entry by key
    pub fn get(&self, key: &str) -> Option<&UrlEntry> {
        self.entries.get(key)
    }

    /// Get all URL entries as (key, entry) pairs
    pub fn all(&self) -> impl Iterator<Item = (&String, &UrlEntry)> {
        self.entries.iter()
    }

    /// Check if any URLs are defined
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ActConfig {
    /// Container architecture for act
    #[serde(default = "default_container_architecture")]
    pub container_architecture: String,
    /// Docker image for act
    #[serde(default = "default_docker_image")]
    pub docker_image: String,
    /// Artifact directory for act
    #[serde(default = "default_artifact_dir")]
    pub artifact_dir: String,
}

fn default_container_architecture() -> String {
    "linux/amd64".to_string()
}

fn default_docker_image() -> String {
    "catthehacker/ubuntu:act-latest".to_string()
}

fn default_artifact_dir() -> String {
    ".act-artifacts".to_string()
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct DefaultsConfig {
    /// Default coverage format
    #[serde(default = "default_coverage_format")]
    pub coverage_format: String,
    /// Default number of releases to list
    #[serde(default = "default_release_list_count")]
    pub release_list_count: u32,
    /// Default number of load test requests
    #[serde(default = "default_load_test_requests")]
    pub load_test_requests: u32,
    /// Default concurrency for load tests
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,
    /// Default CI runs limit
    #[serde(default = "default_ci_runs_limit")]
    pub ci_runs_limit: u32,
}

fn default_coverage_format() -> String {
    "html".to_string()
}
fn default_release_list_count() -> u32 {
    5
}
fn default_load_test_requests() -> u32 {
    1000
}
fn default_concurrency() -> u32 {
    10
}
fn default_ci_runs_limit() -> u32 {
    10
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct TestConfig {
    /// Test runner command (e.g., "cargo test" or "yarn test")
    #[serde(default = "default_test_command")]
    pub command: String,
    /// Watch mode command (e.g., "cargo watch -x test" or "yarn test --watch")
    pub watch_command: Option<String>,
    /// Packages that require test coverage enforcement (empty = all packages)
    #[serde(default)]
    pub coverage_packages: Vec<String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            command: default_test_command(),
            watch_command: None,
            coverage_packages: Vec::new(),
        }
    }
}

fn default_test_command() -> String {
    "cargo test".to_string()
}

// =============================================================================
// Package Configuration (packages/*/dev.toml)
// =============================================================================

/// Package-specific configuration from dev.toml (optional overrides only)
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct PackageToml {
    /// Whether this package is releasable (default: true if release_type is inferred)
    pub releasable: Option<bool>,
    /// ECS service name for CloudWatch logs (required if deployable)
    pub ecs_service: Option<String>,
    /// Other packages included in this release
    #[serde(default)]
    pub includes: Vec<String>,

    // === Capabilities ===
    /// Database capability
    pub database: Option<DatabaseConfig>,
    /// Mobile capability
    pub mobile: Option<MobileConfig>,
    /// AI capability
    pub ai: Option<AiConfig>,

    // === Commands ===
    /// Package commands (shorthand: cmd.name = "string")
    #[serde(default)]
    pub cmd: HashMap<String, CmdEntry>,
}

/// AI capability configuration
#[derive(Debug, Deserialize, Clone)]
pub struct AiConfig {
    /// Path to AI tasks directory (relative to package)
    pub tasks_dir: String,
}

// =============================================================================
// Command Configuration
// =============================================================================

/// Command entry - either a simple string or full config
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum CmdEntry {
    /// Simple command string
    Simple(String),
    /// Full command config with options
    Full(CmdConfig),
}

impl CmdEntry {
    /// Get the default command
    pub fn default_cmd(&self) -> &str {
        match self {
            CmdEntry::Simple(s) => s,
            CmdEntry::Full(c) => &c.default,
        }
    }

    /// Get a variant command by name (e.g., "fix", "watch")
    /// Falls back to default command if variant not defined
    pub fn variant(&self, name: &str) -> &str {
        match self {
            CmdEntry::Simple(s) => s,
            CmdEntry::Full(c) => c
                .variants
                .get(name)
                .map(|s| s.as_str())
                .unwrap_or(&c.default),
        }
    }

    /// Get dependencies
    pub fn deps(&self) -> &[String] {
        match self {
            CmdEntry::Simple(_) => &[],
            CmdEntry::Full(c) => &c.deps,
        }
    }
}

/// Full command configuration
/// All fields except `default` and `deps` are treated as variants
#[derive(Debug, Clone)]
pub struct CmdConfig {
    /// The default command to run
    pub default: String,
    /// Dependencies to run first (format: "package:cmd" or "package" for same cmd)
    pub deps: Vec<String>,
    /// Command variants (any other key becomes a variant)
    pub variants: HashMap<String, String>,
}

impl<'de> Deserialize<'de> for CmdConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut map: HashMap<String, toml::Value> = HashMap::deserialize(deserializer)?;

        // Extract known fields
        let default = map
            .remove("default")
            .and_then(|v| v.as_str().map(String::from))
            .ok_or_else(|| serde::de::Error::missing_field("default"))?;

        let deps = map
            .remove("deps")
            .map(|v| {
                v.as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        // Everything else is a variant
        let variants: HashMap<String, String> = map
            .into_iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
            .collect();

        Ok(CmdConfig {
            default,
            deps,
            variants,
        })
    }
}

/// Resolved package configuration with inferred values
#[derive(Debug, Default)]
pub struct PackageConfig {
    /// Package directory path (auto-set during discovery)
    pub path: PathBuf,
    /// Package directory name (for reference)
    pub dir_name: String,
    /// Package name from Cargo.toml or package.json
    pub name: String,
    /// Release type inferred from files ("server" or "expo")
    pub release_type: Option<String>,
    /// Whether this package is releasable (explicit or inferred from release_type)
    pub releasable: bool,
    /// Other packages included in this release
    pub includes: Vec<String>,
    /// ECS service name for CloudWatch logs
    pub ecs_service: Option<String>,
    /// Database capability
    pub database: Option<DatabaseConfig>,
    /// Mobile capability
    pub mobile: Option<MobileConfig>,
    /// AI capability
    pub ai: Option<AiConfig>,
    /// Package commands
    pub cmd: HashMap<String, CmdEntry>,
}

impl PackageConfig {
    /// Check if this package is releasable (explicit setting or inferred from release_type)
    pub fn is_releasable(&self) -> bool {
        self.releasable
    }

    /// Check if this package is deployable (has ECS service)
    pub fn is_deployable(&self) -> bool {
        self.ecs_service.is_some()
    }

    /// Get the tag prefix for releases (e.g., "api@")
    pub fn tag_prefix(&self) -> String {
        format!("{}@", self.name)
    }

    /// Get full path to migrations directory
    pub fn migrations_path(&self) -> Option<PathBuf> {
        self.database
            .as_ref()
            .map(|db| self.path.join(&db.migrations))
    }

    /// Get full path to seeds file
    pub fn seeds_path(&self) -> Option<PathBuf> {
        self.database
            .as_ref()
            .and_then(|db| db.seeds.as_ref().map(|s| self.path.join(s)))
    }

    /// Get full path to AI tasks directory
    pub fn ai_tasks_path(&self) -> Option<PathBuf> {
        self.ai.as_ref().map(|ai| self.path.join(&ai.tasks_dir))
    }
}

// =============================================================================
// Package Name Inference
// =============================================================================

/// Infer package name from Cargo.toml
fn infer_name_from_cargo_toml(package_path: &Path) -> Option<String> {
    let cargo_path = package_path.join("Cargo.toml");
    if !cargo_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&cargo_path).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;

    parsed
        .get("package")?
        .get("name")?
        .as_str()
        .map(|s| s.to_string())
}

/// Infer package name from package.json
fn infer_name_from_package_json(package_path: &Path) -> Option<String> {
    let json_path = package_path.join("package.json");
    if !json_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&json_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    parsed.get("name")?.as_str().map(|s| {
        // Strip org prefix like "@org/app" -> "app"
        if let Some(stripped) = s.strip_prefix('@') {
            stripped.split('/').nth(1).unwrap_or(s).to_string()
        } else {
            s.to_string()
        }
    })
}

/// Infer package name from existing config files
fn infer_package_name(package_path: &Path, dir_name: &str) -> String {
    // Try Cargo.toml first
    if let Some(name) = infer_name_from_cargo_toml(package_path) {
        return name;
    }

    // Try package.json next
    if let Some(name) = infer_name_from_package_json(package_path) {
        return name;
    }

    // Fallback to directory name
    dir_name.to_string()
}

/// Infer release type from files present in the package
fn infer_release_type(package_path: &Path) -> Option<String> {
    // app.json, app.config.ts, or app.config.js → expo (mobile app)
    if package_path.join("app.json").exists()
        || package_path.join("app.config.ts").exists()
        || package_path.join("app.config.js").exists()
    {
        return Some("expo".to_string());
    }

    // Cargo.toml with [[bin]] or src/main.rs → server
    let cargo_path = package_path.join("Cargo.toml");
    if cargo_path.exists() {
        // Check for src/main.rs (binary crate)
        if package_path.join("src/main.rs").exists() {
            return Some("server".to_string());
        }

        // Check for [[bin]] section in Cargo.toml
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
                if parsed.get("bin").is_some() {
                    return Some("server".to_string());
                }
            }
        }
    }

    None
}

/// Database capability configuration
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// Path to migrations directory (relative to package)
    pub migrations: String,
    /// Path to seed file (relative to package)
    pub seeds: Option<String>,
}

/// Mobile capability configuration
#[derive(Debug, Deserialize, Clone)]
pub struct MobileConfig {
    /// EAS build profiles
    #[serde(default)]
    pub eas_profiles: Vec<String>,
    /// EAS platforms
    #[serde(default)]
    pub eas_platforms: Vec<String>,
    /// Pre-run scripts (relative to package)
    #[serde(default)]
    pub pre_run_scripts: Vec<String>,
    /// Startup timeout in seconds
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout_secs: u32,
}

fn default_startup_timeout() -> u32 {
    300
}

// =============================================================================
// Combined Configuration
// =============================================================================

/// Combined configuration from global and package configs
#[derive(Debug, Default)]
pub struct Config {
    /// Repository root path
    pub repo_root: PathBuf,
    /// Global configuration
    pub global: GlobalConfig,
    /// Package configurations keyed by package name
    pub packages: HashMap<String, PackageConfig>,
}

impl Config {
    /// Load configuration from the repository root
    pub fn load(repo_root: &Path) -> Result<Self> {
        let global = Self::load_global_config(repo_root)?;
        let packages = Self::discover_packages(repo_root, &global)?;

        Ok(Config {
            repo_root: repo_root.to_path_buf(),
            global,
            packages,
        })
    }

    /// Load global configuration from .dev/config.toml
    fn load_global_config(repo_root: &Path) -> Result<GlobalConfig> {
        let config_path = repo_root.join(".dev/config.toml");

        if !config_path.exists() {
            // Return defaults if no config file
            return Ok(GlobalConfig::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", config_path.display()))
    }

    /// Discover packages and load their configurations
    fn discover_packages(
        repo_root: &Path,
        global: &GlobalConfig,
    ) -> Result<HashMap<String, PackageConfig>> {
        let mut packages = HashMap::new();

        // Discover packages using glob patterns
        for pattern in &global.workspaces.packages {
            let full_pattern = repo_root.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            for entry in glob::glob(&pattern_str)? {
                let path = entry?;
                if !path.is_dir() {
                    continue;
                }

                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();

                // Skip excluded packages
                if global.workspaces.exclude.contains(&name)
                    || global
                        .workspaces
                        .exclude
                        .iter()
                        .any(|e| e.ends_with(&format!("/{}", name)))
                {
                    continue;
                }

                let config = Self::load_package_config(&path, &name)?;
                packages.insert(name, config);
            }
        }

        Ok(packages)
    }

    /// Load package configuration with inference from existing files
    fn load_package_config(package_path: &Path, dir_name: &str) -> Result<PackageConfig> {
        // Infer name from Cargo.toml or package.json
        let name = infer_package_name(package_path, dir_name);

        // Infer release type from files
        let release_type = infer_release_type(package_path);

        // Load dev.toml for explicit overrides (optional)
        let config_path = package_path.join("dev.toml");
        let toml_config: PackageToml = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse {}", config_path.display()))?
        } else {
            PackageToml::default()
        };

        // Determine releasable: explicit setting only (default false)
        let releasable = toml_config.releasable.unwrap_or(false);

        // Build resolved config with inferred + explicit values
        Ok(PackageConfig {
            path: package_path.to_path_buf(),
            dir_name: dir_name.to_string(),
            name,
            release_type,
            releasable,
            includes: toml_config.includes,
            ecs_service: toml_config.ecs_service,
            database: toml_config.database,
            mobile: toml_config.mobile,
            ai: toml_config.ai,
            cmd: toml_config.cmd,
        })
    }

    // =========================================================================
    // Capability Discovery
    // =========================================================================

    /// Find all packages with database capability
    pub fn database_packages(&self) -> Vec<(&str, &DatabaseConfig)> {
        self.packages
            .iter()
            .filter_map(|(name, pkg)| pkg.database.as_ref().map(|db| (name.as_str(), db)))
            .collect()
    }

    /// Find all packages with mobile capability
    pub fn mobile_packages(&self) -> Vec<(&str, &MobileConfig)> {
        self.packages
            .iter()
            .filter_map(|(name, pkg)| pkg.mobile.as_ref().map(|m| (name.as_str(), m)))
            .collect()
    }

    /// Find all packages with AI capability
    pub fn ai_packages(&self) -> Vec<(&str, &AiConfig)> {
        self.packages
            .iter()
            .filter_map(|(name, pkg)| pkg.ai.as_ref().map(|ai| (name.as_str(), ai)))
            .collect()
    }

    /// Find all packages that have a specific command
    pub fn packages_with_cmd(&self, cmd_name: &str) -> Vec<(&str, &PackageConfig, &CmdEntry)> {
        self.packages
            .iter()
            .filter_map(|(name, pkg)| pkg.cmd.get(cmd_name).map(|cmd| (name.as_str(), pkg, cmd)))
            .collect()
    }

    /// Get a specific command from a package
    pub fn get_cmd(&self, pkg_name: &str, cmd_name: &str) -> Option<&CmdEntry> {
        self.packages
            .get(pkg_name)
            .and_then(|pkg| pkg.cmd.get(cmd_name))
    }

    /// Get a specific package configuration
    pub fn get_package(&self, name: &str) -> Option<&PackageConfig> {
        self.packages.get(name)
    }

    /// Get release tag pattern for a package
    pub fn release_tag_pattern(&self, package_name: &str) -> String {
        // Use package name for pattern
        format!("{}-v*", package_name)
    }

    /// Find all releasable packages
    pub fn releasable_packages(&self) -> Vec<&PackageConfig> {
        self.packages
            .values()
            .filter(|pkg| pkg.is_releasable())
            .collect()
    }

    /// Find all deployable packages
    pub fn deployable_packages(&self) -> Vec<&PackageConfig> {
        self.packages
            .values()
            .filter(|pkg| pkg.is_deployable())
            .collect()
    }

    /// Get all package names (for commit scopes)
    pub fn package_names(&self) -> Vec<&str> {
        self.packages.values().map(|p| p.name.as_str()).collect()
    }

    /// Get ESC path for an environment
    pub fn esc_path(&self, env: &str) -> String {
        format!("{}/{}", self.global.esc.namespace, env)
    }

    // =========================================================================
    // Package Path Helpers
    // =========================================================================

    /// Get the path to a package
    pub fn package_path(&self, name: &str) -> PathBuf {
        self.repo_root.join("packages").join(name)
    }

    /// Get path to migrations for a database package
    pub fn migrations_path(&self, name: &str) -> Option<PathBuf> {
        self.packages.get(name).and_then(|pkg| {
            pkg.database
                .as_ref()
                .map(|db| self.package_path(name).join(&db.migrations))
        })
    }

    /// Get path to seeds for a database package
    pub fn seeds_path(&self, name: &str) -> Option<PathBuf> {
        self.packages.get(name).and_then(|pkg| {
            pkg.database
                .as_ref()
                .and_then(|db| db.seeds.as_ref().map(|s| self.package_path(name).join(s)))
        })
    }

    /// Get path to AI tasks directory for a package
    pub fn ai_tasks_path(&self, name: &str) -> Option<PathBuf> {
        self.packages.get(name).and_then(|pkg| {
            pkg.ai
                .as_ref()
                .map(|ai| self.package_path(name).join(&ai.tasks_dir))
        })
    }

    // =========================================================================
    // Test Coverage
    // =========================================================================

    /// Check if a file path requires test coverage enforcement based on config.
    /// If coverage_packages is empty, all files require coverage.
    /// Otherwise, only files in the specified packages require coverage.
    pub fn requires_coverage(&self, file_path: &str) -> bool {
        let coverage_packages = &self.global.test.coverage_packages;

        // If no packages specified, all files require coverage
        if coverage_packages.is_empty() {
            return true;
        }

        // Check if file is in one of the coverage packages using configured package paths
        for pkg_name in coverage_packages {
            if let Some(pkg) = self.packages.get(pkg_name) {
                // Get the package path relative to repo root
                let pkg_path = pkg
                    .path
                    .strip_prefix(&self.repo_root)
                    .unwrap_or(&pkg.path)
                    .to_string_lossy();
                let prefix = format!("{}/", pkg_path);
                if file_path.starts_with(&prefix) {
                    return true;
                }
            }
        }

        false
    }

    // =========================================================================
    // Infra Discovery
    // =========================================================================

    /// Discover infra stacks (directories with Pulumi.yaml)
    pub fn discover_infra_stacks(&self) -> Result<Vec<String>> {
        let mut stacks = Vec::new();

        for pattern in &self.global.workspaces.infra {
            let full_pattern = self.repo_root.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            for entry in glob::glob(&pattern_str)? {
                let path = entry?;
                if !path.is_dir() {
                    continue;
                }

                // Only include if Pulumi.yaml exists (valid Pulumi stack)
                if !path.join("Pulumi.yaml").exists() {
                    continue;
                }

                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    stacks.push(name.to_string());
                }
            }
        }

        Ok(stacks)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_global_config() {
        let config = GlobalConfig::default();
        assert_eq!(config.workspaces.packages, vec!["packages/*"]);
        assert_eq!(config.git.default_pr_base, "dev");
        // Services use HashMap with get_port()
        assert_eq!(config.services.get_port("api", 8080), 8080);
    }

    #[test]
    fn test_release_tag_pattern() {
        let config = Config::default();
        let pattern = config.release_tag_pattern("my-package");
        assert_eq!(pattern, "my-package-v*");
    }

    #[test]
    fn test_esc_path() {
        let config = Config::default();
        let namespace = &config.global.esc.namespace;
        assert_eq!(config.esc_path("dev"), format!("{}/dev", namespace));
        assert_eq!(config.esc_path("prod"), format!("{}/prod", namespace));
    }
}
