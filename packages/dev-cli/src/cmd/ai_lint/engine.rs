//! Shared types and functions for AI linters
//!
//! Provides common abstractions used by all linters: rules, violations,
//! ignore directive parsing, and output formatting.

use regex::Regex;
use std::collections::HashSet;

/// Severity level for lint rules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A lint rule with pattern-based detection
#[derive(Debug, Clone)]
pub struct LintRule {
    /// Unique rule ID (e.g., "WF001", "TST001")
    pub id: &'static str,
    /// Short name for the rule
    pub name: &'static str,
    /// Detailed description of what the rule checks
    pub description: &'static str,
    /// File patterns this rule applies to (glob-style)
    pub applies_to: &'static [&'static str],
    /// Regex pattern to detect violations (None = always check via custom logic)
    pub pattern: Option<&'static str>,
    /// Severity level
    pub severity: Severity,
    /// Custom check function for complex rules
    pub check_fn: Option<fn(&str, &str) -> Vec<Violation>>,
}

/// A detected lint violation
#[derive(Debug, Clone)]
pub struct Violation {
    /// Rule that was violated
    pub rule_id: &'static str,
    /// Rule name
    #[allow(dead_code)]
    pub rule_name: &'static str,
    /// Line number (1-indexed, 0 if not applicable)
    pub line: usize,
    /// The matching text that triggered the violation
    #[allow(dead_code)]
    pub matched_text: String,
    /// Severity
    pub severity: Severity,
    /// Detailed message
    pub message: String,
}

/// Check a file against a set of rules
pub fn check_file(
    path: &str,
    content: &str,
    rules: &[LintRule],
    ignored_rules: &HashSet<String>,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    for rule in rules {
        // Skip if rule is ignored
        if ignored_rules.contains(rule.id) {
            continue;
        }

        // Check if rule applies to this file
        let applies = rule.applies_to.iter().any(|pattern| path.contains(pattern));
        if !applies {
            continue;
        }

        // Check using custom function if provided
        if let Some(check_fn) = rule.check_fn {
            let rule_violations = check_fn(path, content);
            for v in rule_violations {
                if !ignored_rules.contains(v.rule_id) {
                    violations.push(v);
                }
            }
            continue;
        }

        // Check using regex pattern
        if let Some(pattern_str) = rule.pattern {
            if let Ok(re) = Regex::new(pattern_str) {
                for (line_num, line) in content.lines().enumerate() {
                    if let Some(m) = re.find(line) {
                        violations.push(Violation {
                            rule_id: rule.id,
                            rule_name: rule.name,
                            line: line_num + 1,
                            matched_text: m.as_str().to_string(),
                            severity: rule.severity,
                            message: rule.description.to_string(),
                        });
                    }
                }
            }
        }
    }

    violations
}

/// Parse ignored rule IDs from file content
///
/// Supports two ignore directive prefixes:
/// - `@ai-lint-ignore` for security rules
/// - `@ai-test-ignore` for test coverage rules
///
/// Format: `@{prefix} RULE_ID` or `@{prefix} RULE_ID1,RULE_ID2`
/// Also supports: `@{prefix}-file` (ignores all rules for file)
pub fn parse_ignored_rules(content: &str, prefix: &str) -> (HashSet<String>, bool) {
    let mut ignored = HashSet::new();
    let mut ignore_file = false;

    let ignore_pattern = format!(r"@{}(?:-file)?\s+([A-Z]+\d+(?:\s*,\s*[A-Z]+\d+)*)", prefix);
    let ignore_file_pattern = format!(r"@{}-file\b", prefix);

    let ignore_re = Regex::new(&ignore_pattern).unwrap();
    let ignore_file_re = Regex::new(&ignore_file_pattern).unwrap();

    for line in content.lines() {
        // Check for file-level ignore
        if ignore_file_re.is_match(line) {
            // If there are rule IDs after it, just ignore those rules file-wide
            // If no rule IDs, ignore entire file
            if let Some(caps) = ignore_re.captures(line) {
                let rules = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                for rule in rules.split(',') {
                    ignored.insert(rule.trim().to_string());
                }
            } else {
                ignore_file = true;
            }
            continue;
        }

        // Check for line-level ignore
        if let Some(caps) = ignore_re.captures(line) {
            let rules = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            for rule in rules.split(',') {
                ignored.insert(rule.trim().to_string());
            }
        }
    }

    (ignored, ignore_file)
}

/// Format rules as help text
pub fn format_rules_help(
    rules: &[LintRule],
    categories: &[(&str, &str)],
    ignore_prefix: &str,
) -> String {
    let mut output = String::new();

    output.push_str("Available lint rules:\n\n");

    for (prefix, title) in categories {
        let category_rules: Vec<_> = rules.iter().filter(|r| r.id.starts_with(prefix)).collect();
        if category_rules.is_empty() {
            continue;
        }

        output.push_str(&format!("## {}\n", title));
        for rule in category_rules {
            let severity = match rule.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            output.push_str(&format!(
                "  {} ({}) - {}\n",
                rule.id, severity, rule.description
            ));
        }
        output.push('\n');
    }

    output.push_str("To ignore a rule, add a comment before the line:\n");
    output.push_str(&format!("  // @{} RULE_ID\n", ignore_prefix));
    output.push_str(&format!("  # @{} RULE_ID1,RULE_ID2\n", ignore_prefix));

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ignored_rules() {
        let content = "// @ai-lint-ignore WF001\nsome code\n# @ai-lint-ignore WF002,WF003";
        let (ignored, skip) = parse_ignored_rules(content, "ai-lint-ignore");
        assert!(!skip);
        assert!(ignored.contains("WF001"));
        assert!(ignored.contains("WF002"));
        assert!(ignored.contains("WF003"));
    }

    #[test]
    fn test_parse_ignore_file() {
        let content = "// @ai-lint-ignore-file\nsome code";
        let (_, skip) = parse_ignored_rules(content, "ai-lint-ignore");
        assert!(skip);
    }

    #[test]
    fn test_check_file_with_pattern() {
        let rules = vec![LintRule {
            id: "TEST001",
            name: "test-rule",
            description: "Test rule",
            applies_to: &[".rs"],
            pattern: Some(r"TODO"),
            severity: Severity::Warning,
            check_fn: None,
        }];

        let content = "fn main() {\n    // TODO: fix this\n}";
        let violations = check_file("test.rs", content, &rules, &HashSet::new());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_id, "TEST001");
        assert_eq!(violations[0].line, 2);
    }
}
