//! Test Coverage Rules
//!
//! AI-powered analysis of test coverage on changed code.
//! Analyzes git diffs to identify business flows lacking test coverage.

use anyhow::Result;
use console::style;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use super::super::engine::{parse_ignored_rules, LintRule, Severity, Violation};
use crate::context::AppContext;

/// Context needed for test coverage analysis (diff-based checks)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TestCoverageContext {
    /// Test files in the current diff
    pub test_files_in_diff: HashSet<String>,
    /// Discovered test patterns from codebase
    pub test_patterns: Vec<TestPattern>,
}

/// Discovered test pattern from the codebase
#[derive(Debug, Clone)]
pub struct TestPattern {
    /// Source file pattern (e.g., "{dir}/{name}.rs")
    pub source_pattern: String,
    /// Test file pattern (e.g., "{dir}/{name}_test.rs")
    pub test_pattern: String,
    /// Number of examples found
    pub match_count: usize,
}

/// Get all test coverage rules
pub fn get_test_coverage_rules() -> Vec<LintRule> {
    let mut rules = Vec::new();
    rules.extend(test_quality_rules());
    rules.extend(coverage_rules());
    rules
}

// =============================================================================
// Test Quality Rules (TST001-TST099)
// =============================================================================

fn test_quality_rules() -> Vec<LintRule> {
    vec![
        LintRule {
            id: "TST001",
            name: "empty-test",
            description: "Test function with no assertions",
            applies_to: &["_test.rs", ".test.ts", ".test.tsx", ".spec.ts", "/tests/"],
            pattern: None,
            severity: Severity::Error,
            check_fn: Some(check_empty_test),
        },
        LintRule {
            id: "TST002",
            name: "todo-test",
            description: "Test marked with TODO/SKIP/IGNORE",
            applies_to: &["_test.rs", ".test.ts", ".test.tsx", ".spec.ts", "/tests/"],
            pattern: Some(
                r"(?i)(#\[ignore\]|\.skip\(|\.todo\(|test\.skip|it\.skip|describe\.skip|@skip|@todo|@ignore)",
            ),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "TST003",
            name: "hardcoded-timeout",
            description: "Test uses hardcoded sleep/timeout",
            applies_to: &["_test.rs", ".test.ts", ".test.tsx", ".spec.ts", "/tests/"],
            pattern: Some(
                r"(?i)(tokio::time::sleep|std::thread::sleep|setTimeout|sleep\(|\.timeout\(\d+\))",
            ),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "TST004",
            name: "flaky-test-pattern",
            description: "Test relies on timing or external state that may cause flakiness",
            applies_to: &["_test.rs", ".test.ts", ".test.tsx", ".spec.ts", "/tests/"],
            pattern: Some(
                r"(?i)(Date\.now\(\)|new Date\(\)|Instant::now|SystemTime::now|random\(|rand::|Math\.random)",
            ),
            severity: Severity::Warning,
            check_fn: None,
        },
        LintRule {
            id: "TST005",
            name: "missing-error-test",
            description: "Function has error path but no error test",
            applies_to: &["_test.rs", ".test.ts", ".test.tsx", ".spec.ts", "/tests/"],
            pattern: None,
            severity: Severity::Warning,
            check_fn: None, // Requires cross-file analysis
        },
    ]
}

// =============================================================================
// Test Coverage Rules (TST100+)
// =============================================================================

fn coverage_rules() -> Vec<LintRule> {
    vec![
        LintRule {
            id: "TST100",
            name: "missing-test-file",
            description: "New source file has no corresponding test file",
            applies_to: &[".rs", ".ts", ".tsx"],
            pattern: None,
            severity: Severity::Error,
            check_fn: None, // Requires cross-file analysis - handled in main lint loop
        },
        LintRule {
            id: "TST101",
            name: "stale-test",
            description: "Modified function but test file unchanged",
            applies_to: &[".rs", ".ts", ".tsx"],
            pattern: None,
            severity: Severity::Warning,
            check_fn: None, // Requires cross-file analysis - handled in main lint loop
        },
        LintRule {
            id: "TST102",
            name: "new-endpoint-no-test",
            description: "New API endpoint without integration test",
            applies_to: &["/routes/", "/api/", "handler"],
            pattern: None,
            severity: Severity::Error,
            check_fn: None, // Requires cross-file analysis
        },
        LintRule {
            id: "TST103",
            name: "untested-error-path",
            description: "New error handling without test coverage",
            applies_to: &[".rs", ".ts", ".tsx"],
            pattern: None,
            severity: Severity::Warning,
            check_fn: None, // Requires cross-file analysis
        },
    ]
}

// =============================================================================
// Custom Check Functions
// =============================================================================

/// Check for empty test functions (TST001)
fn check_empty_test(path: &str, content: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Rust test detection
    if path.ends_with(".rs") {
        let test_fn_re =
            Regex::new(r"#\[(?:tokio::)?test\]\s*(?:\n\s*)*(?:async\s+)?fn\s+(\w+)").unwrap();
        let assert_re = Regex::new(r"(assert|panic!|expect|unwrap\(\)|should_panic)").unwrap();

        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("#[test]") || line.contains("#[tokio::test]") {
                // Find the function body
                let mut brace_count = 0;
                let mut fn_start = i;
                let mut fn_end = i;
                let mut in_fn = false;
                let mut has_assert = false;

                for (j, scan_line) in lines.iter().enumerate().skip(i) {
                    if scan_line.contains("fn ") && !in_fn {
                        fn_start = j;
                    }
                    for c in scan_line.chars() {
                        if c == '{' {
                            brace_count += 1;
                            in_fn = true;
                        } else if c == '}' {
                            brace_count -= 1;
                            if brace_count == 0 && in_fn {
                                fn_end = j;
                                break;
                            }
                        }
                    }
                    if in_fn && assert_re.is_match(scan_line) {
                        has_assert = true;
                    }
                    if brace_count == 0 && in_fn {
                        break;
                    }
                }

                // Check if the function body has any assertions
                if !has_assert && fn_end > fn_start {
                    // Extract function name
                    let fn_body = lines[fn_start..=fn_end.min(lines.len() - 1)].join("\n");
                    let fn_name = test_fn_re
                        .captures(&fn_body)
                        .and_then(|c| c.get(1))
                        .map(|m| m.as_str())
                        .unwrap_or("unknown");

                    violations.push(Violation {
                        rule_id: "TST001",
                        rule_name: "empty-test",
                        line: fn_start + 1,
                        matched_text: format!("fn {}", fn_name),
                        severity: Severity::Error,
                        message: format!("Test function '{}' has no assertions", fn_name),
                    });
                }
            }
        }
    }

    // TypeScript/JavaScript test detection
    if path.ends_with(".ts") || path.ends_with(".tsx") || path.ends_with(".js") {
        let test_fn_re = Regex::new(r#"(?:it|test)\s*\(\s*["']([^"']+)["']"#).unwrap();
        let assert_re =
            Regex::new(r"(expect\(|assert|toBe|toEqual|toMatch|toThrow|rejects|resolves)").unwrap();

        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = test_fn_re.captures(line) {
                let test_name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");

                // Find the test body
                let mut brace_count = 0;
                let mut in_test = false;
                let mut has_assert = false;

                for scan_line in lines.iter().skip(i) {
                    for c in scan_line.chars() {
                        if c == '(' {
                            // Skip paren counting for simplicity
                        } else if c == '{' {
                            brace_count += 1;
                            in_test = true;
                        } else if c == '}' {
                            brace_count -= 1;
                        }
                    }
                    if in_test && assert_re.is_match(scan_line) {
                        has_assert = true;
                    }
                    if brace_count == 0 && in_test {
                        break;
                    }
                }

                if !has_assert {
                    violations.push(Violation {
                        rule_id: "TST001",
                        rule_name: "empty-test",
                        line: i + 1,
                        matched_text: format!("test('{}')", test_name),
                        severity: Severity::Error,
                        message: format!("Test '{}' has no assertions", test_name),
                    });
                }
            }
        }
    }

    violations
}

// =============================================================================
// Test Coverage Helpers (for cross-file analysis)
// =============================================================================

/// Check if a file is a test file
pub fn is_test_file(path: &str) -> bool {
    path.contains("_test.rs")
        || path.contains(".test.ts")
        || path.contains(".test.tsx")
        || path.contains(".spec.ts")
        || path.contains(".spec.tsx")
        || path.contains("/tests/")
        || path.contains("/__tests__/")
        || path.contains("/test/")
}

/// Check if a file is a source file (not a test)
pub fn is_source_file(path: &str) -> bool {
    let extensions = [".rs", ".ts", ".tsx", ".js", ".jsx"];
    extensions.iter().any(|ext| path.ends_with(ext)) && !is_test_file(path)
}

/// Check if a file has an inline test module (Rust #[cfg(test)])
pub fn has_inline_test_module(content: &str) -> bool {
    content.contains("#[cfg(test)]")
}

/// Find the test file for a source file
pub fn find_test_file_for(
    repo_root: &Path,
    source_path: &str,
    patterns: &[TestPattern],
) -> Option<String> {
    let path = Path::new(source_path);
    let dir = path.parent()?.to_string_lossy();
    let stem = path.file_stem()?.to_str()?;
    let ext = path.extension()?.to_str()?;

    // Try discovered patterns first
    for pattern in patterns {
        let test_path = pattern
            .test_pattern
            .replace("{dir}", &dir)
            .replace("{name}", stem);

        let full_path = repo_root.join(&test_path);
        if full_path.exists() {
            return Some(test_path);
        }
    }

    // Fall back to common conventions
    let candidates = match ext {
        "rs" => vec![
            format!("{}/_test.rs", dir).replace("//", "/"),
            format!("{}/{}_test.rs", dir, stem),
            format!("tests/{}.rs", stem),
            format!("{}/tests/{}.rs", dir, stem),
        ],
        "ts" | "tsx" => vec![
            format!("{}/{}.test.{}", dir, stem, ext),
            format!("{}/__tests__/{}.test.{}", dir, stem, ext),
            format!("{}/{}.spec.{}", dir, stem, ext),
        ],
        _ => vec![],
    };

    for candidate in candidates {
        let full_path = repo_root.join(&candidate);
        if full_path.exists() {
            return Some(candidate);
        }
    }

    None
}

/// Guess what the test file path should be
pub fn guess_test_file_path(source_path: &str, patterns: &[TestPattern]) -> String {
    let path = Path::new(source_path);
    let dir = path
        .parent()
        .map(|p| p.to_string_lossy())
        .unwrap_or_default();
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Use most common pattern if available
    if let Some(pattern) = patterns.first() {
        return pattern
            .test_pattern
            .replace("{dir}", &dir)
            .replace("{name}", stem);
    }

    // Default conventions
    match ext {
        "rs" => format!("{}/{}_test.rs", dir, stem),
        "ts" | "tsx" => format!("{}/{}.test.{}", dir, stem, ext),
        _ => format!("{}/{}_test.{}", dir, stem, ext),
    }
}

/// Extract functions from a file
pub fn extract_functions(content: &str, path: &str) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();

    let fn_re = if path.ends_with(".rs") {
        Regex::new(r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap()
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)|^\s*(?:export\s+)?const\s+(\w+)\s*=\s*(?:async\s+)?\(").unwrap()
    } else {
        return functions;
    };

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = fn_re.captures(line) {
            let name = caps
                .get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");

            if !name.is_empty() && !name.starts_with("test_") {
                functions.push(FunctionInfo {
                    name: name.to_string(),
                    line: line_num + 1,
                    signature: line.trim().to_string(),
                });
            }
        }
    }

    functions
}

/// Information about a function in a file
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub line: usize,
    pub signature: String,
}

// =============================================================================
// Release Pre-flight Check
// =============================================================================

/// Check test coverage for changed source files (used in release preflight)
///
/// Returns Ok(true) if all changed source files have adequate test coverage,
/// Ok(false) if issues were found, Err on failure.
pub fn check_test_coverage(ctx: &AppContext, changed_files: &[String]) -> Result<bool> {
    if changed_files.is_empty() {
        return Ok(true);
    }

    // Separate source files from test files
    let (source_files, test_files): (Vec<_>, Vec<_>) = changed_files
        .iter()
        .partition(|f| is_source_file(f) && !is_test_file(f));

    let test_files_set: HashSet<&str> = test_files.iter().map(|s| s.as_str()).collect();

    // Filter to only non-excluded source files that require coverage
    let source_files: Vec<_> = source_files
        .into_iter()
        .filter(|f| !should_exclude_from_coverage(f) && ctx.config.requires_coverage(f))
        .collect();

    if source_files.is_empty() {
        println!(
            "  {} No source files requiring test coverage",
            style("~").dim()
        );
        return Ok(true);
    }

    let mut all_safe = true;
    let mut missing_tests = 0;
    let mut stale_tests = 0;

    for file_path in &source_files {
        let full_path = ctx.repo.join(file_path);
        if !full_path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Parse ignore directives
        let (ignored_rules, skip_file) = parse_ignored_rules(&content, "ai-test-ignore");
        if skip_file {
            continue;
        }

        // Check for inline tests
        let has_inline_tests = has_inline_test_module(&content);

        // Find corresponding test file
        let test_file = find_test_file_for(&ctx.repo, file_path, &[]);
        let test_file_in_diff = test_file
            .as_ref()
            .map(|tf| test_files_set.iter().any(|f| f.contains(tf)))
            .unwrap_or(false);

        // Extract functions from the file
        let functions = extract_functions(&content, file_path);

        if functions.is_empty() {
            continue;
        }

        // TST100: Missing test file
        if test_file.is_none() && !has_inline_tests && !ignored_rules.contains("TST100") {
            missing_tests += 1;
            all_safe = false;
            let filename = Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file_path);
            println!("  {} [{}]: No test file found", style("✗").red(), filename);
        }

        // TST101: Stale test (source changed but test not updated)
        if test_file.is_some()
            && !test_file_in_diff
            && !has_inline_tests
            && !ignored_rules.contains("TST101")
        {
            stale_tests += 1;
            let filename = Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file_path);
            println!(
                "  {} [{}]: Source changed but test file not updated",
                style("⚠").yellow(),
                filename
            );
        }
    }

    if all_safe && stale_tests == 0 {
        println!("  {} Test coverage check passed", style("✓").green());
        Ok(true)
    } else if all_safe {
        println!(
            "  {} Test coverage: {} stale test warning(s)",
            style("⚠").yellow(),
            stale_tests
        );
        Ok(true) // Warnings don't fail the check
    } else {
        println!(
            "  {} Test coverage: {} missing test file(s), {} stale test(s)",
            style("✗").red(),
            missing_tests,
            stale_tests
        );
        Ok(false)
    }
}

/// Check if a file should be excluded from test coverage checks
fn should_exclude_from_coverage(path: &str) -> bool {
    const EXCLUDE_PATTERNS: &[&str] = &[
        "*.md",
        "*.json",
        "*.toml",
        "*.yaml",
        "*.yml",
        "*.lock",
        "*.svg",
        "*.png",
        "**/node_modules/**",
        "**/target/**",
        "**/dist/**",
        "**/build/**",
        "**/.git/**",
        "**/migrations/**",
        "**/generated/**",
        "**/*.generated.*",
        "**/*.d.ts",
        "**/mod.rs",
        "**/lib.rs",
        "**/main.rs",
    ];

    for pattern in EXCLUDE_PATTERNS {
        if pattern.contains("**") {
            let pattern_parts: Vec<&str> = pattern.split("**").collect();
            if pattern_parts.len() == 2 {
                let (prefix, suffix) = (pattern_parts[0], pattern_parts[1]);
                if (prefix.is_empty() || path.contains(prefix.trim_end_matches('/')))
                    && (suffix.is_empty() || path.contains(suffix.trim_start_matches('/')))
                {
                    return true;
                }
            }
        } else if pattern.starts_with("*.") {
            let ext = pattern.trim_start_matches('*');
            if path.ends_with(ext) {
                return true;
            }
        } else if path.contains(pattern) {
            return true;
        }
    }
    false
}

// =============================================================================
// AI-Powered Test Coverage Analysis
// =============================================================================

/// Check test coverage using AI analysis of the diff
///
/// Analyzes code changes to identify business rules that need test coverage
/// before merging to main.
pub fn check_test_coverage_ai(
    ctx: &AppContext,
    changed_files: &[String],
    base_branch: &str,
) -> Result<bool> {
    use super::super::engine::parse_ignored_rules;
    use super::{call_ai, get_api_key};

    // Filter to source files that require coverage
    let source_files: Vec<_> = changed_files
        .iter()
        .filter(|f| {
            ctx.config.requires_coverage(f)
                && is_source_file(f)
                && !is_test_file(f)
                && !should_exclude_from_coverage(f)
        })
        .collect();

    // Build list of files with @ai-test-ignore directive
    let ignored_files: Vec<String> = source_files
        .iter()
        .filter_map(|f| {
            let full_path = ctx.repo.join(f);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let (ignored_rules, skip_file) = parse_ignored_rules(&content, "ai-test-ignore");
                if skip_file || ignored_rules.contains("TST100") {
                    return Some(f.to_string());
                }
            }
            None
        })
        .collect();

    if source_files.is_empty() {
        println!("  {} No source files to check", style("~").dim());
        return Ok(true);
    }

    let api_key = match get_api_key() {
        Some(key) => key,
        None => {
            println!(
                "  {} Skipped (no OPENAI_API_KEY or ANTHROPIC_API_KEY)",
                style("~").yellow()
            );
            return Ok(true);
        }
    };

    println!(
        "  {} Analyzing {} files with AI...",
        style("◌").cyan(),
        source_files.len()
    );

    // Build path filter from coverage_packages config
    let coverage_paths: Vec<String> = ctx
        .config
        .global
        .test
        .coverage_packages
        .iter()
        .map(|pkg| format!("packages/{}/src", pkg))
        .collect();

    if coverage_paths.is_empty() {
        println!("  {} No coverage_packages configured", style("~").dim());
        return Ok(true);
    }

    // Get diff content (limit to avoid token overflow)
    let mut stat_cmd = Command::new("git");
    stat_cmd.args(["diff", base_branch, "--stat", "--"]);
    for path in &coverage_paths {
        stat_cmd.arg(path);
    }
    let diff_output = stat_cmd.current_dir(&ctx.repo).output()?;
    let diff_stat = String::from_utf8_lossy(&diff_output.stdout);

    // Get actual diff but truncate if too large
    let mut diff_cmd = Command::new("git");
    diff_cmd.args(["diff", base_branch, "--"]);
    for path in &coverage_paths {
        diff_cmd.arg(path);
    }
    let diff_output = diff_cmd.current_dir(&ctx.repo).output()?;

    let full_diff = String::from_utf8_lossy(&diff_output.stdout);

    // Truncate diff if too large (keep first ~50KB for API limits)
    const MAX_DIFF_SIZE: usize = 50_000;
    let diff = if full_diff.len() > MAX_DIFF_SIZE {
        println!(
            "    {} Large diff truncated for analysis",
            style("!").yellow()
        );
        format!(
            "{}\n\n[... truncated, {} total bytes ...]",
            &full_diff[..MAX_DIFF_SIZE],
            full_diff.len()
        )
    } else {
        full_diff.to_string()
    };

    if diff.is_empty() {
        println!("  {} No diff to analyze", style("~").dim());
        return Ok(true);
    }

    // Get list of test files for context
    let test_output = Command::new("find")
        .args(["packages/api-core/tests", "-name", "*_tests.rs"])
        .current_dir(&ctx.repo)
        .output()?;
    let _test_files = String::from_utf8_lossy(&test_output.stdout);

    let system = r#"You identify business logic needing test coverage. Focus on actions, validations, permissions, and error handling. Ignore boilerplate (getters, type defs, trait impls)."#;

    // Build ignore list message
    let ignore_msg = if ignored_files.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nIGNORED FILES (have @ai-test-ignore directive - do NOT report these):\n{}",
            ignored_files.join("\n")
        )
    };

    let prompt = format!(
        r#"Review this diff for untested business logic.

FILES: {}{}

DIFF:
{}

If covered, respond: COVERED

If gaps exist, respond with:
GAPS
For each gap: file path, function name, line number, what it does, and specific test scenarios needed."#,
        diff_stat, ignore_msg, diff
    );

    let response = call_ai(&api_key, system, &prompt)?;
    let is_covered = response.trim().starts_with("COVERED");

    if is_covered {
        println!("  {} Test coverage is adequate", style("✓").green());
    } else {
        println!("  {} Coverage gaps found:\n", style("✗").red());
        // Print the structured response with formatting
        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() || line == "GAPS" {
                continue;
            }
            if line.starts_with("## ") {
                // File header
                println!("  {}", style(line).cyan().bold());
            } else if line.starts_with("### ") {
                // Function header
                println!("    {}", style(line).yellow());
            } else if line.starts_with("- **") || line.starts_with("  ") {
                // Details
                println!("      {}", line);
            } else {
                println!("    {}", line);
            }
        }
        println!();
    }

    Ok(is_covered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::ai_lint::engine::check_file;

    #[test]
    fn test_empty_rust_test() {
        let content = r#"
#[test]
fn test_something() {
    let x = 1;
}
"#;
        let rules = get_test_coverage_rules();
        let violations = check_file("foo_test.rs", content, &rules, &HashSet::new());
        assert!(violations.iter().any(|v| v.rule_id == "TST001"));
    }

    #[test]
    fn test_rust_test_with_assert_ok() {
        let content = r#"
#[test]
fn test_something() {
    let x = 1;
    assert_eq!(x, 1);
}
"#;
        let rules = get_test_coverage_rules();
        let violations = check_file("foo_test.rs", content, &rules, &HashSet::new());
        assert!(!violations.iter().any(|v| v.rule_id == "TST001"));
    }

    #[test]
    fn test_todo_test_detection() {
        let content = "it.skip('pending test', () => {});";
        let rules = get_test_coverage_rules();
        let violations = check_file("foo.test.ts", content, &rules, &HashSet::new());
        assert!(violations.iter().any(|v| v.rule_id == "TST002"));
    }
}
