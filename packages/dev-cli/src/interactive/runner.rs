//! Main interactive menu loop

use anyhow::Result;
use console::style;
use dialoguer::{FuzzySelect, Input, Select};

use crate::config::Config;
use crate::context::AppContext;
use crate::history::record_action;
use crate::menu::MenuAction;

use super::preferences::UserPreferences;
use super::render::{build_menu_state, format_item, format_item_indented, format_status_bar};
use super::search::MenuSearch;
use super::state::EnvironmentStatus;
use super::types::{InteractiveMenuItem, WorkflowGroup};

/// Entry for the menu display (can be group header, item, or special)
enum DisplayEntry {
    GroupHeader(WorkflowGroup, bool), // group, is_expanded
    SectionHeader(String),            // non-selectable section label (Pinned, Recent)
    Item(InteractiveMenuItem, usize), // item, indent level
    Spacer,                           // empty line between groups
    Separator,
    SearchPrompt,
    Quit,
}

impl DisplayEntry {
    fn label(&self) -> String {
        match self {
            Self::GroupHeader(group, expanded) => {
                let arrow = if *expanded { "▾" } else { "▸" };
                format!("{} {} {}", arrow, group.icon(), group.label())
            }
            Self::SectionHeader(title) => format!("  {}", title),
            Self::Item(item, indent) => format_item_indented(item, false, *indent),
            Self::Spacer => String::new(),
            Self::Separator => "─".repeat(40),
            Self::SearchPrompt => "/ Search...".to_string(),
            Self::Quit => "q Quit".to_string(),
        }
    }
}

/// Run the new interactive menu
pub fn run_interactive(ctx: &AppContext) -> Result<()> {
    // Use config from context and load preferences
    let config_ref = Some(&ctx.config);
    let mut prefs = UserPreferences::load();

    // Show header
    if !ctx.quiet {
        println!();
        println!("{}", style("Development CLI").bold());
    }

    loop {
        // Collect current environment status
        let env_status = EnvironmentStatus::collect(ctx);

        println!("OK");
        // Show status bar
        if !ctx.quiet {
            println!();
            println!("{}", format_status_bar(&env_status));
            println!();
        }

        // Build menu state
        let menu_state = build_menu_state(&env_status, &prefs);

        // Build display entries
        let mut entries: Vec<DisplayEntry> = Vec::new();

        // Pinned items
        if !menu_state.pinned.is_empty() {
            entries.push(DisplayEntry::Spacer);
            entries.push(DisplayEntry::SectionHeader("★ Pinned".to_string()));
            for item in &menu_state.pinned {
                entries.push(DisplayEntry::Item(item.clone(), 2));
            }
            entries.push(DisplayEntry::Separator);
        }

        // Workflow groups
        for group in &menu_state.groups {
            entries.push(DisplayEntry::Spacer);
            entries.push(DisplayEntry::GroupHeader(group.group, group.expanded));
            if group.expanded {
                for item in &group.items {
                    entries.push(DisplayEntry::Item(item.clone(), 3));
                }
            }
        }

        entries.push(DisplayEntry::Spacer);
        entries.push(DisplayEntry::Quit);

        // Build labels for selection
        let labels: Vec<String> = entries.iter().map(|e| e.label()).collect();

        // Find first selectable item
        let default_idx = entries
            .iter()
            .position(|e| {
                matches!(
                    e,
                    DisplayEntry::Item(_, _) | DisplayEntry::GroupHeader(_, _)
                )
            })
            .unwrap_or(0);

        // Show menu (FuzzySelect allows typing to filter)
        let choice = FuzzySelect::with_theme(&ctx.theme())
            .with_prompt("Select action (type to search)")
            .items(&labels)
            .default(default_idx)
            .interact()?;

        // Handle selection
        match &entries[choice] {
            DisplayEntry::GroupHeader(group, _expanded) => {
                // Toggle group expansion
                prefs.toggle_collapse(*group);
            }
            DisplayEntry::SectionHeader(_) => {
                // Section headers are not actionable, skip
            }
            DisplayEntry::Item(item, _) => {
                // Record action for history
                record_action(&item.label);

                // Execute action
                if let Some(should_exit) =
                    execute_action(ctx, config_ref, &item.action, &mut prefs)?
                {
                    if should_exit {
                        break;
                    }
                }
            }
            DisplayEntry::Quit => break,
            DisplayEntry::Spacer | DisplayEntry::Separator | DisplayEntry::SearchPrompt => {}
        }
    }

    Ok(())
}

/// Run search mode - show search prompt and filter items
fn run_search_mode(
    ctx: &AppContext,
    env_status: &EnvironmentStatus,
    prefs: &UserPreferences,
) -> Result<Option<InteractiveMenuItem>> {
    let menu_state = build_menu_state(env_status, prefs);
    let search = MenuSearch::new();

    // Collect all searchable items
    let all_items: Vec<InteractiveMenuItem> = menu_state
        .groups
        .iter()
        .flat_map(|g| g.items.clone())
        .chain(menu_state.pinned.clone())
        .collect();

    // Get search query
    let query: String = Input::with_theme(&ctx.theme())
        .with_prompt("Search")
        .allow_empty(true)
        .interact_text()?;

    if query.is_empty() {
        return Ok(None);
    }

    // Search items
    let results = search.search(&all_items, &query);
    if results.is_empty() {
        println!("No matches found for '{}'", query);
        return Ok(None);
    }

    // Show results
    let labels: Vec<String> = results
        .iter()
        .map(|r| format_item(&r.item, false))
        .collect();

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt(format!("Results for '{}'", query))
        .items(&labels)
        .default(0)
        .interact()?;

    Ok(Some(results[choice].item.clone()))
}

/// Execute a menu action, returns Some(true) if should exit, Some(false) to continue, None on error
fn execute_action(
    ctx: &AppContext,
    config: Option<&Config>,
    action: &MenuAction,
    _prefs: &mut UserPreferences,
) -> Result<Option<bool>> {
    use crate::cmd::{
        ai_assistant,
        benchmark::benchmark_menu_with_config,
        cmd as cmd_runner,
        coverage::coverage_menu,
        db::{db_menu, db_migrate},
        docker::{
            docker_compose_build, docker_compose_up, docker_shell, logs_menu, nuke_rebuild_menu,
            restart_containers_menu, stop_docker_containers, up_containers_menu,
        },
        env::{env_menu_with_config, pull_env_with_config},
        mobile::start_mobile,
        open_url_menu, print_doctor,
        quality::{run_check, run_fmt, run_lint},
        release::release_interactive,
        status::{init_setup, show_status, sync_all},
        test::test_menu,
        tunnel::start_http_tunnel,
        watch::watch_menu,
    };

    match action {
        MenuAction::Exit => return Ok(Some(true)),

        // Quick Start
        MenuAction::StartDev => {
            let _ = pull_env_with_config(ctx, config, "dev", ".env.dev");
            docker_compose_up(ctx, &[], false)?;
        }
        MenuAction::StopDev => stop_docker_containers(ctx)?,
        MenuAction::StartMobile => start_mobile(ctx, None)?,
        MenuAction::Sync => sync_all(ctx)?,

        // Docker
        MenuAction::DockerUp => up_containers_menu(ctx)?,
        MenuAction::DockerStop => stop_docker_containers(ctx)?,
        MenuAction::DockerRestart => restart_containers_menu(ctx)?,
        MenuAction::DockerBuild => docker_compose_build(ctx, &[], false, false)?,
        MenuAction::DockerNuke => nuke_rebuild_menu(ctx)?,
        MenuAction::Logs => logs_menu(ctx)?,
        MenuAction::Shell => docker_shell(ctx, None)?,

        // Database
        MenuAction::DbMenu => db_menu(ctx)?,
        MenuAction::Migrate => db_migrate(ctx, None)?,

        // Environment
        MenuAction::EnvMenu => env_menu_with_config(ctx, config)?,

        // Code Quality
        MenuAction::FmtFix => {
            run_fmt(ctx, true, false)?;
        }
        MenuAction::LintFix => {
            run_lint(ctx, true, false)?;
        }
        MenuAction::Check => run_check(ctx)?,

        // Build & Watch
        MenuAction::BuildMenu => {
            if let Some(cfg) = config {
                // Interactive build via cmd system
                let opts = cmd_runner::CmdOptions {
                    parallel: false,
                    variant: None,
                    packages: vec![],
                    capture: false,
                };
                let _ = cmd_runner::run_cmd(ctx, cfg, "build", &opts);
            } else {
                println!("Build requires config. Create .dev/config.toml");
            }
        }
        MenuAction::WatchMenu => watch_menu(ctx, config)?,

        // Testing
        MenuAction::RunTests => test_menu(ctx)?,
        MenuAction::RunCoverage => coverage_menu(ctx)?,
        MenuAction::BenchmarkMenu => benchmark_menu_with_config(ctx, config)?,

        // Git
        MenuAction::Release => {
            if let Some(cfg) = config {
                release_interactive(ctx, cfg, false)?;
            } else {
                println!("Release requires config. Create .dev/config.toml");
            }
        }

        // Utilities
        MenuAction::Status => show_status(ctx)?,
        MenuAction::OpenUrls => {
            if let Some(cfg) = config {
                open_url_menu(ctx, cfg)?;
            } else {
                println!("No config found. Create .dev/config.toml with [urls.*] entries.");
            }
        }
        MenuAction::Tunnel => start_http_tunnel(ctx)?,
        MenuAction::Doctor => print_doctor(ctx),
        MenuAction::Init => init_setup(ctx)?,

        // AI
        MenuAction::AiTasksMenu => ai_assistant::task_menu(ctx)?,

        // Todo
        MenuAction::TodoMenu => crate::cmd::todos::todos_menu(ctx)?,
    }

    Ok(Some(false))
}
