//! AI task/preset system
//!
//! Loads AI tasks from markdown files with YAML frontmatter.
//! Tasks are discovered via glob pattern from config.

use anyhow::{anyhow, Result};
use dialoguer::Select;
use glob::glob;
use serde::Deserialize;
use std::path::Path;

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;

/// Task frontmatter parsed from markdown files
#[derive(Deserialize, Default)]
struct TaskFrontmatter {
    name: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    category: Option<String>,
}

/// A loaded AI task
pub struct Task {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub category: String,
    pub content: String,
}

/// Load all tasks from the configured glob pattern
pub fn load_tasks(ctx: &AppContext) -> Result<Vec<Task>> {
    let pattern = &ctx.config.global.ai.tasks;
    let full_pattern = ctx.repo.join(pattern);
    let pattern_str = full_pattern.to_string_lossy();

    let mut tasks = Vec::new();
    for entry in glob(&pattern_str)? {
        let path = entry?;
        if let Some(task) = load_task_file(&path)? {
            tasks.push(task);
        }
    }

    tasks.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tasks)
}

/// Load a single task from a markdown file
fn load_task_file(path: &Path) -> Result<Option<Task>> {
    let content = std::fs::read_to_string(path)?;
    let (frontmatter, body) = parse_frontmatter(&content);

    let id = path.file_stem().unwrap().to_string_lossy().to_string();

    // Name from frontmatter or title-case from filename
    let name = frontmatter.name.unwrap_or_else(|| {
        id.replace('-', " ")
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().chain(c).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    });

    Ok(Some(Task {
        id,
        name,
        aliases: frontmatter.aliases,
        category: frontmatter.category.unwrap_or_else(|| "Other".to_string()),
        content: body,
    }))
}

/// Parse YAML frontmatter from markdown content
fn parse_frontmatter(content: &str) -> (TaskFrontmatter, String) {
    if !content.starts_with("---") {
        return (TaskFrontmatter::default(), content.to_string());
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return (TaskFrontmatter::default(), content.to_string());
    }

    let yaml = parts[1].trim();
    let body = parts[2].trim().to_string();

    match serde_yaml::from_str(yaml) {
        Ok(fm) => (fm, body),
        Err(_) => (TaskFrontmatter::default(), content.to_string()),
    }
}

/// List all available AI commands (built-in + tasks)
pub fn list_all_commands(ctx: &AppContext) -> Result<()> {
    println!("Available AI commands:\n");

    // Built-in commands
    println!("  Built-in:");
    println!(
        "    {:<20} Lint code (security, test coverage, migrations)",
        "lint [CATEGORY]"
    );
    println!(
        "    {:<20} Run command and fix errors with Claude",
        "fix [STEP]"
    );
    println!();

    // Lint categories
    println!("  Lint categories:");
    println!("    {:<20} Test coverage (TST*)", "tc, test-coverage");
    println!(
        "    {:<20} Security (WF*, INF*, DKR*, AUTH*, ENV*)",
        "sec, security"
    );
    println!("    {:<20} Migration safety (AI-powered)", "migrations");
    println!("    {:<20} All categories (default)", "all");
    println!();

    // Fix steps
    println!("  Fix steps:");
    println!("    {:<20} Run formatters and fix", "fmt");
    println!("    {:<20} Run tests and fix failures", "test");
    println!("    {:<20} Build docker and fix errors", "docker [SERVICE]");
    println!("    {:<20} Run all steps (default)", "all");
    println!();

    // Load preset tasks
    let tasks = load_tasks(ctx)?;
    if !tasks.is_empty() {
        // Group by category
        let mut categories: std::collections::HashMap<&str, Vec<&Task>> =
            std::collections::HashMap::new();
        for task in &tasks {
            categories
                .entry(task.category.as_str())
                .or_default()
                .push(task);
        }

        println!("  Preset tasks:");

        // Print in order
        for cat in CATEGORY_ORDER {
            if let Some(cat_tasks) = categories.get(cat) {
                for task in cat_tasks {
                    let aliases = if task.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", task.aliases.join(", "))
                    };
                    println!("    {:<20} {}{}", task.id, task.name, aliases);
                }
            }
        }

        // Print uncategorized
        for (cat, cat_tasks) in &categories {
            if !CATEGORY_ORDER.contains(cat) {
                for task in cat_tasks {
                    let aliases = if task.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", task.aliases.join(", "))
                    };
                    println!("    {:<20} {}{}", task.id, task.name, aliases);
                }
            }
        }
        println!();
    }

    println!("Examples:");
    println!("  ./dev.sh ai lint tc --fix");
    println!("  ./dev.sh ai fix test");
    println!("  ./dev.sh ai security-audit");

    Ok(())
}

/// Interactive AI menu
pub fn ai_menu(ctx: &AppContext) -> Result<()> {
    let items = vec![
        "Lint (security, test coverage, migrations)",
        "Fix (run command and fix errors)",
        "Browse preset tasks...",
    ];

    let selection = Select::with_theme(&ctx.theme())
        .with_prompt("AI Assistant")
        .items(&items)
        .default(0)
        .interact()?;

    match selection {
        0 => lint_menu(ctx),
        1 => fix_menu(ctx),
        2 => task_menu(ctx),
        _ => Ok(()),
    }
}

/// Interactive lint menu
fn lint_menu(ctx: &AppContext) -> Result<()> {
    let items = vec![
        "All (run all linters)",
        "Test Coverage (TST*)",
        "Security (WF*, INF*, DKR*, AUTH*, ENV*)",
        "Migrations (AI-powered)",
    ];

    let selection = Select::with_theme(&ctx.theme())
        .with_prompt("Lint category")
        .items(&items)
        .default(0)
        .interact()?;

    let category = match selection {
        0 => None,
        1 => Some("tc"),
        2 => Some("sec"),
        3 => Some("migrations"),
        _ => None,
    };

    // Ask about --fix
    let fix = ctx.confirm("Fix issues with Claude?", false)?;

    super::ai_lint::ai_lint(ctx, category, fix, false, Some("main"), None)
}

/// Interactive fix menu
fn fix_menu(ctx: &AppContext) -> Result<()> {
    let items = vec![
        "All (fmt, lint, test)",
        "Format only",
        "Test only",
        "Docker",
    ];

    let selection = Select::with_theme(&ctx.theme())
        .with_prompt("What to fix")
        .items(&items)
        .default(0)
        .interact()?;

    let command: Vec<String> = match selection {
        0 => vec![],
        1 => vec!["fmt".to_string()],
        2 => vec!["test".to_string()],
        3 => vec!["docker".to_string()],
        _ => vec![],
    };

    super::ai::ai_fix(ctx, &command)
}

/// List available tasks (grouped by category)
#[allow(dead_code)]
pub fn list_tasks(ctx: &AppContext) -> Result<()> {
    list_all_commands(ctx)
}

/// Category display order
const CATEGORY_ORDER: &[&str] = &[
    "Quick",
    "Code Review",
    "Comprehensive",
    "Architecture",
    "Ops",
    "Other",
];

/// Interactive task selection menu (grouped by category)
pub fn task_menu(ctx: &AppContext) -> Result<()> {
    let tasks = load_tasks(ctx)?;
    if tasks.is_empty() {
        println!("No AI tasks found. Add task files to docs/ai/tasks/");
        return Ok(());
    }

    // Group tasks by category
    let mut categories: std::collections::HashMap<&str, Vec<&Task>> =
        std::collections::HashMap::new();
    for task in &tasks {
        categories
            .entry(task.category.as_str())
            .or_default()
            .push(task);
    }

    // Build ordered category list
    let mut ordered_categories: Vec<&str> = Vec::new();
    for cat in CATEGORY_ORDER {
        if categories.contains_key(cat) {
            ordered_categories.push(cat);
        }
    }
    // Add any categories not in CATEGORY_ORDER
    for cat in categories.keys() {
        if !ordered_categories.contains(cat) {
            ordered_categories.push(cat);
        }
    }

    // First select category
    let selection = Select::with_theme(&ctx.theme())
        .with_prompt("Select category")
        .items(&ordered_categories)
        .default(0)
        .interact()?;

    let selected_category = ordered_categories[selection];
    let category_tasks = &categories[selected_category];

    // Then select task within category
    let task_names: Vec<&str> = category_tasks.iter().map(|t| t.name.as_str()).collect();
    let task_selection = Select::with_theme(&ctx.theme())
        .with_prompt(format!("{} tasks", selected_category))
        .items(&task_names)
        .default(0)
        .interact()?;

    run_task(ctx, category_tasks[task_selection])
}

/// Run a task by ID or alias (supports prefix matching)
pub fn run_task_by_id(ctx: &AppContext, task_id: &str) -> Result<()> {
    let tasks = load_tasks(ctx)?;

    let task = tasks
        .iter()
        .find(|t| {
            t.id == task_id
                || t.aliases.contains(&task_id.to_string())
                || t.id.starts_with(task_id)
                || t.aliases.iter().any(|a| a.starts_with(task_id))
        })
        .ok_or_else(|| {
            anyhow!(
                "Unknown task: {}. Run './dev.sh ai task --list' to see available.",
                task_id
            )
        })?;

    run_task(ctx, task)
}

/// Run a task
pub fn run_task(ctx: &AppContext, task: &Task) -> Result<()> {
    ctx.print_header(&format!("AI: {}", task.name));
    println!();

    let code = CmdBuilder::new("claude")
        .arg(&task.content)
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;

    if code != 0 {
        return Err(anyhow!("AI task failed"));
    }
    Ok(())
}
