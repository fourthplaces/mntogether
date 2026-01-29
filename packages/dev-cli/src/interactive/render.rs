//! Menu rendering with status indicators and workflow groups

use console::style;

use crate::menu::MenuAction;

use super::preferences::UserPreferences;
use super::state::EnvironmentStatus;
use super::types::{
    InteractiveMenuItem, InteractiveMenuState, ItemStatus, MenuGroup, WorkflowGroup,
};

/// Build the complete menu state with all items, status, and user preferences
pub fn build_menu_state(
    env_status: &EnvironmentStatus,
    prefs: &UserPreferences,
) -> InteractiveMenuState {
    let all_items = build_all_items(env_status);

    // Separate pinned items
    let pinned: Vec<InteractiveMenuItem> = all_items
        .iter()
        .filter(|item| prefs.is_pinned(&item.id))
        .cloned()
        .map(|mut item| {
            item.is_pinned = true;
            item
        })
        .collect();

    // Build groups
    let groups: Vec<MenuGroup> = WorkflowGroup::all()
        .iter()
        .map(|&group| {
            let items: Vec<InteractiveMenuItem> = all_items
                .iter()
                .filter(|item| item.group == group && !prefs.is_pinned(&item.id))
                .cloned()
                .collect();
            MenuGroup {
                group,
                items,
                expanded: !prefs.is_collapsed(group),
            }
        })
        .collect();

    InteractiveMenuState {
        groups,
        pinned,
        search_query: None,
        filtered_items: Vec::new(),
        selected_index: 0,
    }
}

/// Build all menu items with current status
fn build_all_items(env_status: &EnvironmentStatus) -> Vec<InteractiveMenuItem> {
    let docker_running = !env_status.docker.running.is_empty();
    let docker_status = if docker_running {
        ItemStatus::Running
    } else {
        ItemStatus::Stopped
    };

    vec![
        // ━━━ Bootstrap (first-time + recovery) ━━━
        InteractiveMenuItem::new(
            "status",
            "View status",
            MenuAction::Status,
            WorkflowGroup::Bootstrap,
        )
        .with_keywords(&["show", "info", "overview"]),
        InteractiveMenuItem::new(
            "doctor",
            "Run doctor",
            MenuAction::Doctor,
            WorkflowGroup::Bootstrap,
        )
        .with_keywords(&["health", "check", "diagnose"]),
        InteractiveMenuItem::new(
            "init",
            "Run init",
            MenuAction::Init,
            WorkflowGroup::Bootstrap,
        )
        .with_keywords(&["setup", "initialize", "start", "first-time"]),
        InteractiveMenuItem::new(
            "sync",
            "Sync all",
            MenuAction::Sync,
            WorkflowGroup::Bootstrap,
        )
        .with_keywords(&["pull", "update", "refresh", "git", "env", "migrate"]),
        // ━━━ Develop (day-to-day loop) ━━━
        InteractiveMenuItem::new(
            "dev:start",
            "Start dev",
            MenuAction::StartDev,
            WorkflowGroup::Develop,
        )
        .with_status(docker_status.clone())
        .with_keywords(&["up", "run", "docker", "environment"]),
        InteractiveMenuItem::new(
            "dev:stop",
            "Stop dev",
            MenuAction::StopDev,
            WorkflowGroup::Develop,
        )
        .with_keywords(&["down", "halt", "docker", "environment"]),
        InteractiveMenuItem::new(
            "watch",
            "Start watch",
            MenuAction::WatchMenu,
            WorkflowGroup::Develop,
        )
        .with_keywords(&["auto", "reload", "rebuild"]),
        InteractiveMenuItem::new(
            "logs",
            "View logs",
            MenuAction::Logs,
            WorkflowGroup::Develop,
        )
        .with_keywords(&["tail", "output", "container"]),
        InteractiveMenuItem::new(
            "shell",
            "Open shell",
            MenuAction::Shell,
            WorkflowGroup::Develop,
        )
        .with_keywords(&["bash", "terminal", "container", "exec"]),
        InteractiveMenuItem::new(
            "dev:mobile",
            "Start mobile",
            MenuAction::StartMobile,
            WorkflowGroup::Develop,
        )
        .with_keywords(&["ios", "android", "expo", "app"]),
        InteractiveMenuItem::new(
            "build",
            "Build",
            MenuAction::BuildMenu,
            WorkflowGroup::Develop,
        )
        .with_keywords(&["compile", "make"]),
        InteractiveMenuItem::new("todo", "Todo", MenuAction::TodoMenu, WorkflowGroup::Develop)
            .with_keywords(&["tasks", "issues", "github", "milestone", "sprint"]),
        // ━━━ Validate (pre-ship confidence) ━━━
        InteractiveMenuItem::new(
            "test",
            "Run tests",
            MenuAction::RunTests,
            WorkflowGroup::Validate,
        )
        .with_keywords(&["spec", "jest", "cargo"]),
        InteractiveMenuItem::new(
            "coverage",
            "Run coverage",
            MenuAction::RunCoverage,
            WorkflowGroup::Validate,
        )
        .with_keywords(&["test", "report"]),
        InteractiveMenuItem::new(
            "benchmark",
            "Run benchmarks",
            MenuAction::BenchmarkMenu,
            WorkflowGroup::Validate,
        )
        .with_keywords(&["perf", "performance", "speed"]),
        InteractiveMenuItem::new(
            "fmt",
            "Format code",
            MenuAction::FmtFix,
            WorkflowGroup::Validate,
        )
        .with_keywords(&["prettier", "rustfmt", "format"]),
        InteractiveMenuItem::new(
            "lint",
            "Lint code",
            MenuAction::LintFix,
            WorkflowGroup::Validate,
        )
        .with_keywords(&["eslint", "clippy", "check"]),
        InteractiveMenuItem::new(
            "check",
            "Run pre-commit",
            MenuAction::Check,
            WorkflowGroup::Validate,
        )
        .with_keywords(&["validate", "verify"]),
        // ━━━ Debug (deep inspection) ━━━
        InteractiveMenuItem::new(
            "ai",
            "AI tasks",
            MenuAction::AiTasksMenu,
            WorkflowGroup::Debug,
        )
        .with_keywords(&["assistant", "help", "claude"]),
        // ━━━ Ship (irreversible actions) ━━━
        InteractiveMenuItem::new(
            "release",
            "Release",
            MenuAction::Release,
            WorkflowGroup::Ship,
        )
        .with_keywords(&["tag", "version", "publish"]),
        // ━━━ Operate (infrastructure & environment) ━━━
        InteractiveMenuItem::new(
            "docker:up",
            "Start Docker",
            MenuAction::DockerUp,
            WorkflowGroup::Operate,
        )
        .with_status(docker_status.clone())
        .with_keywords(&["containers", "up"]),
        InteractiveMenuItem::new(
            "docker:stop",
            "Stop Docker",
            MenuAction::DockerStop,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["containers", "down"]),
        InteractiveMenuItem::new(
            "docker:restart",
            "Restart Docker",
            MenuAction::DockerRestart,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["containers", "reboot"]),
        InteractiveMenuItem::new(
            "docker:build",
            "Build Docker",
            MenuAction::DockerBuild,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["containers", "image"]),
        InteractiveMenuItem::new(
            "docker:nuke",
            "Rebuild (clean)",
            MenuAction::DockerNuke,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["destroy", "nuke", "reset"]),
        InteractiveMenuItem::new(
            "db",
            "Open database",
            MenuAction::DbMenu,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["postgres", "sql", "psql"]),
        InteractiveMenuItem::new(
            "migrate",
            "Run migrations",
            MenuAction::Migrate,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["database", "schema"]),
        InteractiveMenuItem::new(
            "env",
            "Open environment",
            MenuAction::EnvMenu,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["variables", "secrets", "config"]),
        InteractiveMenuItem::new(
            "tunnel",
            "Start tunnel",
            MenuAction::Tunnel,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["ngrok", "expose", "public", "http"]),
        InteractiveMenuItem::new(
            "urls",
            "Open URLs",
            MenuAction::OpenUrls,
            WorkflowGroup::Operate,
        )
        .with_keywords(&["browser", "link"]),
    ]
}

/// Format a menu item for display (with optional indentation)
pub fn format_item(item: &InteractiveMenuItem, _is_selected: bool) -> String {
    format_item_indented(item, _is_selected, 0)
}

/// Format a menu item with specific indentation level
/// Note: No ANSI styling - FuzzySelect doesn't render it properly
pub fn format_item_indented(
    item: &InteractiveMenuItem,
    _is_selected: bool,
    indent: usize,
) -> String {
    let mut parts = Vec::new();

    // Indentation
    if indent > 0 {
        parts.push(" ".repeat(indent));
    }

    // Pin indicator
    if item.is_pinned {
        parts.push("★ ".to_string());
    }

    // Label
    parts.push(item.label.clone());

    // Status indicator (suffix, no color since FuzzySelect doesn't support ANSI)
    if item.status == ItemStatus::Running {
        parts.push(" ●".to_string());
    } else if item.status == ItemStatus::Stopped {
        parts.push(" ○".to_string());
    }

    parts.join("")
}

/// Format a group header
pub fn format_group_header(group: &MenuGroup) -> String {
    let arrow = if group.expanded { "▾" } else { "▸" };
    format!(
        "{} {} {}",
        arrow,
        group.group.icon(),
        style(group.group.label()).bold()
    )
}

/// Format the status bar at the top of the menu
pub fn format_status_bar(env_status: &EnvironmentStatus) -> String {
    let mut parts = Vec::new();

    // Docker status
    let docker_indicator = if env_status.docker.running.is_empty() {
        style("○").red()
    } else if env_status.docker.stopped.is_empty() {
        style("●").green()
    } else {
        style("◐").yellow()
    };
    parts.push(format!(
        "Docker {} {}",
        docker_indicator,
        style(&env_status.docker.summary()).dim()
    ));

    // Git status
    if env_status.git.is_repo {
        let git_indicator = if env_status.git.uncommitted > 0 {
            style("●").yellow()
        } else {
            style("●").green()
        };
        parts.push(format!(
            "Git {} {}",
            git_indicator,
            style(&env_status.git.summary()).dim()
        ));
    }

    // DB status
    if env_status.migrations.db_available {
        let db_indicator = if env_status.migrations.pending > 0 {
            style("◐").yellow()
        } else {
            style("●").green()
        };
        parts.push(format!(
            "DB {} {}",
            db_indicator,
            style(&env_status.migrations.summary()).dim()
        ));
    }

    parts.join("  │  ")
}
