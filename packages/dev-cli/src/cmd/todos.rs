//! Interactive todo management via GitHub Issues + Milestones
//!
//! Milestone-centric workflow:
//! 1. View milestones (sprints/goals)
//! 2. Select issues to work on
//! 3. Start work session with AI

use anyhow::{Context, Result};
use console::style;
use dialoguer::{FuzzySelect, Input, MultiSelect, Select};
use serde::Deserialize;

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use crate::utils::cmd_exists;

// =============================================================================
// Data Structures
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
struct GitHubIssue {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    labels: Vec<GitHubLabel>,
    assignees: Vec<GitHubUser>,
    milestone: Option<IssueMilestone>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubLabel {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Debug, Clone, Deserialize)]
struct IssueMilestone {
    title: String,
    number: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct Milestone {
    title: String,
    number: u64,
    state: String,
    description: Option<String>,
    due_on: Option<String>,
    open_issues: u64,
    closed_issues: u64,
}

/// Issue status based on state and labels
#[derive(Debug, Clone, Copy, PartialEq)]
enum IssueStatus {
    Todo,
    InProgress,
    Done,
}

impl IssueStatus {
    fn from_issue(issue: &GitHubIssue) -> Self {
        if issue.state == "closed" {
            IssueStatus::Done
        } else if issue.labels.iter().any(|l| {
            let name = l.name.to_lowercase();
            name == "in-progress" || name == "in progress" || name == "wip"
        }) {
            IssueStatus::InProgress
        } else {
            IssueStatus::Todo
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            IssueStatus::Todo => "○",
            IssueStatus::InProgress => "◐",
            IssueStatus::Done => "●",
        }
    }
}

// =============================================================================
// Main Entry Point
// =============================================================================

/// Main entry point for todos command
pub fn todos_menu(ctx: &AppContext) -> Result<()> {
    if !cmd_exists("gh") {
        println!("{}", style("GitHub CLI (gh) not installed.").yellow());
        println!();
        println!("Install: https://cli.github.com/");
        return Ok(());
    }

    // Check if gh is authenticated
    let auth_check = CmdBuilder::new("gh")
        .args(["auth", "status"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    if auth_check.is_err() || auth_check.as_ref().map(|o| o.code != 0).unwrap_or(true) {
        println!(
            "{}",
            style("GitHub CLI not authenticated. Run: gh auth login").yellow()
        );
        return Ok(());
    }

    main_menu(ctx)
}

fn main_menu(ctx: &AppContext) -> Result<()> {
    loop {
        // Fetch milestones
        let milestones = fetch_milestones(ctx, "open")?;
        let backlog_count = fetch_backlog_count(ctx)?;

        ctx.print_header("Todo");
        println!();

        // Build menu: sprints first, then options at bottom
        let mut items: Vec<String> = milestones
            .iter()
            .map(|m| format_milestone_menu_item(m))
            .collect();

        // Add separator if there are milestones
        if !milestones.is_empty() {
            items.push("─────────────────────────────────────────".to_string());
        }

        // Bottom options
        items.push(format!("Backlog ({})", backlog_count));
        items.push("Past sprints".to_string());
        items.push("+ New sprint".to_string());
        items.push("← Back".to_string());

        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("Select")
            .items(&items)
            .default(0)
            .interact()?;

        let milestone_count = milestones.len();
        let separator_offset = if milestones.is_empty() { 0 } else { 1 };

        if choice < milestone_count {
            // Selected a milestone
            view_milestone(ctx, &milestones[choice])?;
        } else if !milestones.is_empty() && choice == milestone_count {
            // Separator - re-show menu
            continue;
        } else if choice == milestone_count + separator_offset {
            // Backlog
            view_backlog(ctx)?;
        } else if choice == milestone_count + separator_offset + 1 {
            // Past milestones
            view_past_milestones(ctx)?;
        } else if choice == milestone_count + separator_offset + 2 {
            // New milestone
            create_milestone(ctx)?;
        } else {
            break;
        }
    }

    Ok(())
}

fn format_milestone_menu_item(m: &Milestone) -> String {
    let total = m.open_issues + m.closed_issues;
    let pct = if total > 0 {
        (m.closed_issues * 100) / total
    } else {
        0
    };

    let status_icon = if pct == 100 {
        "●"
    } else if pct > 50 {
        "◐"
    } else {
        "○"
    };

    // Progress bar
    let bar_width = 10;
    let filled = if total > 0 {
        (pct as usize * bar_width) / 100
    } else {
        0
    };
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled));

    format!(
        "{}   {:<16} {} {:>2}/{:<2} ({}%)",
        status_icon, m.title, bar, m.closed_issues, total, pct,
    )
}

fn print_milestone_row(m: &Milestone) {
    let total = m.open_issues + m.closed_issues;
    let pct = if total > 0 {
        (m.closed_issues * 100) / total
    } else {
        0
    };

    let status_icon = if pct == 100 {
        style("●").green()
    } else if pct > 50 {
        style("◐").yellow()
    } else {
        style("○").dim()
    };

    // Progress bar
    let bar_width = 15;
    let filled = if total > 0 {
        (pct as usize * bar_width) / 100
    } else {
        0
    };
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled));

    // Due date
    let due = m
        .due_on
        .as_ref()
        .and_then(|d| d.split('T').next())
        .map(|d| format!("  due {}", d))
        .unwrap_or_default();

    println!(
        "  {} {:<20} {} {}/{} ({}%){}",
        status_icon,
        style(&m.title).bold(),
        style(bar).cyan(),
        style(m.closed_issues).green(),
        total,
        pct,
        style(due).dim(),
    );
}

// =============================================================================
// Milestone View
// =============================================================================

fn view_milestone(ctx: &AppContext, milestone: &Milestone) -> Result<()> {
    loop {
        let issues = fetch_milestone_issues(ctx, &milestone.title)?;

        // Header
        let total = milestone.open_issues + milestone.closed_issues;
        let pct = if total > 0 {
            (milestone.closed_issues * 100) / total
        } else {
            0
        };

        ctx.print_header(&format!(
            "{} ({}/{})",
            milestone.title, milestone.closed_issues, total
        ));

        if let Some(due) = &milestone.due_on {
            if let Some(date) = due.split('T').next() {
                println!("Due: {}", style(date).yellow());
            }
        }
        println!();

        // Check for completion
        if pct == 100 && milestone.open_issues == 0 && total > 0 {
            println!("{}", style("🎉 All issues complete!").green());
            if ctx.confirm("Close this milestone?", false)? {
                close_milestone(ctx, milestone.number)?;
                return Ok(());
            }
            println!();
        }

        if issues.is_empty() {
            println!("  {}", style("No issues").dim());
            println!();
        } else {
            print_issue_table(&issues);
        }

        // Menu
        let items = vec!["Pick up work (select issues)", "+ Add issue", "← Back"];

        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("Action")
            .items(&items)
            .default(0)
            .interact()?;

        match choice {
            0 => pick_up_work(ctx, milestone, &issues)?,
            1 => quick_add_issue(ctx, Some(&milestone.title))?,
            _ => break,
        }
    }

    Ok(())
}

fn print_issue_table(issues: &[GitHubIssue]) {
    // Header
    println!(
        "  {}",
        style("  #      Status     Title                                    Labels              Assignee").dim()
    );
    println!(
        "  {}",
        style("  ─────  ─────────  ───────────────────────────────────────  ──────────────────  ────────").dim()
    );

    for issue in issues {
        let status = IssueStatus::from_issue(issue);
        let status_display = match status {
            IssueStatus::Todo => style("○ todo  ").dim(),
            IssueStatus::InProgress => style("◐ wip   ").yellow(),
            IssueStatus::Done => style("● done  ").green(),
        };

        // Labels (excluding status labels)
        let labels: Vec<&str> = issue
            .labels
            .iter()
            .filter(|l| {
                let name = l.name.to_lowercase();
                name != "in-progress" && name != "in progress" && name != "wip"
            })
            .map(|l| l.name.as_str())
            .collect();
        let labels_display = if labels.is_empty() {
            style("-".to_string()).dim().to_string()
        } else {
            labels
                .iter()
                .map(|l| style(*l).cyan().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };

        // Assignee
        let assignee = issue
            .assignees
            .first()
            .map(|a| format!("@{}", a.login))
            .unwrap_or_else(|| "-".to_string());
        let assignee_display = if assignee == "-" {
            style(assignee).dim().to_string()
        } else {
            style(assignee).magenta().to_string()
        };

        // Title (truncate if needed)
        let title = if issue.title.len() > 38 {
            format!("{}...", &issue.title[..35])
        } else {
            issue.title.clone()
        };

        println!(
            "  {:<6} {} {:<40} {:<18} {}",
            style(format!("#{}", issue.number)).white(),
            status_display,
            title,
            labels_display,
            assignee_display,
        );
    }
    println!();
}

// =============================================================================
// Pick Up Work (Multi-Select + Work Session)
// =============================================================================

fn pick_up_work(ctx: &AppContext, milestone: &Milestone, all_issues: &[GitHubIssue]) -> Result<()> {
    // Get current user
    let me = get_current_user(ctx)?;

    // Filter to actionable issues: unassigned OR assigned to me and in-progress
    // Sort: unassigned first, then my in-progress
    let mut actionable: Vec<&GitHubIssue> = all_issues
        .iter()
        .filter(|i| {
            let status = IssueStatus::from_issue(i);
            if status == IssueStatus::Done {
                return false;
            }
            let assigned_to_me = i.assignees.iter().any(|a| a.login == me);
            let unassigned = i.assignees.is_empty();
            unassigned || (assigned_to_me && status == IssueStatus::InProgress)
        })
        .collect();

    // Sort: unassigned first
    actionable.sort_by_key(|i| !i.assignees.is_empty());

    if actionable.is_empty() {
        println!();
        println!("  {}", style("No actionable issues").dim());
        println!("  All issues are either done or assigned to others.");
        println!();
        return Ok(());
    }

    ctx.print_header(&format!("{} - Pick up work", milestone.title));
    println!();

    // Build selection items
    let items: Vec<String> = actionable
        .iter()
        .map(|i| {
            let status = IssueStatus::from_issue(i);
            let assignee = i
                .assignees
                .first()
                .map(|a| format!("@{}", a.login))
                .unwrap_or_else(|| "-".to_string());

            let labels: String = i
                .labels
                .iter()
                .filter(|l| {
                    let name = l.name.to_lowercase();
                    name != "in-progress" && name != "in progress" && name != "wip"
                })
                .map(|l| l.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            let title = if i.title.len() > 35 {
                format!("{}...", &i.title[..32])
            } else {
                i.title.clone()
            };

            format!(
                "#{:<5} {} {:<38} {:<15} {}",
                i.number,
                status.icon(),
                title,
                if labels.is_empty() {
                    "-".to_string()
                } else {
                    format!("[{}]", labels)
                },
                assignee,
            )
        })
        .collect();

    // Show table header for context
    println!(
        "  {}",
        style(
            "  #      Status  Title                                    Labels           Assignee"
        )
        .dim()
    );
    println!();

    let selected = MultiSelect::with_theme(&ctx.theme())
        .with_prompt("Select issues (Space to toggle, Enter to confirm)")
        .items(&items)
        .interact()?;

    if selected.is_empty() {
        println!("No issues selected.");
        return Ok(());
    }

    let selected_issues: Vec<&GitHubIssue> = selected.iter().map(|&idx| actionable[idx]).collect();

    println!();
    println!(
        "{} issues selected. Starting work session...",
        selected_issues.len()
    );
    println!();

    work_session(ctx, &selected_issues)
}

// =============================================================================
// Work Session
// =============================================================================

fn work_session(ctx: &AppContext, issues: &[&GitHubIssue]) -> Result<()> {
    let total = issues.len();

    for (idx, issue) in issues.iter().enumerate() {
        println!();
        println!(
            "{}",
            style(format!(
                "━━━ {}/{}: #{} {} ━━━",
                idx + 1,
                total,
                issue.number,
                issue.title
            ))
            .cyan()
        );
        println!();

        // Show issue details
        if let Some(body) = &issue.body {
            if !body.is_empty() {
                // Parse and show tasks
                let tasks = parse_tasks(body);
                if !tasks.is_empty() {
                    let done = tasks.iter().filter(|t| t.completed).count();
                    println!("Tasks ({}/{}):", done, tasks.len());
                    for task in &tasks {
                        let icon = if task.completed { "●" } else { "○" };
                        let styled = if task.completed {
                            style(format!("  {} {}", icon, task.text)).dim()
                        } else {
                            style(format!("  {} {}", icon, task.text)).white()
                        };
                        println!("{}", styled);
                    }
                    println!();
                } else {
                    // Show body excerpt
                    let excerpt = if body.len() > 300 {
                        format!("{}...", &body[..300])
                    } else {
                        body.clone()
                    };
                    println!("{}", style(excerpt).dim());
                    println!();
                }
            }
        }

        // Labels
        let labels: Vec<&str> = issue
            .labels
            .iter()
            .filter(|l| {
                let name = l.name.to_lowercase();
                name != "in-progress" && name != "in progress" && name != "wip"
            })
            .map(|l| l.name.as_str())
            .collect();
        if !labels.is_empty() {
            println!(
                "Labels: {}",
                labels
                    .iter()
                    .map(|l| style(*l).cyan().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        // Assignee
        if let Some(assignee) = issue.assignees.first() {
            println!(
                "Assignee: {}",
                style(format!("@{}", assignee.login)).magenta()
            );
        }
        println!();

        // Action menu
        let actions = vec![
            "Work on this with AI",
            "Skip for now",
            "Mark complete",
            "Exit session",
        ];

        let action = Select::with_theme(&ctx.theme())
            .with_prompt("What would you like to do?")
            .items(&actions)
            .default(0)
            .interact()?;

        match action {
            0 => {
                // Assign to self and mark in-progress
                assign_and_start(ctx, issue.number)?;

                // Prepare context for AI (will be used when AI integration is added)
                let _context = format!(
                    "Working on GitHub issue #{}: {}\n\n{}\n\nLabels: {}",
                    issue.number,
                    issue.title,
                    issue.body.as_deref().unwrap_or(""),
                    labels.join(", ")
                );

                println!();
                println!(
                    "{}",
                    style("─────────────────────────────────────────────").dim()
                );
                println!();
                println!(
                    "Issue context loaded. I'm ready to help with #{}.",
                    issue.number
                );
                println!();
                println!(
                    "{}",
                    style("When done, type /done to mark complete and continue.").dim()
                );
                println!();

                // Return context - the CLI will use this
                // For now, we'll use an interactive prompt
                loop {
                    let input: String = Input::with_theme(&ctx.theme())
                        .with_prompt(">")
                        .allow_empty(true)
                        .interact_text()?;

                    if input.trim() == "/done" {
                        if ctx.confirm(&format!("Mark #{} as complete?", issue.number), true)? {
                            close_issue(ctx, issue.number)?;
                            ctx.print_success(&format!("#{} closed!", issue.number));
                        }
                        break;
                    } else if input.trim() == "/skip" {
                        println!("Skipping...");
                        break;
                    } else if input.trim() == "/exit" {
                        return Ok(());
                    } else if input.trim().starts_with("/issue ") {
                        // Quick create issue
                        let title = input.trim().strip_prefix("/issue ").unwrap();
                        if let Some(milestone) = &issue.milestone {
                            create_issue_in_milestone(ctx, title, &milestone.title)?;
                        } else {
                            create_issue(ctx, title)?;
                        }
                    } else {
                        println!();
                        println!(
                            "{}",
                            style("Commands: /done, /skip, /exit, /issue <title>").dim()
                        );
                        println!(
                            "{}",
                            style("(AI integration coming soon - for now, work in your editor)")
                                .dim()
                        );
                        println!();
                    }
                }
            }
            1 => {
                // Skip
                println!("Skipping #{}...", issue.number);
            }
            2 => {
                // Mark complete
                close_issue(ctx, issue.number)?;
                ctx.print_success(&format!("#{} closed!", issue.number));
            }
            3 => {
                // Exit
                return Ok(());
            }
            _ => {}
        }
    }

    println!();
    ctx.print_success(&format!(
        "Work session complete! {} issues processed.",
        total
    ));
    println!();

    Ok(())
}

// =============================================================================
// Task Parsing
// =============================================================================

struct Task {
    text: String,
    completed: bool,
}

fn parse_tasks(body: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- [ ] ") {
            tasks.push(Task {
                text: trimmed[6..].to_string(),
                completed: false,
            });
        } else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
            tasks.push(Task {
                text: trimmed[6..].to_string(),
                completed: true,
            });
        }
    }
    tasks
}

// =============================================================================
// Backlog & Past Milestones
// =============================================================================

fn view_backlog(ctx: &AppContext) -> Result<()> {
    let issues = fetch_backlog_issues(ctx)?;

    ctx.print_header("Backlog (no milestone)");
    println!();

    if issues.is_empty() {
        println!("  {}", style("No issues without a milestone").dim());
        println!();
        return Ok(());
    }

    print_issue_table(&issues);

    // Menu
    let items = vec!["Move issue to milestone", "+ Add issue", "← Back"];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Action")
        .items(&items)
        .default(0)
        .interact()?;

    match choice {
        0 => move_issue_to_milestone(ctx, &issues)?,
        1 => quick_add_issue(ctx, None)?,
        _ => {}
    }

    Ok(())
}

fn view_past_milestones(ctx: &AppContext) -> Result<()> {
    let milestones = fetch_milestones(ctx, "closed")?;

    ctx.print_header("Past Milestones");
    println!();

    if milestones.is_empty() {
        println!("  {}", style("No closed milestones").dim());
        println!();
        return Ok(());
    }

    for m in &milestones {
        print_milestone_row(m);
    }
    println!();

    Ok(())
}

// =============================================================================
// Issue Actions
// =============================================================================

fn quick_add_issue(ctx: &AppContext, milestone: Option<&str>) -> Result<()> {
    let title: String = Input::with_theme(&ctx.theme())
        .with_prompt("Issue title")
        .interact_text()?;

    if title.is_empty() {
        return Ok(());
    }

    if let Some(m) = milestone {
        create_issue_in_milestone(ctx, &title, m)?;
    } else {
        create_issue(ctx, &title)?;
    }

    Ok(())
}

fn move_issue_to_milestone(ctx: &AppContext, issues: &[GitHubIssue]) -> Result<()> {
    // Select issue
    let issue_items: Vec<String> = issues
        .iter()
        .map(|i| format!("#{} {}", i.number, i.title))
        .collect();

    let issue_choice = FuzzySelect::with_theme(&ctx.theme())
        .with_prompt("Select issue to move")
        .items(&issue_items)
        .default(0)
        .interact()?;

    let issue = &issues[issue_choice];

    // Select target milestone
    let milestones = fetch_milestones(ctx, "open")?;

    if milestones.is_empty() {
        println!("No open milestones. Create one first.");
        return Ok(());
    }

    let milestone_items: Vec<String> = milestones.iter().map(|m| m.title.clone()).collect();

    let milestone_choice = Select::with_theme(&ctx.theme())
        .with_prompt("Move to milestone")
        .items(&milestone_items)
        .default(0)
        .interact()?;

    let target = &milestones[milestone_choice];

    // Move issue
    CmdBuilder::new("gh")
        .args([
            "issue",
            "edit",
            &issue.number.to_string(),
            "--milestone",
            &target.title,
        ])
        .cwd(&ctx.repo)
        .run()?;

    ctx.print_success(&format!("#{} moved to {}", issue.number, target.title));

    Ok(())
}

fn assign_and_start(ctx: &AppContext, issue_number: u64) -> Result<()> {
    // Assign to self
    let _ = CmdBuilder::new("gh")
        .args([
            "issue",
            "edit",
            &issue_number.to_string(),
            "--add-assignee",
            "@me",
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    // Add in-progress label
    let _ = CmdBuilder::new("gh")
        .args([
            "issue",
            "edit",
            &issue_number.to_string(),
            "--add-label",
            "in-progress",
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    Ok(())
}

fn close_issue(ctx: &AppContext, issue_number: u64) -> Result<()> {
    CmdBuilder::new("gh")
        .args(["issue", "close", &issue_number.to_string()])
        .cwd(&ctx.repo)
        .run()?;
    Ok(())
}

// =============================================================================
// Milestone Actions
// =============================================================================

fn create_milestone(ctx: &AppContext) -> Result<()> {
    let title: String = Input::with_theme(&ctx.theme())
        .with_prompt("Milestone name")
        .interact_text()?;

    if title.is_empty() {
        return Ok(());
    }

    let description: String = Input::with_theme(&ctx.theme())
        .with_prompt("Description (optional)")
        .allow_empty(true)
        .interact_text()?;

    let due_date: String = Input::with_theme(&ctx.theme())
        .with_prompt("Due date YYYY-MM-DD (optional)")
        .allow_empty(true)
        .interact_text()?;

    // Create via API
    let mut args = vec![
        "api".to_string(),
        "repos/{owner}/{repo}/milestones".to_string(),
        "-f".to_string(),
        format!("title={}", title),
    ];

    if !description.is_empty() {
        args.push("-f".to_string());
        args.push(format!("description={}", description));
    }

    if !due_date.is_empty() {
        args.push("-f".to_string());
        args.push(format!("due_on={}T00:00:00Z", due_date));
    }

    CmdBuilder::new("gh")
        .args(args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .cwd(&ctx.repo)
        .run()?;

    ctx.print_success(&format!("Milestone '{}' created!", title));

    Ok(())
}

fn close_milestone(ctx: &AppContext, milestone_number: u64) -> Result<()> {
    CmdBuilder::new("gh")
        .args([
            "api",
            &format!("repos/{{owner}}/{{repo}}/milestones/{}", milestone_number),
            "-X",
            "PATCH",
            "-f",
            "state=closed",
        ])
        .cwd(&ctx.repo)
        .run()?;

    ctx.print_success("Milestone closed!");
    Ok(())
}

// =============================================================================
// GitHub API Helpers
// =============================================================================

fn fetch_milestones(ctx: &AppContext, state: &str) -> Result<Vec<Milestone>> {
    let output = CmdBuilder::new("gh")
        .args([
            "api",
            &format!(
                "repos/{{owner}}/{{repo}}/milestones?state={}&per_page=100",
                state
            ),
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let milestones: Vec<Milestone> =
        serde_json::from_str(&output.stdout_string()).unwrap_or_default();

    Ok(milestones)
}

fn fetch_milestone_issues(ctx: &AppContext, milestone: &str) -> Result<Vec<GitHubIssue>> {
    let output = CmdBuilder::new("gh")
        .args([
            "issue",
            "list",
            "--milestone",
            milestone,
            "--state",
            "all",
            "--json",
            "number,title,body,state,labels,assignees,milestone,createdAt",
            "--limit",
            "100",
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let mut issues: Vec<GitHubIssue> =
        serde_json::from_str(&output.stdout_string()).unwrap_or_default();

    // Sort by created date (newest first)
    issues.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(issues)
}

fn fetch_backlog_issues(ctx: &AppContext) -> Result<Vec<GitHubIssue>> {
    let output = CmdBuilder::new("gh")
        .args([
            "issue",
            "list",
            "--state",
            "open",
            "--json",
            "number,title,body,state,labels,assignees,milestone,createdAt",
            "--limit",
            "100",
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let all_issues: Vec<GitHubIssue> =
        serde_json::from_str(&output.stdout_string()).unwrap_or_default();

    // Filter to issues without milestone and sort by date (newest first)
    let mut backlog: Vec<GitHubIssue> = all_issues
        .into_iter()
        .filter(|i| i.milestone.is_none())
        .collect();

    backlog.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(backlog)
}

fn fetch_backlog_count(ctx: &AppContext) -> Result<usize> {
    let issues = fetch_backlog_issues(ctx)?;
    Ok(issues.len())
}

fn get_current_user(ctx: &AppContext) -> Result<String> {
    let output = CmdBuilder::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    Ok(output.stdout_string().trim().to_string())
}

fn create_issue(ctx: &AppContext, title: &str) -> Result<()> {
    let output = CmdBuilder::new("gh")
        .args(["issue", "create", "--title", title])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let stdout = output.stdout_string();
    let url = stdout.trim();
    ctx.print_success(&format!("Created: {}", url));
    Ok(())
}

fn create_issue_in_milestone(ctx: &AppContext, title: &str, milestone: &str) -> Result<()> {
    let output = CmdBuilder::new("gh")
        .args([
            "issue",
            "create",
            "--title",
            title,
            "--milestone",
            milestone,
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let stdout = output.stdout_string();
    let url = stdout.trim();
    ctx.print_success(&format!("Created: {}", url));
    Ok(())
}

// =============================================================================
// CLI Subcommands
// =============================================================================

/// Quick add a todo with sprint selection and optional expansion via Claude
pub fn quick_add_with_sprint(ctx: &AppContext, title: &str) -> Result<()> {
    if !cmd_exists("gh") {
        println!("{}", style("GitHub CLI (gh) not installed.").yellow());
        println!();
        println!("Install: https://cli.github.com/");
        return Ok(());
    }

    // Check if gh is authenticated
    let auth_check = CmdBuilder::new("gh")
        .args(["auth", "status"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    if auth_check.is_err() || auth_check.as_ref().map(|o| o.code != 0).unwrap_or(true) {
        println!(
            "{}",
            style("GitHub CLI not authenticated. Run: gh auth login").yellow()
        );
        return Ok(());
    }

    println!();
    println!("Adding: {}", style(title).cyan().bold());
    println!();

    // Fetch milestones for sprint selection
    let milestones = fetch_milestones(ctx, "open")?;

    if milestones.is_empty() {
        println!("{}", style("No active sprints/milestones found.").yellow());
        if ctx.confirm("Create issue in backlog (no milestone)?", true)? {
            // Ask if they want to expand on the task
            let body = prompt_for_expansion(ctx, title)?;
            create_issue_with_body(ctx, title, body.as_deref())?;
        }
        return Ok(());
    }

    // Build sprint selection
    let mut items: Vec<String> = milestones
        .iter()
        .map(|m| {
            let total = m.open_issues + m.closed_issues;
            let progress = if total > 0 {
                format!(" ({}/{})", m.closed_issues, total)
            } else {
                String::new()
            };
            format!("{}{}", m.title, progress)
        })
        .collect();
    items.push("📥 Backlog (no milestone)".to_string());

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Add to sprint")
        .items(&items)
        .default(0)
        .interact()?;

    // Ask if they want to expand on the task
    let body = prompt_for_expansion(ctx, title)?;

    if choice < milestones.len() {
        let milestone = &milestones[choice];
        create_issue_in_milestone_with_body(ctx, title, &milestone.title, body.as_deref())?;
    } else {
        create_issue_with_body(ctx, title, body.as_deref())?;
    }

    Ok(())
}

/// Prompt user if they want to expand on the task details
fn prompt_for_expansion(ctx: &AppContext, title: &str) -> Result<Option<String>> {
    let expand_options = vec!["Create issue now", "Add description first"];

    let expand_choice = Select::with_theme(&ctx.theme())
        .with_prompt("Options")
        .items(&expand_options)
        .default(0)
        .interact()?;

    if expand_choice == 1 {
        // Check if claude CLI is available for AI expansion
        if cmd_exists("claude") {
            let ai_options = vec!["Write description manually", "Expand with Claude AI"];

            let ai_choice = Select::with_theme(&ctx.theme())
                .with_prompt("How to add description?")
                .items(&ai_options)
                .default(0)
                .interact()?;

            if ai_choice == 1 {
                return expand_with_claude(ctx, title);
            }
        }

        // Manual description entry
        println!();
        println!(
            "{}",
            style("Enter description (empty line to finish):").dim()
        );
        let mut lines = Vec::new();
        loop {
            let line: String = Input::with_theme(&ctx.theme())
                .with_prompt(">")
                .allow_empty(true)
                .interact_text()?;
            if line.is_empty() {
                break;
            }
            lines.push(line);
        }

        if lines.is_empty() {
            return Ok(None);
        }

        Ok(Some(lines.join("\n")))
    } else {
        Ok(None)
    }
}

/// Use Claude CLI to expand on a task
fn expand_with_claude(ctx: &AppContext, title: &str) -> Result<Option<String>> {
    println!();
    println!(
        "{}",
        style("Starting conversation with Claude to expand on this task...").cyan()
    );
    println!(
        "{}",
        style("Type your thoughts, context, and requirements. Claude will help structure them.")
            .dim()
    );
    println!(
        "{}",
        style("Type 'done' when finished, 'cancel' to skip.").dim()
    );
    println!();

    let mut conversation = Vec::new();
    conversation.push(format!("Task: {}", title));

    loop {
        let input: String = Input::with_theme(&ctx.theme())
            .with_prompt("You")
            .allow_empty(true)
            .interact_text()?;

        let trimmed = input.trim().to_lowercase();
        if trimmed == "done" {
            break;
        }
        if trimmed == "cancel" {
            return Ok(None);
        }
        if input.is_empty() {
            continue;
        }

        conversation.push(format!("Context: {}", input));
    }

    if conversation.len() <= 1 {
        return Ok(None);
    }

    // Use claude CLI to generate a structured description
    println!();
    println!("{}", style("Generating issue description...").dim());

    let prompt = format!(
        "Based on this task and context, write a clear, concise GitHub issue description. \
         Include acceptance criteria as a checklist if appropriate. \
         Keep it practical and actionable. Output ONLY the issue body markdown, no extra text.\n\n{}",
        conversation.join("\n")
    );

    let output = CmdBuilder::new("claude")
        .args(["-p", &prompt])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    match output {
        Ok(result) if result.code == 0 => {
            let description = result.stdout_string().trim().to_string();
            println!();
            println!("{}", style("Generated description:").green());
            println!("{}", style("─".repeat(40)).dim());
            println!("{}", description);
            println!("{}", style("─".repeat(40)).dim());
            println!();

            if ctx.confirm("Use this description?", true)? {
                Ok(Some(description))
            } else {
                Ok(None)
            }
        }
        _ => {
            println!(
                "{}",
                style("Failed to generate description with Claude").yellow()
            );
            Ok(None)
        }
    }
}

/// Create an issue with optional body
fn create_issue_with_body(ctx: &AppContext, title: &str, body: Option<&str>) -> Result<()> {
    let mut args = vec!["issue", "create", "--title", title];

    let body_owned: String;
    if let Some(b) = body {
        body_owned = b.to_string();
        args.push("--body");
        args.push(&body_owned);
    }

    let output = CmdBuilder::new("gh")
        .args(args)
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let stdout = output.stdout_string();
    let url = stdout.trim();
    ctx.print_success(&format!("Created: {}", url));
    Ok(())
}

/// Create an issue in a milestone with optional body
fn create_issue_in_milestone_with_body(
    ctx: &AppContext,
    title: &str,
    milestone: &str,
    body: Option<&str>,
) -> Result<()> {
    let mut args = vec![
        "issue",
        "create",
        "--title",
        title,
        "--milestone",
        milestone,
    ];

    let body_owned: String;
    if let Some(b) = body {
        body_owned = b.to_string();
        args.push("--body");
        args.push(&body_owned);
    }

    let output = CmdBuilder::new("gh")
        .args(args)
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let stdout = output.stdout_string();
    let url = stdout.trim();
    ctx.print_success(&format!("Created: {}", url));
    Ok(())
}

/// Clean up backlog issues interactively - review and tidy each one
pub fn cleanup_issues(
    ctx: &AppContext,
    milestone_filter: Option<&str>,
    include_all: bool,
) -> Result<()> {
    if !cmd_exists("gh") {
        println!("{}", style("GitHub CLI (gh) not installed.").yellow());
        println!();
        println!("Install: https://cli.github.com/");
        return Ok(());
    }

    // Check if gh is authenticated
    let auth_check = CmdBuilder::new("gh")
        .args(["auth", "status"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    if auth_check.is_err() || auth_check.as_ref().map(|o| o.code != 0).unwrap_or(true) {
        println!(
            "{}",
            style("GitHub CLI not authenticated. Run: gh auth login").yellow()
        );
        return Ok(());
    }

    let header = if milestone_filter.is_some() {
        format!("Cleanup: {}", milestone_filter.unwrap())
    } else if include_all {
        "Cleanup: All Issues".to_string()
    } else {
        "Cleanup: Backlog".to_string()
    };
    ctx.print_header(&header);
    println!();

    // Fetch issues based on filters
    let issues = fetch_issues_for_cleanup(ctx, milestone_filter, include_all)?;

    if issues.is_empty() {
        println!("{}", style("No issues to clean up!").green());
        return Ok(());
    }

    println!("Found {} issues to review", style(issues.len()).cyan());
    println!();

    let mut cleaned = 0;
    let mut skipped = 0;
    let mut closed = 0;
    let mut moved = 0;

    let has_claude = cmd_exists("claude");

    for (idx, issue) in issues.iter().enumerate() {
        println!();
        println!(
            "{}",
            style(format!("━━━ {}/{} ━━━", idx + 1, issues.len())).dim()
        );
        println!();

        // Show issue details
        print_issue_details(issue);

        // Build action menu - include Claude option if available
        let mut actions = vec![
            "Skip (keep as-is)".to_string(),
            "Move to sprint".to_string(),
            "Close issue".to_string(),
        ];

        if has_claude {
            actions.push("Clean up with Claude".to_string());
        }

        actions.push("Edit title".to_string());
        actions.push("Edit description".to_string());
        actions.push("Delete issue".to_string());
        actions.push("Exit cleanup".to_string());

        let action = Select::with_theme(&ctx.theme())
            .with_prompt("Action")
            .items(&actions)
            .default(0)
            .interact()?;

        let action_str = &actions[action];

        if action_str == "Skip (keep as-is)" {
            skipped += 1;
            println!("{}", style("Skipped").dim());
        } else if action_str == "Move to sprint" {
            if move_issue_to_milestone_interactive(ctx, issue)? {
                moved += 1;
                cleaned += 1;
            }
        } else if action_str == "Close issue" {
            if issue.state == "open" {
                close_issue(ctx, issue.number)?;
                closed += 1;
                cleaned += 1;
                ctx.print_success(&format!("#{} closed", issue.number));
            } else {
                println!("{}", style("Already closed").dim());
            }
        } else if action_str == "Clean up with Claude" {
            if cleanup_with_claude(ctx, issue)? {
                cleaned += 1;
            }
        } else if action_str == "Edit title" {
            if edit_issue_title(ctx, issue)? {
                cleaned += 1;
            }
        } else if action_str == "Edit description" {
            if edit_issue_body(ctx, issue)? {
                cleaned += 1;
            }
        } else if action_str == "Delete issue" {
            if ctx.confirm(&format!("Delete #{} permanently?", issue.number), false)? {
                delete_issue(ctx, issue.number)?;
                cleaned += 1;
                ctx.print_success(&format!("#{} deleted", issue.number));
            }
        } else if action_str == "Exit cleanup" {
            println!();
            break;
        }
    }

    // Summary
    println!();
    println!("{}", style("━━━ Cleanup Summary ━━━").cyan());
    println!("  Reviewed: {}", issues.len());
    println!("  Cleaned:  {}", style(cleaned).green());
    println!("  Closed:   {}", closed);
    println!("  Moved:    {}", moved);
    println!("  Skipped:  {}", style(skipped).dim());
    println!();

    Ok(())
}

/// Use Claude to clean up an issue - improve title, description, add structure
fn cleanup_with_claude(ctx: &AppContext, issue: &GitHubIssue) -> Result<bool> {
    println!();
    println!("{}", style("Analyzing issue with Claude...").cyan());

    let current_body = issue.body.as_deref().unwrap_or("");
    let milestone_info = issue
        .milestone
        .as_ref()
        .map(|m| format!("Milestone: {}", m.title))
        .unwrap_or_else(|| "No milestone (backlog)".to_string());
    let labels: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();

    let prompt = format!(
        r#"You are helping clean up a GitHub issue. Analyze this issue and suggest improvements.

Current issue:
- Title: {}
- {}
- Labels: {}
- Description:
{}

Provide a cleaned up version with:
1. A clear, actionable title (if the current one is vague)
2. A well-structured description with:
   - Brief summary of what needs to be done
   - Acceptance criteria as a checkbox list (if applicable)
   - Any relevant context

Output format (use exactly this structure):
TITLE: <improved title or "KEEP" if current is good>
---
DESCRIPTION:
<improved description in markdown>

Be concise and practical. If the issue is already well-written, output TITLE: KEEP and keep the description minimal."#,
        issue.title,
        milestone_info,
        if labels.is_empty() {
            "none".to_string()
        } else {
            labels.join(", ")
        },
        if current_body.is_empty() {
            "(no description)"
        } else {
            current_body
        }
    );

    let output = CmdBuilder::new("claude")
        .args(["-p", &prompt])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    match output {
        Ok(result) if result.code == 0 => {
            let response = result.stdout_string();

            // Parse response
            let (new_title, new_body) = parse_claude_cleanup_response(&response, &issue.title);

            println!();
            println!("{}", style("Suggested changes:").green());
            println!("{}", style("─".repeat(50)).dim());

            let title_changed = new_title != issue.title;
            if title_changed {
                println!("{}", style("Title:").bold());
                println!("  {} {}", style("Old:").dim(), issue.title);
                println!("  {} {}", style("New:").cyan(), new_title);
            } else {
                println!("{} {}", style("Title:").bold(), style("(no change)").dim());
            }

            println!();
            println!("{}", style("Description:").bold());
            for line in new_body.lines().take(15) {
                println!("  {}", line);
            }
            if new_body.lines().count() > 15 {
                println!("  {}", style("... (truncated)").dim());
            }

            println!("{}", style("─".repeat(50)).dim());
            println!();

            // Ask what to apply
            let apply_options = vec![
                "Apply all changes",
                "Apply title only",
                "Apply description only",
                "Skip (no changes)",
            ];

            let apply_choice = Select::with_theme(&ctx.theme())
                .with_prompt("What to apply?")
                .items(&apply_options)
                .default(0)
                .interact()?;

            let mut changed = false;

            match apply_choice {
                0 => {
                    // Apply all
                    if title_changed {
                        CmdBuilder::new("gh")
                            .args([
                                "issue",
                                "edit",
                                &issue.number.to_string(),
                                "--title",
                                &new_title,
                            ])
                            .cwd(&ctx.repo)
                            .run()?;
                    }
                    CmdBuilder::new("gh")
                        .args([
                            "issue",
                            "edit",
                            &issue.number.to_string(),
                            "--body",
                            &new_body,
                        ])
                        .cwd(&ctx.repo)
                        .run()?;
                    ctx.print_success(&format!("#{} cleaned up", issue.number));
                    changed = true;
                }
                1 => {
                    // Title only
                    if title_changed {
                        CmdBuilder::new("gh")
                            .args([
                                "issue",
                                "edit",
                                &issue.number.to_string(),
                                "--title",
                                &new_title,
                            ])
                            .cwd(&ctx.repo)
                            .run()?;
                        ctx.print_success(&format!("#{} title updated", issue.number));
                        changed = true;
                    } else {
                        println!("{}", style("No title change needed").dim());
                    }
                }
                2 => {
                    // Description only
                    CmdBuilder::new("gh")
                        .args([
                            "issue",
                            "edit",
                            &issue.number.to_string(),
                            "--body",
                            &new_body,
                        ])
                        .cwd(&ctx.repo)
                        .run()?;
                    ctx.print_success(&format!("#{} description updated", issue.number));
                    changed = true;
                }
                _ => {
                    println!("{}", style("No changes applied").dim());
                }
            }

            Ok(changed)
        }
        _ => {
            println!("{}", style("Failed to analyze with Claude").yellow());
            Ok(false)
        }
    }
}

fn parse_claude_cleanup_response(response: &str, original_title: &str) -> (String, String) {
    let mut new_title = original_title.to_string();
    let mut new_body = String::new();

    let mut in_description = false;

    for line in response.lines() {
        if line.starts_with("TITLE:") {
            let title_part = line.strip_prefix("TITLE:").unwrap().trim();
            if title_part.to_uppercase() != "KEEP" && !title_part.is_empty() {
                new_title = title_part.to_string();
            }
        } else if line.starts_with("DESCRIPTION:") {
            in_description = true;
        } else if line == "---" {
            // Skip separator
        } else if in_description {
            if !new_body.is_empty() {
                new_body.push('\n');
            }
            new_body.push_str(line);
        }
    }

    // Trim leading/trailing whitespace from body
    new_body = new_body.trim().to_string();

    (new_title, new_body)
}

fn fetch_issues_for_cleanup(
    ctx: &AppContext,
    milestone_filter: Option<&str>,
    include_all: bool,
) -> Result<Vec<GitHubIssue>> {
    let args = vec![
        "issue",
        "list",
        "--state",
        "open",
        "--json",
        "number,title,body,state,labels,assignees,milestone,createdAt",
        "--limit",
        "100",
    ];

    let output = CmdBuilder::new("gh")
        .args(args)
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let all_issues: Vec<GitHubIssue> =
        serde_json::from_str(&output.stdout_string()).unwrap_or_default();

    // Filter based on options and sort by date (newest first)
    let mut filtered: Vec<GitHubIssue> = all_issues
        .into_iter()
        .filter(|i| {
            if let Some(m) = milestone_filter {
                // Filter to specific milestone
                i.milestone.as_ref().map(|im| im.title.as_str()) == Some(m)
            } else if include_all {
                // Include all
                true
            } else {
                // Default: backlog only (no milestone)
                i.milestone.is_none()
            }
        })
        .collect();

    filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(filtered)
}

fn print_issue_details(issue: &GitHubIssue) {
    let status = IssueStatus::from_issue(issue);
    let status_str = match status {
        IssueStatus::Todo => style("○ open").white(),
        IssueStatus::InProgress => style("◐ in progress").yellow(),
        IssueStatus::Done => style("● closed").green(),
    };

    println!(
        "{} {} {}",
        style(format!("#{}", issue.number)).cyan().bold(),
        status_str,
        style(&issue.title).bold()
    );

    // Milestone
    if let Some(m) = &issue.milestone {
        println!("  Milestone: {}", style(&m.title).magenta());
    } else {
        println!("  Milestone: {}", style("none (backlog)").dim());
    }

    // Labels
    if !issue.labels.is_empty() {
        let labels: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();
        println!("  Labels: {}", style(labels.join(", ")).cyan());
    }

    // Assignees
    if !issue.assignees.is_empty() {
        let assignees: Vec<String> = issue
            .assignees
            .iter()
            .map(|a| format!("@{}", a.login))
            .collect();
        println!("  Assignees: {}", style(assignees.join(", ")).magenta());
    }

    // Body excerpt
    if let Some(body) = &issue.body {
        if !body.is_empty() {
            let excerpt = if body.len() > 200 {
                format!("{}...", &body[..200])
            } else {
                body.clone()
            };
            println!();
            println!("  {}", style("Description:").dim());
            for line in excerpt.lines().take(5) {
                println!("    {}", style(line).dim());
            }
        }
    }
    println!();
}

fn move_issue_to_milestone_interactive(ctx: &AppContext, issue: &GitHubIssue) -> Result<bool> {
    let milestones = fetch_milestones(ctx, "open")?;

    let mut items: Vec<String> = milestones.iter().map(|m| m.title.clone()).collect();
    items.push("📥 Remove from milestone (backlog)".to_string());
    items.push("← Cancel".to_string());

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Move to")
        .items(&items)
        .default(0)
        .interact()?;

    if choice == items.len() - 1 {
        // Cancel
        return Ok(false);
    }

    if choice == items.len() - 2 {
        // Remove from milestone
        CmdBuilder::new("gh")
            .args([
                "issue",
                "edit",
                &issue.number.to_string(),
                "--milestone",
                "",
            ])
            .cwd(&ctx.repo)
            .run()?;
        ctx.print_success(&format!("#{} moved to backlog", issue.number));
    } else {
        let target = &milestones[choice];
        CmdBuilder::new("gh")
            .args([
                "issue",
                "edit",
                &issue.number.to_string(),
                "--milestone",
                &target.title,
            ])
            .cwd(&ctx.repo)
            .run()?;
        ctx.print_success(&format!("#{} moved to {}", issue.number, target.title));
    }

    Ok(true)
}

fn edit_issue_title(ctx: &AppContext, issue: &GitHubIssue) -> Result<bool> {
    let new_title: String = Input::with_theme(&ctx.theme())
        .with_prompt("New title")
        .with_initial_text(&issue.title)
        .interact_text()?;

    if new_title.trim() == issue.title || new_title.trim().is_empty() {
        println!("{}", style("No changes").dim());
        return Ok(false);
    }

    CmdBuilder::new("gh")
        .args([
            "issue",
            "edit",
            &issue.number.to_string(),
            "--title",
            new_title.trim(),
        ])
        .cwd(&ctx.repo)
        .run()?;

    ctx.print_success(&format!("#{} title updated", issue.number));
    Ok(true)
}

fn edit_issue_body(ctx: &AppContext, issue: &GitHubIssue) -> Result<bool> {
    let current_body = issue.body.as_deref().unwrap_or("");

    let options = vec!["Write new description", "Expand with Claude AI", "Cancel"];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("How to edit?")
        .items(&options)
        .default(0)
        .interact()?;

    let new_body = match choice {
        0 => {
            // Manual entry
            println!();
            if !current_body.is_empty() {
                println!("{}", style("Current description:").dim());
                for line in current_body.lines().take(10) {
                    println!("  {}", style(line).dim());
                }
                println!();
            }
            println!(
                "{}",
                style("Enter new description (empty line to finish):").dim()
            );

            let mut lines = Vec::new();
            loop {
                let line: String = Input::with_theme(&ctx.theme())
                    .with_prompt(">")
                    .allow_empty(true)
                    .interact_text()?;
                if line.is_empty() {
                    break;
                }
                lines.push(line);
            }

            if lines.is_empty() {
                return Ok(false);
            }

            lines.join("\n")
        }
        1 => {
            // Claude AI
            if !cmd_exists("claude") {
                println!("{}", style("Claude CLI not installed").yellow());
                return Ok(false);
            }

            match expand_with_claude(ctx, &issue.title)? {
                Some(body) => body,
                None => return Ok(false),
            }
        }
        _ => return Ok(false),
    };

    CmdBuilder::new("gh")
        .args([
            "issue",
            "edit",
            &issue.number.to_string(),
            "--body",
            &new_body,
        ])
        .cwd(&ctx.repo)
        .run()?;

    ctx.print_success(&format!("#{} description updated", issue.number));
    Ok(true)
}

fn reopen_issue(ctx: &AppContext, issue_number: u64) -> Result<()> {
    CmdBuilder::new("gh")
        .args(["issue", "reopen", &issue_number.to_string()])
        .cwd(&ctx.repo)
        .run()?;
    Ok(())
}

fn delete_issue(ctx: &AppContext, issue_number: u64) -> Result<()> {
    CmdBuilder::new("gh")
        .args(["issue", "delete", &issue_number.to_string(), "--yes"])
        .cwd(&ctx.repo)
        .run()?;
    Ok(())
}

pub fn add_todo(ctx: &AppContext, text: &str, section: Option<&str>) -> Result<()> {
    if let Some(milestone) = section {
        create_issue_in_milestone(ctx, text, milestone)
    } else {
        create_issue(ctx, text)
    }
}

pub fn remove_todo(ctx: &AppContext, item: &str) -> Result<()> {
    let issue_num = item.trim_start_matches('#');
    close_issue(ctx, issue_num.parse().context("Invalid issue number")?)
}

pub fn list_todos(ctx: &AppContext, pending_only: bool) -> Result<()> {
    let state = if pending_only { "open" } else { "all" };

    let output = CmdBuilder::new("gh")
        .args([
            "issue",
            "list",
            "--state",
            state,
            "--json",
            "number,title,state,labels,milestone",
            "--limit",
            "50",
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let issues: Vec<GitHubIssue> =
        serde_json::from_str(&output.stdout_string()).unwrap_or_default();

    for issue in &issues {
        let status = IssueStatus::from_issue(issue);
        let milestone = issue
            .milestone
            .as_ref()
            .map(|m| format!("[{}]", m.title))
            .unwrap_or_default();

        println!(
            "{} #{} {} {}",
            status.icon(),
            issue.number,
            issue.title,
            style(milestone).dim()
        );
    }

    Ok(())
}

pub fn show_status(ctx: &AppContext) -> Result<()> {
    let milestones = fetch_milestones(ctx, "open")?;

    ctx.print_header("Milestone Status");
    println!();

    if milestones.is_empty() {
        println!("No active milestones.");
        return Ok(());
    }

    for m in &milestones {
        print_milestone_row(m);
    }

    Ok(())
}

pub fn reset_checklist(_ctx: &AppContext, _name: &str) -> Result<()> {
    println!("Checklists are now managed via GitHub milestones.");
    println!("Use: gh api repos/{{owner}}/{{repo}}/milestones");
    Ok(())
}

pub fn run_checklist_by_name(ctx: &AppContext, name: &str) -> Result<()> {
    let milestones = fetch_milestones(ctx, "open")?;

    if let Some(m) = milestones
        .iter()
        .find(|m| m.title.to_lowercase() == name.to_lowercase())
    {
        view_milestone(ctx, m)
    } else {
        println!("Milestone '{}' not found.", name);
        println!();
        println!("Available milestones:");
        for m in &milestones {
            println!("  - {}", m.title);
        }
        Ok(())
    }
}
