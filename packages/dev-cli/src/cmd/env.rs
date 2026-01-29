//! Environment variable management commands

use anyhow::{anyhow, Context, Result};
use dialoguer::{MultiSelect, Select};
use std::collections::HashMap;
use std::fs;

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Get ESC path for an environment, using config if available
fn get_esc_path(config: Option<&Config>, env_name: &str) -> Result<String> {
    config.map(|c| c.esc_path(env_name)).ok_or_else(|| {
        anyhow!("No config found. Run from a directory with dev.yaml or specify config.")
    })
}

/// Sync environment variables from ESC
pub fn pull_env(ctx: &AppContext, env_name: &str, out_file: &str) -> Result<()> {
    pull_env_with_config(ctx, None, env_name, out_file)
}

/// Sync environment variables from ESC with config
pub fn pull_env_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    env_name: &str,
    out_file: &str,
) -> Result<()> {
    if !cmd_exists("esc") {
        return Err(anyhow!(
            "esc not found. Install esc CLI to pull env vars (needed for {out_file})."
        ));
    }

    let esc_path = get_esc_path(config, env_name)?;

    let out = CmdBuilder::new("esc")
        .args([
            "env",
            "get",
            &esc_path,
            "--value",
            "dotenv",
            "--show-secrets",
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()
        .with_context(|| format!("failed to pull env {env_name} via esc"))?;

    let target = ctx.repo.join(out_file);
    fs::write(&target, &out.stdout).with_context(|| format!("write {}", target.display()))?;
    ctx.print_success(&format!("Wrote {}", target.display()));
    Ok(())
}

#[allow(dead_code)]
/// Set an environment variable in ESC
pub fn set_env_var(
    ctx: &AppContext,
    env_name: &str,
    key: &str,
    value: &str,
    is_secret: bool,
) -> Result<()> {
    set_env_var_with_config(ctx, None, env_name, key, value, is_secret)
}

/// Set an environment variable in ESC with config
pub fn set_env_var_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    env_name: &str,
    key: &str,
    value: &str,
    is_secret: bool,
) -> Result<()> {
    if !cmd_exists("esc") {
        return Err(anyhow!("esc not found. Install esc CLI to set env vars."));
    }

    let path = format!("values.{}", key);
    let env_path = get_esc_path(config, env_name)?;

    let mut args = vec!["env", "set", &env_path, &path, value];

    if is_secret {
        args.push("--secret");
    }

    ctx.print_header(&format!(
        "Setting {} in {} (secret: {})...",
        key, env_name, is_secret
    ));

    let code = CmdBuilder::new("esc").args(args).cwd(&ctx.repo).run()?;

    if code != 0 {
        return Err(anyhow!("esc env set exited with code {code}"));
    }

    ctx.print_success(&format!("Successfully set {} in {}", key, env_name));
    Ok(())
}

#[allow(dead_code)]
/// Interactive menu for setting environment variables
pub fn set_env_menu(ctx: &AppContext) -> Result<()> {
    set_env_menu_with_config(ctx, None)
}

/// Interactive menu for setting environment variables with config
pub fn set_env_menu_with_config(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    ctx.print_header("Set environment variable");

    let env_items: Vec<&str> = config
        .map(|c| {
            c.global
                .environments
                .available
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_else(|| vec!["dev", "prod"]);
    let env_choice = if ctx.quiet {
        0
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Which environment?")
            .items(&env_items)
            .default(0)
            .interact()?
    };
    let env_name = env_items[env_choice];

    let key: String = dialoguer::Input::with_theme(&ctx.theme())
        .with_prompt("Variable name")
        .interact_text()?;

    if key.trim().is_empty() {
        return Err(anyhow!("Variable name cannot be empty"));
    }

    let value: String = dialoguer::Input::with_theme(&ctx.theme())
        .with_prompt("Variable value")
        .interact_text()?;

    let is_secret = ctx.confirm("Is this a secret value?", true)?;

    set_env_var_with_config(ctx, config, env_name, key.trim(), &value, is_secret)
}

/// Show deployment info for an environment (ESC secrets and Pulumi outputs)
pub fn show_deployments(
    ctx: &AppContext,
    config: Option<&Config>,
    env: Option<&str>,
) -> Result<()> {
    let envs = match env {
        Some(e) => vec![e],
        None => {
            // Interactive selection
            let env_items = vec!["dev", "prod"];
            let choice = if ctx.quiet {
                0
            } else {
                Select::with_theme(&ctx.theme())
                    .with_prompt("Which environment?")
                    .items(&env_items)
                    .default(0)
                    .interact()?
            };
            vec![env_items[choice]]
        }
    };

    for env_name in &envs {
        let esc_env = get_esc_path(config, env_name)?;

        ctx.print_header(&format!("{} Environment", env_name.to_uppercase()));

        // Show ESC secrets
        println!();
        println!("ESC Secrets:");
        if cmd_exists("esc") {
            let result = CmdBuilder::new("esc")
                .args(["env", "get", &esc_env, "--value", "json", "--show-secrets"])
                .cwd(&ctx.repo)
                .capture_stdout()
                .run_capture();

            match result {
                Ok(output) => {
                    // Parse and display key secrets
                    let stdout = output.stdout_string();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        print_secrets_from_json(&json, "  ");
                    } else {
                        println!("  (Could not parse ESC output)");
                    }
                }
                Err(e) => {
                    println!("  (Failed to fetch: {})", e);
                }
            }
        } else {
            println!("  (esc CLI not installed)");
        }

        // Show Pulumi stack outputs
        println!();
        println!("Pulumi Stack Outputs:");
        if cmd_exists("pulumi") {
            let stacks = ["core", "api", "ember", "app"];
            for stack in stacks {
                let stack_name = format!("shaya/{}/{}", stack, env_name);
                let result = CmdBuilder::new("pulumi")
                    .args([
                        "stack",
                        "output",
                        "--json",
                        "--show-secrets",
                        "-s",
                        &stack_name,
                    ])
                    .cwd(&ctx.repo)
                    .capture_stdout()
                    .run_capture();

                if let Ok(output) = result {
                    let stdout = output.stdout_string();
                    if !stdout.trim().is_empty() {
                        println!("  [{}]", stack);
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                            if let Some(obj) = json.as_object() {
                                for (key, value) in obj {
                                    let display_val = format_value(value);
                                    println!("    {}: {}", key, display_val);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            println!("  (pulumi CLI not installed)");
        }

        // Show connection commands
        println!();
        println!("Connection Commands:");
        println!("  # Start a shell with {} secrets:", env_name);
        println!("  esc run {} -- bash", esc_env);
        println!();
        println!("  # Run a command with {} secrets:", env_name);
        println!("  esc run {} -- <command>", esc_env);
        println!();
        println!("  # Open environment in browser:");
        println!("  esc env open {}", esc_env);
        println!();
    }

    Ok(())
}

fn print_secrets_from_json(json: &serde_json::Value, indent: &str) {
    if let Some(obj) = json.as_object() {
        for (key, value) in obj {
            match value {
                serde_json::Value::Object(_) => {
                    println!("{}[{}]", indent, key);
                    print_secrets_from_json(value, &format!("{}  ", indent));
                }
                _ => {
                    let display_val = format_value(value);
                    println!("{}{}: {}", indent, key, display_val);
                }
            }
        }
    }
}

fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "(null)".to_string(),
        other => other.to_string(),
    }
}

/// Parse a .env file and return a list of (key, value) pairs
fn parse_env_file(content: &str) -> Vec<(String, String)> {
    let mut vars = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE format
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim();

            // Remove surrounding quotes if present
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value)
                .to_string();

            vars.push((key, value));
        }
    }

    vars
}

/// Fetch current environment variables from ESC as a HashMap
/// Returns (values, secrets_set) where secrets_set contains keys that are marked as secrets
fn fetch_esc_env(
    ctx: &AppContext,
    config: Option<&Config>,
    env_name: &str,
) -> Result<(HashMap<String, String>, std::collections::HashSet<String>)> {
    let esc_env = get_esc_path(config, env_name)?;

    // First fetch without --show-secrets to identify which are secrets
    let out_masked = CmdBuilder::new("esc")
        .args(["env", "get", &esc_env, "--value", "json"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()
        .with_context(|| format!("failed to fetch env {} via esc", env_name))?;

    let masked_json: serde_json::Value = serde_json::from_str(&out_masked.stdout_string())
        .with_context(|| "failed to parse ESC JSON output")?;

    let mut secrets_set = std::collections::HashSet::new();
    // Check environmentVariables section for secrets
    if let Some(values) = masked_json
        .get("environmentVariables")
        .and_then(|v| v.as_object())
    {
        for (key, value) in values {
            if value.as_str() == Some("[secret]") {
                secrets_set.insert(key.clone());
            }
        }
    }

    // Now fetch with --show-secrets to get actual values
    let out = CmdBuilder::new("esc")
        .args(["env", "get", &esc_env, "--value", "json", "--show-secrets"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()
        .with_context(|| format!("failed to fetch env {} via esc", env_name))?;

    let stdout = out.stdout_string();
    let json: serde_json::Value =
        serde_json::from_str(&stdout).with_context(|| "failed to parse ESC JSON output")?;

    let mut result = HashMap::new();

    // Read from environmentVariables section (where env vars are stored)
    if let Some(values) = json.get("environmentVariables").and_then(|v| v.as_object()) {
        for (key, value) in values {
            if let Some(s) = value.as_str() {
                result.insert(key.clone(), s.to_string());
            } else if !value.is_null() {
                result.insert(key.clone(), value.to_string());
            }
        }
    }

    Ok((result, secrets_set))
}

/// Check if a variable name looks like it should be a secret
fn looks_like_secret(key: &str) -> bool {
    let key_upper = key.to_uppercase();
    key_upper.contains("SECRET")
        || key_upper.contains("PASSWORD")
        || key_upper.contains("TOKEN")
        || key_upper.contains("KEY")
        || key_upper.contains("CREDENTIAL")
        || key_upper.contains("AUTH")
}

/// Represents a change between local and remote env vars
#[derive(Debug)]
enum EnvChange {
    Added(String, String),
    Modified(String, String, String), // key, old_value, new_value
    Removed(String, String),          // key, old_value
}

#[allow(dead_code)]
/// Push environment variables from a local .env file to ESC
pub fn push_env(ctx: &AppContext, env_name: &str, env_file: &str) -> Result<()> {
    push_env_with_config(ctx, None, env_name, env_file)
}

/// Push environment variables from a local .env file to ESC with config
pub fn push_env_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    env_name: &str,
    env_file: &str,
) -> Result<()> {
    if !cmd_exists("esc") {
        return Err(anyhow!("esc not found. Install esc CLI to push env vars."));
    }

    let env_path = ctx.repo.join(env_file);
    if !env_path.exists() {
        return Err(anyhow!(
            "{} not found. Create it first or pull from esc.",
            env_file
        ));
    }

    let content =
        fs::read_to_string(&env_path).with_context(|| format!("failed to read {}", env_file))?;

    let local_vars: HashMap<String, String> = parse_env_file(&content).into_iter().collect();

    if local_vars.is_empty() {
        return Err(anyhow!("No variables found in {}", env_file));
    }

    let esc_env = get_esc_path(config, env_name)?;

    // Fetch current ESC values and which are secrets
    ctx.print_header(&format!(
        "Fetching current {} environment from ESC...",
        env_name
    ));
    let (remote_vars, existing_secrets) = fetch_esc_env(ctx, config, env_name)?;

    // Compare and find changes
    let mut changes: Vec<EnvChange> = Vec::new();

    // Find added and modified vars
    for (key, local_value) in &local_vars {
        match remote_vars.get(key) {
            Some(remote_value) if remote_value != local_value => {
                changes.push(EnvChange::Modified(
                    key.clone(),
                    remote_value.clone(),
                    local_value.clone(),
                ));
            }
            None => {
                changes.push(EnvChange::Added(key.clone(), local_value.clone()));
            }
            _ => {} // Same value, no change
        }
    }

    // Find removed vars (in remote but not in local)
    for (key, remote_value) in &remote_vars {
        if !local_vars.contains_key(key) {
            changes.push(EnvChange::Removed(key.clone(), remote_value.clone()));
        }
    }

    if changes.is_empty() {
        ctx.print_success(&format!(
            "No changes detected between {} and {}",
            env_file, esc_env
        ));
        return Ok(());
    }

    // Sort changes for consistent display
    changes.sort_by(|a, b| {
        let key_a = match a {
            EnvChange::Added(k, _) | EnvChange::Modified(k, _, _) | EnvChange::Removed(k, _) => k,
        };
        let key_b = match b {
            EnvChange::Added(k, _) | EnvChange::Modified(k, _, _) | EnvChange::Removed(k, _) => k,
        };
        key_a.cmp(key_b)
    });

    // Display changes and build selection items
    ctx.print_header(&format!(
        "Found {} change(s) between {} and {}:",
        changes.len(),
        env_file,
        esc_env
    ));
    println!();

    let mut selection_items: Vec<String> = Vec::new();

    for change in &changes {
        let (label, description) = match change {
            EnvChange::Added(key, value) => {
                let display_value = truncate_value(value, 40);
                let label = format!("[+] {} = {}", key, display_value);
                println!("  \x1b[32m{}\x1b[0m", label); // Green
                (format!("[+] {}", key), format!("Add: {}", display_value))
            }
            EnvChange::Modified(key, old_value, new_value) => {
                let old_display = truncate_value(old_value, 30);
                let new_display = truncate_value(new_value, 30);
                let label = format!("[~] {} : {} → {}", key, old_display, new_display);
                println!("  \x1b[33m{}\x1b[0m", label); // Yellow
                (
                    format!("[~] {}", key),
                    format!("{} → {}", old_display, new_display),
                )
            }
            EnvChange::Removed(key, old_value) => {
                let display_value = truncate_value(old_value, 40);
                let label = format!("[-] {} (was: {})", key, display_value);
                println!("  \x1b[31m{}\x1b[0m", label); // Red
                (
                    format!("[-] {}", key),
                    format!("Remove (was: {})", display_value),
                )
            }
        };
        selection_items.push(format!("{} - {}", label, description));
    }
    println!();

    // Let user select which changes to apply
    if ctx.quiet {
        // In quiet mode, apply all changes with existing secret status preserved
        let secrets: std::collections::HashSet<String> = changes
            .iter()
            .filter_map(|c| match c {
                EnvChange::Added(k, _) | EnvChange::Modified(k, _, _) => {
                    if existing_secrets.contains(k) || looks_like_secret(k) {
                        Some(k.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        return apply_changes(
            ctx,
            &esc_env,
            &changes,
            (0..changes.len()).collect(),
            secrets,
        );
    }

    let selections = MultiSelect::with_theme(&ctx.theme())
        .with_prompt("Select changes to push (space to toggle, enter to confirm)")
        .items(&selection_items)
        .defaults(&vec![true; changes.len()])
        .interact()?;

    if selections.is_empty() {
        println!("No changes selected. Cancelled.");
        return Ok(());
    }

    // Collect add/modify changes that need secret selection
    let secret_candidates: Vec<(usize, &str)> = selections
        .iter()
        .filter_map(|&idx| match &changes[idx] {
            EnvChange::Added(key, _) | EnvChange::Modified(key, _, _) => Some((idx, key.as_str())),
            EnvChange::Removed(_, _) => None,
        })
        .collect();

    // Ask which should be secrets
    let mut secrets_to_push: std::collections::HashSet<String> = std::collections::HashSet::new();

    if !secret_candidates.is_empty() {
        println!();
        ctx.print_header("Select which variables should be stored as secrets:");

        let secret_items: Vec<String> = secret_candidates
            .iter()
            .map(|(_, key)| {
                let is_existing_secret = existing_secrets.contains(*key);
                if is_existing_secret {
                    format!("{} (currently a secret)", key)
                } else {
                    key.to_string()
                }
            })
            .collect();

        // Default to existing secrets + anything that looks like a secret
        let defaults: Vec<bool> = secret_candidates
            .iter()
            .map(|(_, key)| existing_secrets.contains(*key) || looks_like_secret(key))
            .collect();

        let secret_selections = MultiSelect::with_theme(&ctx.theme())
            .with_prompt("Mark as secret (space to toggle, enter to confirm)")
            .items(&secret_items)
            .defaults(&defaults)
            .interact()?;

        for sel_idx in secret_selections {
            secrets_to_push.insert(secret_candidates[sel_idx].1.to_string());
        }
    }

    // Re-print selected changes for confirmation
    println!();
    ctx.print_header(&format!(
        "The following {} change(s) will be pushed to {}:",
        selections.len(),
        esc_env
    ));
    println!();

    for &idx in &selections {
        let change = &changes[idx];
        match change {
            EnvChange::Added(key, value) => {
                let display_value = truncate_value(value, 40);
                let secret_marker = if secrets_to_push.contains(key) {
                    " [secret]"
                } else {
                    ""
                };
                println!(
                    "  \x1b[32m[+] {} = {}{}\x1b[0m",
                    key, display_value, secret_marker
                );
            }
            EnvChange::Modified(key, old_value, new_value) => {
                let old_display = truncate_value(old_value, 30);
                let new_display = truncate_value(new_value, 30);
                let secret_marker = if secrets_to_push.contains(key) {
                    " [secret]"
                } else {
                    ""
                };
                println!(
                    "  \x1b[33m[~] {} : {} → {}{}\x1b[0m",
                    key, old_display, new_display, secret_marker
                );
            }
            EnvChange::Removed(key, old_value) => {
                let display_value = truncate_value(old_value, 40);
                println!("  \x1b[31m[-] {} (was: {})\x1b[0m", key, display_value);
            }
        }
    }
    println!();

    if !ctx.confirm(
        &format!("Push {} change(s) to {}?", selections.len(), esc_env),
        false,
    )? {
        println!("Cancelled.");
        return Ok(());
    }

    apply_changes(ctx, &esc_env, &changes, selections, secrets_to_push)
}

fn truncate_value(value: &str, max_len: usize) -> String {
    if value.len() > max_len {
        format!("{}...", &value[..max_len.saturating_sub(3)])
    } else {
        value.to_string()
    }
}

fn apply_changes(
    ctx: &AppContext,
    esc_env: &str,
    changes: &[EnvChange],
    selected_indices: Vec<usize>,
    secrets: std::collections::HashSet<String>,
) -> Result<()> {
    ctx.print_header(&format!(
        "Pushing {} change(s) to {}...",
        selected_indices.len(),
        esc_env
    ));

    let mut success_count = 0;
    let mut error_count = 0;

    for idx in selected_indices {
        let change = &changes[idx];

        match change {
            EnvChange::Added(key, value) | EnvChange::Modified(key, _, value) => {
                let path = format!("environmentVariables[\"{}\"]", key);
                let is_secret = secrets.contains(key);

                let mut args = vec!["env", "set", esc_env, &path, value.as_str()];
                if is_secret {
                    args.push("--secret");
                }

                let result = CmdBuilder::new("esc").args(args).cwd(&ctx.repo).run();

                match result {
                    Ok(0) => {
                        success_count += 1;
                        let secret_marker = if is_secret { " [secret]" } else { "" };
                        println!("  Set {}{}", key, secret_marker);
                    }
                    Ok(code) => {
                        eprintln!("  Warning: failed to set {} (exit code {})", key, code);
                        error_count += 1;
                    }
                    Err(e) => {
                        eprintln!("  Warning: failed to set {}: {}", key, e);
                        error_count += 1;
                    }
                }
            }
            EnvChange::Removed(key, _) => {
                let path = format!("environmentVariables[\"{}\"]", key);
                let result = CmdBuilder::new("esc")
                    .args(["env", "set", esc_env, &path, "--delete"])
                    .cwd(&ctx.repo)
                    .run();

                match result {
                    Ok(0) => {
                        success_count += 1;
                        println!("  Removed {}", key);
                    }
                    Ok(code) => {
                        eprintln!("  Warning: failed to remove {} (exit code {})", key, code);
                        error_count += 1;
                    }
                    Err(e) => {
                        eprintln!("  Warning: failed to remove {}: {}", key, e);
                        error_count += 1;
                    }
                }
            }
        }
    }

    println!();
    if error_count > 0 {
        ctx.print_success(&format!(
            "Applied {} change(s) ({} failed) to {}",
            success_count, error_count, esc_env
        ));
    } else {
        ctx.print_success(&format!(
            "Applied {} change(s) to {}",
            success_count, esc_env
        ));
    }

    Ok(())
}

#[allow(dead_code)]
/// Interactive menu for environment variable operations
pub fn env_menu(ctx: &AppContext) -> Result<()> {
    env_menu_with_config(ctx, None)
}

/// Interactive menu for environment variable operations with config
pub fn env_menu_with_config(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    // Get available environments from config or use defaults
    let envs: Vec<&str> = config
        .map(|c| {
            c.global
                .environments
                .available
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_else(|| vec!["dev", "prod"]);

    // Build menu items dynamically based on available environments
    let mut items: Vec<String> = Vec::new();
    for env in &envs {
        items.push(format!("Pull {} (.env.{})", env, env));
    }
    if envs.len() > 1 {
        items.push("Pull all".to_string());
    }
    for env in &envs {
        items.push(format!("Push {} (.env.{})", env, env));
    }
    if envs.len() > 1 {
        items.push("Push all".to_string());
    }
    items.push("Set environment variable".to_string());
    items.push("Back".to_string());

    let items_ref: Vec<&str> = items.iter().map(|s| s.as_str()).collect();

    let choice = if ctx.quiet {
        envs.len() // "Pull all" in quiet mode (or first env if only one)
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Environment variables (via esc)")
            .items(&items_ref)
            .default(0)
            .interact()?
    };

    let pull_count = envs.len();
    let pull_all_idx = if envs.len() > 1 {
        pull_count
    } else {
        usize::MAX
    };
    let push_start = if envs.len() > 1 {
        pull_count + 1
    } else {
        pull_count
    };
    let push_all_idx = if envs.len() > 1 {
        push_start + envs.len()
    } else {
        usize::MAX
    };
    let set_idx = if envs.len() > 1 {
        push_all_idx + 1
    } else {
        push_start + envs.len()
    };

    if choice < pull_count {
        // Pull specific env
        let env = envs[choice];
        pull_env_with_config(ctx, config, env, &format!(".env.{}", env))?;
    } else if choice == pull_all_idx {
        // Pull all
        for env in &envs {
            pull_env_with_config(ctx, config, env, &format!(".env.{}", env))?;
        }
    } else if choice >= push_start && choice < push_start + envs.len() {
        // Push specific env
        let env = envs[choice - push_start];
        push_env_with_config(ctx, config, env, &format!(".env.{}", env))?;
    } else if choice == push_all_idx {
        // Push all
        for env in &envs {
            push_env_with_config(ctx, config, env, &format!(".env.{}", env))?;
        }
    } else if choice == set_idx {
        set_env_menu_with_config(ctx, config)?;
    }

    Ok(())
}
