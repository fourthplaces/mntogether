//! Security Rules (WF*, INF*, DKR*, AUTH*, ENV*)
//!
//! Rules for detecting security issues in sensitive files:
//! - GitHub Actions workflows
//! - Infrastructure code (Pulumi/AWS)
//! - Docker configurations
//! - Authentication code
//! - Environment files

use regex::Regex;

use super::super::engine::{LintRule, Severity, Violation};

/// Get all security rules
pub fn get_security_rules() -> Vec<LintRule> {
    let mut rules = Vec::new();
    rules.extend(workflow_rules());
    rules.extend(infrastructure_rules());
    rules.extend(docker_rules());
    rules.extend(auth_rules());
    rules.extend(env_rules());
    rules
}

/// Check if a file path matches sensitive patterns that should be linted
pub fn is_sensitive_path(path: &str) -> bool {
    let patterns = [
        "infra/packages/",
        ".github/workflows/",
        "domains/member/",
        "domains/vault/",
        "domains/entry/actions/",     // Entry actions handle encrypted data
        "domains/container/actions/", // Container actions handle permissions
        "domains/agent/",             // Agent code handles user data
        "Dockerfile",
        "docker-compose",
        ".env",
    ];

    // Exclude node_modules and other non-source files
    if path.contains("node_modules") || path.contains(".d.ts") {
        return false;
    }

    patterns.iter().any(|p| path.contains(p))
}

// =============================================================================
// Workflow Rules (WF*)
// =============================================================================

fn workflow_rules() -> Vec<LintRule> {
    vec![
        LintRule {
            id: "WF001",
            name: "unpinned-action",
            description: "GitHub Action not pinned to SHA",
            applies_to: &[".github/workflows/"],
            pattern: None,
            severity: Severity::Error,
            check_fn: Some(check_unpinned_actions),
        },
        LintRule {
            id: "WF002",
            name: "secrets-in-echo",
            description: "Potential secret exposure via echo/print",
            applies_to: &[".github/workflows/"],
            pattern: Some(r#"(?i)(echo|print|printf)\s+.*\$\{\{\s*secrets\."#),
            severity: Severity::Error,
            check_fn: None,
        },
        LintRule {
            id: "WF003",
            name: "env-dump",
            description: "Environment dump that could expose secrets",
            applies_to: &[".github/workflows/"],
            pattern: Some(r"(?i)(^\s*printenv\b|run:.*\benv\s*$|run:.*\bset\s*$|\$ENV\b)"),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "WF004",
            name: "write-all-permissions",
            description: "Overly permissive write-all permissions",
            applies_to: &[".github/workflows/"],
            pattern: Some(r"permissions:\s*write-all"),
            severity: Severity::Error,
            check_fn: None,
        },
        LintRule {
            id: "WF005",
            name: "command-injection",
            description: "Potential command injection via ${{ }} in run",
            applies_to: &[".github/workflows/"],
            pattern: None,
            severity: Severity::Error,
            check_fn: Some(check_command_injection),
        },
        LintRule {
            id: "WF006",
            name: "pull-request-target",
            description: "pull_request_target event requires careful handling",
            applies_to: &[".github/workflows/"],
            pattern: Some(r"on:\s*\n\s*pull_request_target"),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "WF007",
            name: "hardcoded-credentials",
            description: "Hardcoded credentials or API keys",
            applies_to: &[".github/workflows/"],
            pattern: Some(r#"(?i)(password|api[_-]?key|secret|token)\s*[:=]\s*["'][^"'\$\{]+"#),
            severity: Severity::Error,
            check_fn: None,
        },
        LintRule {
            id: "WF008",
            name: "third-party-action",
            description: "Third-party action (not GitHub-owned)",
            applies_to: &[".github/workflows/"],
            pattern: None,
            severity: Severity::Warning,
            check_fn: Some(check_third_party_actions),
        },
    ]
}

// =============================================================================
// Infrastructure Rules (INF*)
// =============================================================================

fn infrastructure_rules() -> Vec<LintRule> {
    vec![
        LintRule {
            id: "INF001",
            name: "public-ingress",
            description: "Security group allows 0.0.0.0/0 ingress",
            applies_to: &["infra/"],
            pattern: Some(r#"(?i)(cidr|ingress).*0\.0\.0\.0/0"#),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "INF002",
            name: "wildcard-iam",
            description: "IAM policy with wildcard (*) actions",
            applies_to: &["infra/"],
            pattern: Some(r#"(?i)(actions|Action)\s*[:=]\s*\[?\s*["']\*["']"#),
            severity: Severity::Error,
            check_fn: None,
        },
        LintRule {
            id: "INF003",
            name: "hardcoded-secret",
            description: "Hardcoded secret in infrastructure code",
            applies_to: &["infra/"],
            pattern: Some(r#"(?i)(password|secret|api[_-]?key)\s*[:=]\s*["'][^"'\$\{]+"#),
            severity: Severity::Error,
            check_fn: None,
        },
        LintRule {
            id: "INF004",
            name: "missing-encryption",
            description: "Resource may be missing encryption",
            applies_to: &["infra/"],
            pattern: Some(r"(?i)encrypted\s*[:=]\s*false"),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "INF005",
            name: "public-bucket",
            description: "S3 bucket with public access",
            applies_to: &["infra/"],
            pattern: Some(r"(?i)(publicRead|public-read|BlockPublicAccess.*false)"),
            severity: Severity::Error,
            check_fn: None,
        },
    ]
}

// =============================================================================
// Docker Rules (DKR*)
// =============================================================================

fn docker_rules() -> Vec<LintRule> {
    vec![
        LintRule {
            id: "DKR001",
            name: "latest-tag",
            description: "Using :latest tag instead of pinned version",
            applies_to: &["Dockerfile", "docker-compose"],
            pattern: Some(r"(?i)(FROM|image:)\s+\S+:latest"),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "DKR002",
            name: "secret-in-env",
            description: "Secret passed via ENV or environment",
            applies_to: &["Dockerfile", "docker-compose"],
            pattern: Some(
                r#"(?i)(ENV|environment:).*(?:password|secret|api[_-]?key|token)\s*[:=]"#,
            ),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "DKR003",
            name: "secret-in-arg",
            description: "Secret passed via build ARG",
            applies_to: &["Dockerfile"],
            pattern: Some(r#"(?i)ARG\s+(?:password|secret|api[_-]?key|token)"#),
            severity: Severity::Error,
            check_fn: None,
        },
        LintRule {
            id: "DKR004",
            name: "privileged-mode",
            description: "Container running in privileged mode",
            applies_to: &["docker-compose"],
            pattern: Some(r"(?i)privileged:\s*true"),
            severity: Severity::Warning,
            check_fn: None,
        },
    ]
}

// =============================================================================
// Auth Rules (AUTH*)
// =============================================================================

fn auth_rules() -> Vec<LintRule> {
    vec![
        LintRule {
            id: "AUTH001",
            name: "logged-secret",
            description: "Potential secret logged (password, token, code)",
            applies_to: &["domains/member/", "auth"],
            pattern: Some(
                r#"(?i)(info!|debug!|warn!|error!|trace!|log::).*(?:password|token|code|secret)"#,
            ),
            severity: Severity::Error,
            check_fn: None,
        },
        LintRule {
            id: "AUTH002",
            name: "raw-identifier-log",
            description: "Raw identifier (email/phone) may be logged",
            applies_to: &["domains/member/", "auth"],
            pattern: Some(r#"(?i)(info!|debug!|warn!|error!).*(?:email|phone|identifier)[^_]"#),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "AUTH003",
            name: "timing-unsafe-compare",
            description: "Non-constant-time comparison for secrets",
            applies_to: &["domains/member/", "auth"],
            pattern: Some(r#"(?i)(?:password|token|secret|code)\s*==\s*"#),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "AUTH004",
            name: "missing-permission-check",
            description: "Action returns sensitive data without permission check",
            applies_to: &[
                "domains/entry/actions/",
                "domains/container/actions/",
                "domains/agent/",
            ],
            pattern: None,
            severity: Severity::Error,
            check_fn: Some(check_missing_permission),
        },
        LintRule {
            id: "AUTH005",
            name: "data-access-without-visitor",
            description: "Action accesses user data without visitor_id parameter",
            applies_to: &["domains/entry/actions/", "domains/container/actions/"],
            pattern: None,
            severity: Severity::Warning,
            check_fn: Some(check_missing_visitor_param),
        },
    ]
}

// =============================================================================
// Environment Rules (ENV*)
// =============================================================================

fn env_rules() -> Vec<LintRule> {
    vec![
        LintRule {
            id: "ENV001",
            name: "committed-secret",
            description: "Secret value in environment file",
            applies_to: &[".env"],
            pattern: Some(r#"(?i)(?:password|secret|api[_-]?key|token)\s*=\s*[^$\{][^\s]+"#),
            severity: Severity::Error,
            check_fn: None,
        },
        LintRule {
            id: "ENV002",
            name: "production-in-dev",
            description: "Production URL/credentials in dev config",
            applies_to: &[".env"],
            pattern: Some(r"(?i)(prod|production)\.(aws|database|redis)"),
            severity: Severity::Warning,
            check_fn: None,
        },
    ]
}

// =============================================================================
// Custom Check Functions
// =============================================================================

/// Check for actions not pinned to SHA (WF001)
fn check_unpinned_actions(path: &str, content: &str) -> Vec<Violation> {
    if !path.contains(".github/workflows/") {
        return vec![];
    }

    let mut violations = Vec::new();
    let uses_re = Regex::new(r"uses:\s+([^@\s]+)@([^\s]+)").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = uses_re.captures(line) {
            let action = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let version = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            // Skip local actions
            if action.starts_with("./") {
                continue;
            }

            // Check if pinned to SHA (40 hex characters)
            let is_pinned =
                version.len() >= 40 && version.chars().take(40).all(|c| c.is_ascii_hexdigit());

            if !is_pinned {
                violations.push(Violation {
                    rule_id: "WF001",
                    rule_name: "unpinned-action",
                    line: line_num + 1,
                    matched_text: line.trim().to_string(),
                    severity: Severity::Error,
                    message: format!("Action '{}@{}' not pinned to SHA", action, version),
                });
            }
        }
    }

    violations
}

/// Check for command injection via ${{ }} in run steps
fn check_command_injection(path: &str, content: &str) -> Vec<Violation> {
    if !path.contains(".github/workflows/") {
        return vec![];
    }

    let mut violations = Vec::new();
    let dangerous_contexts = [
        "github.event.issue.title",
        "github.event.issue.body",
        "github.event.pull_request.title",
        "github.event.pull_request.body",
        "github.event.comment.body",
        "github.event.review.body",
        "github.event.head_commit.message",
        "github.head_ref",
    ];

    for (line_num, line) in content.lines().enumerate() {
        // Only check inside run: blocks
        if !line.trim().starts_with("run:") && !line.contains("run: |") {
            continue;
        }

        for ctx in &dangerous_contexts {
            let pattern = format!("${{{{ {} }}}}", ctx);
            if line.contains(&pattern) || line.contains(&pattern.replace(" ", "")) {
                violations.push(Violation {
                    rule_id: "WF005",
                    rule_name: "command-injection",
                    line: line_num + 1,
                    matched_text: line.trim().to_string(),
                    severity: Severity::Error,
                    message: format!("Potential command injection: {} is user-controlled", ctx),
                });
            }
        }
    }

    violations
}

/// Check for third-party (non-GitHub-owned) actions
fn check_third_party_actions(path: &str, content: &str) -> Vec<Violation> {
    if !path.contains(".github/workflows/") {
        return vec![];
    }

    let mut violations = Vec::new();
    let github_owned = ["actions/", "github/"];
    let uses_re = Regex::new(r"uses:\s+([^@\s]+)@").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = uses_re.captures(line) {
            let action = caps.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip local actions
            if action.starts_with("./") {
                continue;
            }

            // Check if it's GitHub-owned
            let is_github_owned = github_owned.iter().any(|prefix| action.starts_with(prefix));

            if !is_github_owned {
                // Check if it's pinned to SHA
                let is_pinned = line.contains("@") && {
                    let after_at = line.split('@').nth(1).unwrap_or("");
                    after_at.len() >= 40 && after_at.chars().take(40).all(|c| c.is_ascii_hexdigit())
                };

                let severity = if is_pinned {
                    Severity::Warning
                } else {
                    Severity::Error
                };

                let msg = if is_pinned {
                    format!(
                        "Third-party action '{}' (pinned to SHA, but verify trust)",
                        action
                    )
                } else {
                    format!("Third-party action '{}' not pinned to SHA", action)
                };

                violations.push(Violation {
                    rule_id: "WF008",
                    rule_name: "third-party-action",
                    line: line_num + 1,
                    matched_text: line.trim().to_string(),
                    severity,
                    message: msg,
                });
            }
        }
    }

    violations
}

/// Check for action functions that return sensitive data without permission checks (AUTH004)
/// This catches vulnerabilities like get_entry_analysis returning data without authorization.
fn check_missing_permission(path: &str, content: &str) -> Vec<Violation> {
    // Only check action files
    if !path.contains("/actions/") {
        return vec![];
    }

    let mut violations = Vec::new();

    // Sensitive type names that require authorization
    let sensitive_types = ["Entry", "EntryAnalysis", "Vault", "Member", "Container"];

    // Patterns indicating permission checks
    let permission_patterns = [
        "assert_container_permission",
        "assert_self_or_admin",
        "check_permission",
        "can_access",
        "PermissionLevel",
    ];

    // Find public async functions that might be actions
    // Match function signatures with GenericActionResult return type
    let fn_re = Regex::new(r"pub async fn (\w+)\s*\(").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = fn_re.captures(line) {
            let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip internal/helper functions
            if fn_name.starts_with('_')
                || fn_name.starts_with("fetch_")
                || fn_name.starts_with("build_")
                || fn_name.starts_with("process_")
            {
                continue;
            }

            // Get the function signature and body (next ~60 lines)
            let fn_context: String = content
                .lines()
                .skip(line_num)
                .take(60)
                .collect::<Vec<_>>()
                .join("\n");

            // Check if function returns GenericActionResult with sensitive type
            let returns_sensitive = fn_context.contains("GenericActionResult")
                && sensitive_types.iter().any(|t| fn_context.contains(t));

            if returns_sensitive {
                // Check for permission checks in the function body
                let has_permission_check =
                    permission_patterns.iter().any(|p| fn_context.contains(p));

                if !has_permission_check {
                    violations.push(Violation {
                        rule_id: "AUTH004",
                        rule_name: "missing-permission-check",
                        line: line_num + 1,
                        matched_text: line.trim().to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "Function '{}' returns sensitive data without permission check. \
                             Add assert_container_permission() or similar check.",
                            fn_name
                        ),
                    });
                }
            }
        }
    }

    violations
}

/// Check for action functions accessing user data without visitor_id parameter (AUTH005)
fn check_missing_visitor_param(path: &str, content: &str) -> Vec<Violation> {
    // Only check action files
    if !path.contains("/actions/") {
        return vec![];
    }

    let mut violations = Vec::new();

    // Find Option structs that might be missing visitor_id
    let struct_re = Regex::new(r"pub struct (\w+Options)\s*\{").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = struct_re.captures(line) {
            let struct_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip internal structs
            if struct_name.starts_with('_') {
                continue;
            }

            // Get struct body (next ~20 lines)
            let struct_body: String = content
                .lines()
                .skip(line_num)
                .take(20)
                .take_while(|l| !l.trim().starts_with('}') || l.contains('{'))
                .collect::<Vec<_>>()
                .join("\n");

            // Check if it has visitor_id field
            let has_visitor_id = struct_body.contains("visitor_id");

            // Check if this is a "Get" operation (reading data)
            let is_get_operation = struct_name.starts_with("Get");

            if is_get_operation && !has_visitor_id {
                violations.push(Violation {
                    rule_id: "AUTH005",
                    rule_name: "data-access-without-visitor",
                    line: line_num + 1,
                    matched_text: line.trim().to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Options struct '{}' is missing visitor_id field. \
                         Consider adding visitor_id for permission checking.",
                        struct_name
                    ),
                });
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::ai_lint::engine::check_file;
    use std::collections::HashSet;

    #[test]
    fn test_unpinned_action() {
        let content = "uses: actions/checkout@v4";
        let rules = get_security_rules();
        let violations = check_file(".github/workflows/ci.yml", content, &rules, &HashSet::new());
        assert!(violations.iter().any(|v| v.rule_id == "WF001"));
    }

    #[test]
    fn test_pinned_action_ok() {
        let content = "uses: actions/checkout@692973e3d937129bcbf40652eb9f2f61becf3332";
        let rules = get_security_rules();
        let violations = check_file(".github/workflows/ci.yml", content, &rules, &HashSet::new());
        assert!(!violations.iter().any(|v| v.rule_id == "WF001"));
    }

    #[test]
    fn test_third_party_action_pinned() {
        let content = "uses: gaurav-nelson/github-action-markdown-link-check@d53a906aa6b22b8979d33bc86170567e619495ec";
        let rules = get_security_rules();
        let violations = check_file(".github/workflows/ci.yml", content, &rules, &HashSet::new());
        // Should warn but not error because it's pinned
        let v = violations.iter().find(|v| v.rule_id == "WF008");
        assert!(v.is_some());
        assert_eq!(v.unwrap().severity, Severity::Warning);
    }

    #[test]
    fn test_missing_permission_check() {
        // This simulates the vulnerability we found in get_entry_analysis
        let content = r#"
pub struct GetAnalysisOptions {
    pub entry_id: Uuid,
}

pub async fn get_analysis(
    options: GetAnalysisOptions,
    ctx: DomainContext,
) -> Result<GenericActionResult<Option<EntryAnalysis>>> {
    let analysis = EntryAnalysis::find_by_entry_id(options.entry_id, &ctx.deps().db_connection).await;
    Ok(GenericActionResult { value: analysis.ok() })
}
"#;
        let rules = get_security_rules();
        let violations = check_file(
            "packages/api-core/src/domains/entry/actions/analysis.rs",
            content,
            &rules,
            &HashSet::new(),
        );
        assert!(
            violations.iter().any(|v| v.rule_id == "AUTH004"),
            "Should detect missing permission check: {:?}",
            violations
        );
    }

    #[test]
    fn test_with_permission_check_ok() {
        // This has a proper permission check
        let content = r#"
pub struct GetAnalysisOptions {
    pub entry_id: Uuid,
    pub visitor_id: Option<MemberId>,
}

pub async fn get_analysis(
    options: GetAnalysisOptions,
    ctx: DomainContext,
) -> Result<GenericActionResult<Option<EntryAnalysis>>> {
    let entry = Entry::find_by_id(options.entry_id, &ctx.deps().db_connection).await?;

    let visitor_id = options.visitor_id.ok_or_else(|| anyhow::anyhow!("Auth required"))?;
    assert_container_permission(entry.container_id, visitor_id, PermissionLevel::Read, ctx.deps()).await?;

    let analysis = EntryAnalysis::find_by_entry_id(options.entry_id, &ctx.deps().db_connection).await;
    Ok(GenericActionResult { value: analysis.ok() })
}
"#;
        let rules = get_security_rules();
        let violations = check_file(
            "packages/api-core/src/domains/entry/actions/analysis.rs",
            content,
            &rules,
            &HashSet::new(),
        );
        assert!(
            !violations.iter().any(|v| v.rule_id == "AUTH004"),
            "Should NOT flag when permission check exists: {:?}",
            violations
        );
    }

    #[test]
    fn test_missing_visitor_id_warning() {
        let content = r#"
pub struct GetEntryDataOptions {
    pub entry_id: Uuid,
}
"#;
        let rules = get_security_rules();
        let violations = check_file(
            "packages/api-core/src/domains/entry/actions/mod.rs",
            content,
            &rules,
            &HashSet::new(),
        );
        assert!(
            violations.iter().any(|v| v.rule_id == "AUTH005"),
            "Should warn about missing visitor_id: {:?}",
            violations
        );
    }
}
