//! Migration Safety Check (Pattern + AI-powered)
//!
//! Checks database migrations for:
//! 1. Pattern-based violations (fast, no AI needed)
//! 2. AI semantic analysis for complex issues
//! 3. Data migration validation (ensures data migrations exist when needed)

use anyhow::{anyhow, Context, Result};
use console::style;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;

use crate::context::AppContext;

/// Pattern-based migration issue
struct PatternIssue {
    file: String,
    line: usize,
    severity: &'static str, // "error" or "warning"
    message: String,
    hint: String,
}

/// Check migrations for backward compatibility issues
///
/// Returns Ok(true) if safe, Ok(false) if issues found, Err on failure
pub fn check_migrations(ctx: &AppContext, migrations: &[String]) -> Result<bool> {
    if migrations.is_empty() {
        return Ok(true);
    }

    let mut all_safe = true;

    // Step 0: Get completed data migrations (allows unsafe patterns as cleanup)
    let completed_migrations = get_completed_data_migrations(ctx);
    if !completed_migrations.is_empty() {
        println!(
            "  {} Completed data migrations: {}",
            style("ℹ").blue(),
            completed_migrations
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "    {}",
            style("(Unsafe patterns allowed as cleanup for these)").dim()
        );
        println!();
    }

    // Step 1: Fast pattern-based checks (no AI needed)
    println!("  {} Checking migration patterns...", style("◌").cyan());

    let pattern_issues = check_migration_patterns(ctx, migrations, &completed_migrations)?;

    let errors: Vec<_> = pattern_issues
        .iter()
        .filter(|i| i.severity == "error")
        .collect();
    let warnings: Vec<_> = pattern_issues
        .iter()
        .filter(|i| i.severity == "warning")
        .collect();

    if !errors.is_empty() {
        all_safe = false;
        println!(
            "  {} {} error(s) found in migration patterns",
            style("✗").red(),
            errors.len()
        );
        for issue in &errors {
            println!(
                "    {}:{} {}: {}",
                issue.file,
                issue.line,
                style("error").red().bold(),
                issue.message
            );
            println!("      {}", style(&issue.hint).dim());
        }
    }

    if !warnings.is_empty() {
        println!(
            "  {} {} warning(s) in migration patterns",
            style("⚠").yellow(),
            warnings.len()
        );
        for issue in &warnings {
            println!(
                "    {}:{} {}: {}",
                issue.file,
                issue.line,
                style("warn").yellow(),
                issue.message
            );
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        println!("  {} Pattern check passed", style("✓").green());
    }

    // Step 2: Check if data migrations are needed
    println!();
    println!(
        "  {} Checking data migration requirements...",
        style("◌").cyan()
    );

    let data_migration_issues =
        check_data_migration_requirements(ctx, migrations, &completed_migrations)?;
    if !data_migration_issues.is_empty() {
        println!(
            "  {} {} data migration issue(s) found",
            style("⚠").yellow(),
            data_migration_issues.len()
        );
        for issue in &data_migration_issues {
            println!("    {}: {}", style("warn").yellow(), issue);
        }
    } else {
        println!("  {} Data migration check passed", style("✓").green());
    }

    // Step 3: AI semantic analysis (if API key available)
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
        .ok();

    if api_key.is_none() {
        println!();
        println!(
            "  {} AI analysis skipped (no OPENAI_API_KEY or ANTHROPIC_API_KEY)",
            style("~").dim()
        );
        return Ok(all_safe);
    }

    let api_key = api_key.unwrap();
    let use_openai = std::env::var("OPENAI_API_KEY").is_ok();

    println!();
    println!("  {} Analyzing migrations with AI...", style("◌").cyan());

    // Read migration rules from pre-release docs
    let rules_path = ctx.repo.join("docs/pre-release/migration-rules.md");
    let guidelines = if rules_path.exists() {
        std::fs::read_to_string(&rules_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Read migration contents
    let mut migration_contents = String::new();
    for migration_path in migrations {
        let path = ctx.repo.join(migration_path);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            migration_contents.push_str(&format!("\n--- {} ---\n{}\n", migration_path, content));
        }
    }

    if migration_contents.is_empty() {
        return Ok(true);
    }

    let system_prompt = r#"You are a grumpy senior database engineer who has been paged at 3am too many times because of bad migrations. You've seen it all: the "quick fixes" that took down prod, the "it worked in dev" disasters, the "we'll fix it later" promises that never got fixed.

You review migrations with zero tolerance for anything that could cause a rollback nightmare. Your responses are direct, slightly frustrated, and occasionally sarcastic - but always technically accurate.

Remember: There is NO database rollback. When deployments fail, we rollback CODE but the DATABASE stays migrated. Old code must work with new schema. Period."#;

    // Build context about completed data migrations
    let completed_context = if !completed_migrations.is_empty() {
        format!(
            "\n\nCOMPLETED DATA MIGRATIONS:\nThe following data migrations have completed. SQL migrations that clean up after these (DROP COLUMN, RENAME TABLE, etc.) are ALLOWED:\n- {}\n\nLook for comments like '-- CLEANUP_FOR: <name>' or migration names in filenames to identify cleanup migrations.",
            completed_migrations.iter().cloned().collect::<Vec<_>>().join("\n- ")
        )
    } else {
        String::new()
    };

    let user_prompt = format!(
        r#"Review these migrations. I need to know if they're safe to deploy.

PROJECT RULES:
{}{}

MIGRATIONS:
{}

RESPONSE FORMAT:
If safe, just say: SAFE - [brief sarcastic comment about how they managed to not break anything]

If unsafe:
UNSAFE
Then list each issue like:
- [filename]: [what's wrong and why you're disappointed]

Don't sugarcoat it. If something's wrong, I need to know before it's 3am and my phone is ringing."#,
        if guidelines.is_empty() {
            "Standard rules: No DROP/RENAME on active columns. No NOT NULL without DEFAULT."
                .to_string()
        } else {
            guidelines
        },
        completed_context,
        migration_contents
    );

    // Call the appropriate API
    let response = if use_openai {
        call_openai_api(&api_key, system_prompt, &user_prompt)?
    } else {
        call_anthropic_api(&api_key, system_prompt, &user_prompt)?
    };

    // Check if response indicates safety
    let is_safe = response.trim().starts_with("SAFE");

    if is_safe {
        println!("  {} Migration check passed", style("✓").green());
        // Print the sarcastic comment if there is one
        if let Some(comment) = response
            .strip_prefix("SAFE")
            .map(|s| s.trim().trim_start_matches('-').trim())
        {
            if !comment.is_empty() {
                println!("    {}", style(comment).dim());
            }
        }
        Ok(true)
    } else {
        println!("  {} Migration issues found:", style("✗").red());
        // Print the issues
        for line in response.lines() {
            let line = line.trim();
            if !line.is_empty() && line != "UNSAFE" {
                println!("    {}", style(line).yellow());
            }
        }
        Ok(false)
    }
}

/// Get API key from environment (prefers OpenAI)
pub fn get_api_key() -> Option<String> {
    std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
        .ok()
}

// =============================================================================
// Pattern-Based Checks
// =============================================================================

/// Get completed data migrations from workflow table
fn get_completed_data_migrations(_ctx: &AppContext) -> HashSet<String> {
    let database_url = std::env::var("DATABASE_URL").ok();

    if database_url.is_none() {
        // Try to read from deprecated_code.yaml for migrations marked as complete
        // (fallback when no DB access)
        return HashSet::new();
    }

    let url = database_url.unwrap();

    // Query the workflows table for completed migrations
    let output = std::process::Command::new("psql")
        .args([
            &url,
            "-t",
            "-A",
            "-c",
            "SELECT name FROM workflows WHERE workflow_type = 'migration' AND phase = 'completed'",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => HashSet::new(),
    }
}

/// Check if a SQL migration is related to a completed data migration
/// Uses naming conventions and comments to match them
fn is_cleanup_for_completed_migration(
    content: &str,
    filename: &str,
    completed_migrations: &HashSet<String>,
) -> Option<String> {
    // Check for explicit cleanup marker: -- CLEANUP_FOR: migration_name
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("-- CLEANUP_FOR:") {
            let name = rest.trim();
            if completed_migrations.contains(name) {
                return Some(name.to_string());
            }
        }
    }

    // Check if filename contains a completed migration name
    // e.g., "20260120_cleanup_aad_v2_encryption.sql" matches "aad_v2_encryption"
    let filename_lower = filename.to_lowercase();
    for migration in completed_migrations {
        if filename_lower.contains(&migration.to_lowercase()) {
            return Some(migration.clone());
        }
    }

    // Check for migration name in comments
    for migration in completed_migrations {
        if content.contains(&format!("MIGRATION_DEPRECATED: {}", migration))
            || content.contains(&format!("cleanup for {}", migration))
            || content.contains(&format!("Cleanup for {}", migration))
        {
            return Some(migration.clone());
        }
    }

    None
}

/// Check migration files for dangerous patterns
fn check_migration_patterns(
    ctx: &AppContext,
    migrations: &[String],
    completed_migrations: &HashSet<String>,
) -> Result<Vec<PatternIssue>> {
    let mut issues = Vec::new();

    // Load allowlist
    let allowlist_path = ctx.repo.join("scripts/migration-allowlist.txt");
    let allowlist: HashSet<String> = if allowlist_path.exists() {
        std::fs::read_to_string(&allowlist_path)?
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .map(|s| s.to_string())
            .collect()
    } else {
        HashSet::new()
    };

    for migration_path in migrations {
        let path = ctx.repo.join(migration_path);
        if !path.exists() {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(migration_path);

        // Skip if in allowlist
        if allowlist.contains(filename) {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let content_upper = content.to_uppercase();
        let has_safe_marker = content.contains("MIGRATION_SAFE");

        // Check if this is cleanup for a completed data migration
        let cleanup_migration =
            is_cleanup_for_completed_migration(&content, filename, completed_migrations);
        let is_cleanup = cleanup_migration.is_some();

        // Check each pattern
        for (line_num, line) in content.lines().enumerate() {
            let line_upper = line.to_uppercase();
            let line_num = line_num + 1;

            // ERROR: DROP COLUMN (allowed as cleanup after data migration)
            if Regex::new(r"(?i)DROP\s+COLUMN")
                .unwrap()
                .is_match(&line_upper)
            {
                if is_cleanup {
                    // This is cleanup - downgrade to info (not even warning)
                    continue;
                }
                if !has_safe_marker {
                    issues.push(PatternIssue {
                        file: filename.to_string(),
                        line: line_num,
                        severity: "error",
                        message: "DROP COLUMN detected".to_string(),
                        hint: "Old code may still reference this column. Either: 1) Wait 2+ weeks after code stops using it, 2) Complete a data migration first, or 3) Add '-- MIGRATION_SAFE: column unused since YYYY-MM-DD' to acknowledge.".to_string(),
                    });
                }
            }

            // ERROR: RENAME COLUMN (allowed as cleanup after data migration)
            if Regex::new(r"(?i)RENAME\s+COLUMN")
                .unwrap()
                .is_match(&line_upper)
            {
                if is_cleanup {
                    continue;
                }
                if !has_safe_marker {
                    issues.push(PatternIssue {
                        file: filename.to_string(),
                        line: line_num,
                        severity: "error",
                        message: "RENAME COLUMN detected".to_string(),
                        hint: "This breaks old code immediately. Use expand-contract pattern: 1. Add new column 2. Run data migration 3. Drop old column after migration completes".to_string(),
                    });
                }
            }

            // ERROR: RENAME TABLE (allowed as cleanup after data migration)
            if Regex::new(r"(?i)RENAME\s+(TABLE|TO)")
                .unwrap()
                .is_match(&line_upper)
            {
                if is_cleanup {
                    continue;
                }
                if !has_safe_marker {
                    issues.push(PatternIssue {
                        file: filename.to_string(),
                        line: line_num,
                        severity: "error",
                        message: "RENAME TABLE detected".to_string(),
                        hint: "This breaks old code immediately. Create new table + run data migration instead.".to_string(),
                    });
                }
            }

            // ERROR: DROP TABLE (allowed as cleanup after data migration)
            if Regex::new(r"(?i)DROP\s+TABLE")
                .unwrap()
                .is_match(&line_upper)
            {
                if is_cleanup {
                    continue;
                }
                if !has_safe_marker {
                    issues.push(PatternIssue {
                        file: filename.to_string(),
                        line: line_num,
                        severity: "error",
                        message: "DROP TABLE detected".to_string(),
                        hint: "Ensure no code references this table. Add '-- MIGRATION_SAFE: table unused since YYYY-MM-DD' or complete a data migration first.".to_string(),
                    });
                }
            }

            // ERROR: TRUNCATE (still requires explicit acknowledgment even for cleanup)
            if Regex::new(r"(?i)\bTRUNCATE\b")
                .unwrap()
                .is_match(&line_upper)
            {
                if !has_safe_marker {
                    issues.push(PatternIssue {
                        file: filename.to_string(),
                        line: line_num,
                        severity: "error",
                        message: "TRUNCATE detected".to_string(),
                        hint: "This deletes all data. Add '-- MIGRATION_SAFE: intentional data deletion' to acknowledge.".to_string(),
                    });
                }
            }

            // WARNING: SET NOT NULL
            if Regex::new(r"(?i)SET\s+NOT\s+NULL")
                .unwrap()
                .is_match(&line_upper)
            {
                issues.push(PatternIssue {
                    file: filename.to_string(),
                    line: line_num,
                    severity: "warning",
                    message: "SET NOT NULL on existing column".to_string(),
                    hint: "Ensure no NULL values exist and old code always provides a value."
                        .to_string(),
                });
            }

            // WARNING: ADD CONSTRAINT
            if Regex::new(r"(?i)ADD\s+CONSTRAINT")
                .unwrap()
                .is_match(&line_upper)
            {
                issues.push(PatternIssue {
                    file: filename.to_string(),
                    line: line_num,
                    severity: "warning",
                    message: "ADD CONSTRAINT detected".to_string(),
                    hint: "Ensure existing data doesn't violate this constraint.".to_string(),
                });
            }

            // WARNING: ALTER COLUMN TYPE
            if Regex::new(r"(?i)ALTER\s+COLUMN.*TYPE")
                .unwrap()
                .is_match(&line_upper)
            {
                issues.push(PatternIssue {
                    file: filename.to_string(),
                    line: line_num,
                    severity: "warning",
                    message: "ALTER COLUMN TYPE detected".to_string(),
                    hint: "This may fail or truncate data. Test against production data copy."
                        .to_string(),
                });
            }
        }

        // ERROR: ADD COLUMN NOT NULL without DEFAULT (check full content)
        if Regex::new(r"(?i)ADD\s+COLUMN.*NOT\s+NULL")
            .unwrap()
            .is_match(&content_upper)
            && !Regex::new(r"(?i)DEFAULT").unwrap().is_match(&content_upper)
        {
            issues.push(PatternIssue {
                file: filename.to_string(),
                line: 1,
                severity: "error",
                message: "ADD COLUMN ... NOT NULL without DEFAULT".to_string(),
                hint: "Old code won't provide this value on INSERT. Either add DEFAULT or make nullable first.".to_string(),
            });
        }

        // WARNING: DELETE without WHERE
        if Regex::new(r"(?i)DELETE\s+FROM")
            .unwrap()
            .is_match(&content_upper)
            && !Regex::new(r"(?i)WHERE").unwrap().is_match(&content_upper)
        {
            issues.push(PatternIssue {
                file: filename.to_string(),
                line: 1,
                severity: "warning",
                message: "DELETE without WHERE clause".to_string(),
                hint: "This deletes all rows. Is this intentional?".to_string(),
            });
        }
    }

    Ok(issues)
}

// =============================================================================
// Data Migration Requirement Checks
// =============================================================================

/// Patterns that typically require a data migration
const DATA_MIGRATION_TRIGGERS: &[(&str, &str)] = &[
    (
        r"(?i)encrypted",
        "Changes to encrypted columns typically require data migration to re-encrypt existing data",
    ),
    (
        r"(?i)aad|additional_authenticated_data",
        "AAD changes require re-encrypting all affected entries",
    ),
    (
        r"(?i)backfill|populate|seed",
        "Backfill operations should use data migrations for large datasets",
    ),
];

/// Check if SQL migrations require corresponding data migrations
fn check_data_migration_requirements(
    ctx: &AppContext,
    migrations: &[String],
    completed_migrations: &HashSet<String>,
) -> Result<Vec<String>> {
    let mut issues = Vec::new();

    // Load registered data migrations
    let data_migrations_dir = ctx.repo.join("packages/api-core/src/data_migrations");
    let registered_migrations: HashSet<String> = if data_migrations_dir.exists() {
        std::fs::read_dir(&data_migrations_dir)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.ends_with(".rs") && name != "mod.rs" {
                    Some(name.trim_end_matches(".rs").to_string())
                } else {
                    None
                }
            })
            .collect()
    } else {
        HashSet::new()
    };

    // Load deprecated_code.yaml to check for expected data migrations
    let manifest_path = ctx.repo.join("packages/api-core/deprecated_code.yaml");
    let manifest_migrations: HashSet<String> = if manifest_path.exists() {
        // Simple YAML parsing - look for migration names
        let content = std::fs::read_to_string(&manifest_path)?;
        let mut migrations = HashSet::new();
        let mut in_migrations = false;
        for line in content.lines() {
            if line.trim() == "migrations:" {
                in_migrations = true;
                continue;
            }
            if in_migrations && !line.starts_with(' ') && !line.starts_with('#') {
                in_migrations = false;
            }
            if in_migrations && line.starts_with("  ") && !line.starts_with("    ") {
                // This is a migration key
                if let Some(name) = line.trim().strip_suffix(':') {
                    if !name.starts_with('#') {
                        migrations.insert(name.to_string());
                    }
                }
            }
        }
        migrations
    } else {
        HashSet::new()
    };

    // Check deprecated_code.yaml entries have corresponding data migrations
    for manifest_migration in &manifest_migrations {
        if !registered_migrations.contains(manifest_migration) {
            issues.push(format!(
                "deprecated_code.yaml references '{}' but no data migration found in src/data_migrations/",
                manifest_migration
            ));
        }
    }

    // Check SQL migrations for patterns that suggest data migration is needed
    for migration_path in migrations {
        let path = ctx.repo.join(migration_path);
        if !path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(migration_path);

        // Check migration comments for data migration hints
        if content.contains("-- DATA_MIGRATION_REQUIRED") {
            let has_data_migration = content.lines().any(|line| {
                if let Some(rest) = line.strip_prefix("-- DATA_MIGRATION_REQUIRED:") {
                    let name = rest.trim();
                    registered_migrations.contains(name)
                } else {
                    false
                }
            });

            if !has_data_migration {
                issues.push(format!(
                    "{}: Migration marked as requiring data migration but none found",
                    filename
                ));
            }
        }

        // Check for trigger patterns (skip if this is cleanup for a completed migration)
        let is_cleanup =
            is_cleanup_for_completed_migration(&content, filename, completed_migrations).is_some();

        if !is_cleanup {
            for (pattern, reason) in DATA_MIGRATION_TRIGGERS {
                if Regex::new(pattern).unwrap().is_match(&content) {
                    // Only warn if this looks like a substantive change
                    if content.contains("ALTER") || content.contains("UPDATE") {
                        issues.push(format!(
                            "{}: {}. Consider adding a data migration.",
                            filename, reason
                        ));
                        break; // One warning per file
                    }
                }
            }
        }
    }

    Ok(issues)
}

/// Call AI API (OpenAI or Anthropic based on available key)
pub fn call_ai(api_key: &str, system: &str, user: &str) -> Result<String> {
    let use_openai = std::env::var("OPENAI_API_KEY").is_ok();
    if use_openai {
        call_openai_api(api_key, system, user)
    } else {
        call_anthropic_api(api_key, system, user)
    }
}

fn call_anthropic_api(api_key: &str, system: &str, user: &str) -> Result<String> {
    let payload = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "system": system,
        "messages": [{"role": "user", "content": user}]
    });

    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "https://api.anthropic.com/v1/messages",
            "-H",
            &format!("x-api-key: {}", api_key),
            "-H",
            "anthropic-version: 2023-06-01",
            "-H",
            "content-type: application/json",
            "-d",
            &payload.to_string(),
        ])
        .output()?;

    let response: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse Anthropic API response")?;

    response["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("No text in Anthropic response: {:?}", response))
}

fn call_openai_api(api_key: &str, system: &str, user: &str) -> Result<String> {
    let payload = serde_json::json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ]
    });

    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "https://api.openai.com/v1/chat/completions",
            "-H",
            &format!("Authorization: Bearer {}", api_key),
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload.to_string(),
        ])
        .output()?;

    let response: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse OpenAI API response")?;

    response["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("No content in OpenAI response: {:?}", response))
}

/// Get migration files from git diff
#[allow(dead_code)]
pub fn get_migration_files_from_diff(repo_root: &Path, base_branch: &str) -> Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", &format!("{}..HEAD", base_branch)])
        .current_dir(repo_root)
        .output()?;

    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|f| f.contains("migrations/") && f.ends_with(".sql"))
        .map(|s| s.to_string())
        .collect();

    Ok(files)
}
