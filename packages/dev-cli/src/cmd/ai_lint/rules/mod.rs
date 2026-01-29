//! Lint rules organized by category
//!
//! Each category has its own module with rules prefixed by a unique ID.

pub mod cleanup;
pub mod migrations;
pub mod sec;
pub mod tc;

pub use cleanup::check_cleanup;
#[allow(unused_imports)]
pub use cleanup::list_deprecated_markers;
pub use migrations::{call_ai, check_migrations, get_api_key};
pub use sec::get_security_rules;
pub use tc::{check_test_coverage_ai, get_test_coverage_rules};

use super::engine::LintRule;

/// Lint categories available via `ai lint [CATEGORY]`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCategory {
    /// Test coverage rules (TST*)
    TestCoverage,
    /// Security/infrastructure rules (WF*, INF*, DKR*, AUTH*, ENV*)
    Security,
    /// Migration safety (AI-powered)
    Migrations,
    /// Migration cleanup (deprecated code detection)
    Cleanup,
    /// All rules
    All,
}

impl LintCategory {
    /// Parse from string (CLI argument)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tc" | "test-coverage" | "testcoverage" => Some(Self::TestCoverage),
            "sec" | "s" | "security" => Some(Self::Security),
            "m" | "migrations" | "migrate" => Some(Self::Migrations),
            "c" | "cleanup" | "clean" | "deprecated" => Some(Self::Cleanup),
            "all" | "a" | "" => Some(Self::All),
            _ => None,
        }
    }

    /// Get display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::TestCoverage => "Test Coverage",
            Self::Security => "Security",
            Self::Migrations => "Migrations",
            Self::Cleanup => "Migration Cleanup",
            Self::All => "All",
        }
    }

    /// Get the ignore directive prefix for this category
    pub fn ignore_prefix(&self) -> &'static str {
        match self {
            Self::TestCoverage => "ai-test-ignore",
            Self::Security => "ai-lint-ignore",
            Self::Migrations => "ai-lint-ignore",
            Self::Cleanup => "ai-lint-ignore",
            Self::All => "ai-lint-ignore",
        }
    }
}

/// Get all rules for a category (excludes migrations which is AI-powered)
pub fn get_rules(category: LintCategory) -> Vec<LintRule> {
    match category {
        LintCategory::TestCoverage => get_test_coverage_rules(),
        LintCategory::Security => get_security_rules(),
        LintCategory::Migrations => vec![], // Migrations uses AI, not pattern rules
        LintCategory::Cleanup => vec![],    // Cleanup uses custom logic, not pattern rules
        LintCategory::All => {
            let mut rules = get_security_rules();
            rules.extend(get_test_coverage_rules());
            rules
        }
    }
}

/// Get rule categories for help display
pub fn get_rule_categories(category: LintCategory) -> Vec<(&'static str, &'static str)> {
    match category {
        LintCategory::TestCoverage => vec![("TST0", "Test Quality"), ("TST1", "Test Coverage")],
        LintCategory::Security => vec![
            ("WF", "Workflow (GitHub Actions)"),
            ("INF", "Infrastructure (Pulumi/AWS)"),
            ("DKR", "Docker"),
            ("AUTH", "Authentication"),
            ("ENV", "Environment Files"),
        ],
        LintCategory::Migrations => vec![],
        LintCategory::Cleanup => vec![],
        LintCategory::All => {
            let mut cats = vec![
                ("WF", "Workflow (GitHub Actions)"),
                ("INF", "Infrastructure (Pulumi/AWS)"),
                ("DKR", "Docker"),
                ("AUTH", "Authentication"),
                ("ENV", "Environment Files"),
            ];
            cats.extend(vec![("TST0", "Test Quality"), ("TST1", "Test Coverage")]);
            cats
        }
    }
}
