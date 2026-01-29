//! Command runner for package-defined commands
//!
//! Runs commands defined in package dev.toml files:
//! ```toml
//! [cmd]
//! build = "npx tsc"
//! test = "npx jest"
//!
//! [cmd.typecheck]
//! run = "npx tsc --noEmit"
//! deps = ["common-js:build"]
//!
//! [cmd.fmt]
//! run = "npx prettier --check src/**/*.{ts,tsx}"
//! fix = "npx prettier --write src/**/*.{ts,tsx}"
//! ```

use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::config::Config;
use crate::context::AppContext;

/// Options for running commands
#[derive(Debug, Default)]
pub struct CmdOptions {
    /// Run commands in parallel where possible
    pub parallel: bool,
    /// Variant to use (e.g., "fix", "watch")
    pub variant: Option<String>,
    /// Only run for specific packages
    pub packages: Vec<String>,
    /// Capture output instead of streaming
    pub capture: bool,
}

/// Result of running a command
#[derive(Debug)]
pub struct CmdResult {
    pub package: String,
    pub cmd_name: String,
    pub success: bool,
    pub output: Option<String>,
}

/// Run a command across all packages that define it
pub fn run_cmd(
    ctx: &AppContext,
    config: &Config,
    cmd_name: &str,
    opts: &CmdOptions,
) -> Result<Vec<CmdResult>> {
    // Find all packages with this command
    let packages = config.packages_with_cmd(cmd_name);

    if packages.is_empty() {
        return Err(anyhow!(
            "No packages define the '{}' command.\n\
             Add it to a package's dev.toml:\n\n\
             [cmd]\n\
             {} = \"your command here\"",
            cmd_name,
            cmd_name
        ));
    }

    // Filter to specific packages if requested
    let packages: Vec<_> = if opts.packages.is_empty() {
        packages
    } else {
        packages
            .into_iter()
            .filter(|(name, _, _)| opts.packages.iter().any(|p| p == *name))
            .collect()
    };

    if packages.is_empty() {
        return Err(anyhow!(
            "None of the specified packages define the '{}' command",
            cmd_name
        ));
    }

    // Build dependency graph and execution order
    let order = resolve_execution_order(config, cmd_name, &packages)?;

    if opts.parallel {
        run_parallel(ctx, config, cmd_name, &order, opts)
    } else {
        run_sequential(ctx, config, cmd_name, &order, opts)
    }
}

/// Resolve execution order respecting dependencies
fn resolve_execution_order<'a>(
    config: &'a Config,
    cmd_name: &str,
    packages: &[(
        &'a str,
        &'a crate::config::PackageConfig,
        &'a crate::config::CmdEntry,
    )],
) -> Result<
    Vec<(
        &'a str,
        &'a crate::config::PackageConfig,
        &'a crate::config::CmdEntry,
    )>,
> {
    let mut result = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut in_stack: HashSet<String> = HashSet::new();

    // Create a map for quick lookup
    let pkg_map: HashMap<&str, _> = packages.iter().map(|p| (p.0, (p.1, p.2))).collect();

    fn visit<'a>(
        pkg_name: &'a str,
        cmd_name: &str,
        config: &'a Config,
        pkg_map: &HashMap<
            &'a str,
            (
                &'a crate::config::PackageConfig,
                &'a crate::config::CmdEntry,
            ),
        >,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        result: &mut Vec<(
            &'a str,
            &'a crate::config::PackageConfig,
            &'a crate::config::CmdEntry,
        )>,
    ) -> Result<()> {
        if visited.contains(pkg_name) {
            return Ok(());
        }

        if in_stack.contains(pkg_name) {
            return Err(anyhow!(
                "Circular dependency detected involving '{}'",
                pkg_name
            ));
        }

        in_stack.insert(pkg_name.to_string());

        // Get the command entry to check dependencies
        if let Some(cmd) = config.get_cmd(pkg_name, cmd_name) {
            for dep in cmd.deps() {
                // Parse "package:cmd" or "package" (defaults to same cmd)
                let (dep_pkg, dep_cmd) = if dep.contains(':') {
                    let parts: Vec<_> = dep.split(':').collect();
                    (parts[0], parts[1])
                } else {
                    (dep.as_str(), cmd_name)
                };

                // Run the dependency command first
                if let Some(dep_entry) = config.get_cmd(dep_pkg, dep_cmd) {
                    if let Some(dep_pkg_config) = config.get_package(dep_pkg) {
                        // Get the package name with the right lifetime from config
                        if let Some((pkg_key, _)) =
                            config.packages.iter().find(|(k, _)| k.as_str() == dep_pkg)
                        {
                            visit(
                                pkg_key.as_str(),
                                dep_cmd,
                                config,
                                pkg_map,
                                visited,
                                in_stack,
                                result,
                            )?;
                            // Add dependency to result if not already there
                            if !result.iter().any(|(n, _, _)| *n == dep_pkg) {
                                result.push((pkg_key.as_str(), dep_pkg_config, dep_entry));
                            }
                        }
                    }
                }
            }
        }

        in_stack.remove(pkg_name);
        visited.insert(pkg_name.to_string());

        // Add this package if it's in our target list
        if let Some((pkg_config, cmd_entry)) = pkg_map.get(pkg_name) {
            if !result.iter().any(|(n, _, _)| *n == pkg_name) {
                result.push((pkg_name, *pkg_config, *cmd_entry));
            }
        }

        Ok(())
    }

    for (pkg_name, _, _) in packages {
        visit(
            pkg_name,
            cmd_name,
            config,
            &pkg_map,
            &mut visited,
            &mut in_stack,
            &mut result,
        )?;
    }

    Ok(result)
}

/// Get command string for the given variant
fn get_cmd_for_variant<'a>(
    cmd_entry: &'a crate::config::CmdEntry,
    variant: Option<&str>,
) -> &'a str {
    match variant {
        Some(v) => cmd_entry.variant(v),
        None => cmd_entry.default_cmd(),
    }
}

/// Run commands sequentially
fn run_sequential(
    ctx: &AppContext,
    _config: &Config,
    cmd_name: &str,
    packages: &[(
        &str,
        &crate::config::PackageConfig,
        &crate::config::CmdEntry,
    )],
    opts: &CmdOptions,
) -> Result<Vec<CmdResult>> {
    let mut results = Vec::new();

    for (pkg_name, pkg_config, cmd_entry) in packages {
        let cmd_str = get_cmd_for_variant(cmd_entry, opts.variant.as_deref());

        if !ctx.quiet {
            println!("[{}] Running {} on {}...", cmd_name, cmd_str, pkg_name);
        }

        let result = run_single_cmd(pkg_name, cmd_name, &pkg_config.path, cmd_str, opts.capture)?;
        let success = result.success;
        results.push(result);

        if !success && !opts.capture {
            // Fail fast in sequential mode unless capturing
            break;
        }
    }

    Ok(results)
}

/// Run commands in parallel where dependencies allow
fn run_parallel(
    ctx: &AppContext,
    _config: &Config,
    cmd_name: &str,
    packages: &[(
        &str,
        &crate::config::PackageConfig,
        &crate::config::CmdEntry,
    )],
    opts: &CmdOptions,
) -> Result<Vec<CmdResult>> {
    // For now, simple parallel execution (ignoring dep ordering for parallel)
    // TODO: Implement proper parallel execution with dependency graph
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for (pkg_name, pkg_config, cmd_entry) in packages {
        let cmd_str = get_cmd_for_variant(cmd_entry, opts.variant.as_deref());

        if !ctx.quiet {
            println!("[{}] Starting {} on {}...", cmd_name, cmd_str, pkg_name);
        }

        let pkg_name = pkg_name.to_string();
        let cmd_name = cmd_name.to_string();
        let path = pkg_config.path.clone();
        let cmd_str = cmd_str.to_string();
        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let result = run_single_cmd(&pkg_name, &cmd_name, &path, &cmd_str, true)
                .unwrap_or_else(|e| CmdResult {
                    package: pkg_name.clone(),
                    cmd_name: cmd_name.clone(),
                    success: false,
                    output: Some(e.to_string()),
                });
            results.lock().unwrap().push(result);
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.join().map_err(|_| anyhow!("Thread panicked"))?;
    }

    let results = Arc::try_unwrap(results)
        .map_err(|_| anyhow!("Failed to unwrap results"))?
        .into_inner()
        .map_err(|_| anyhow!("Failed to get results"))?;

    Ok(results)
}

/// Run a single command
fn run_single_cmd(
    pkg_name: &str,
    cmd_name: &str,
    cwd: &std::path::Path,
    cmd_str: &str,
    capture: bool,
) -> Result<CmdResult> {
    // Parse command string into program and args
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow!("Empty command for {} in {}", cmd_name, pkg_name));
    }

    let program = parts[0];
    let args = &parts[1..];

    let mut cmd = Command::new(program);
    cmd.args(args).current_dir(cwd);

    if capture {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    let output = cmd.output()?;
    let success = output.status.success();

    let output_str = if capture {
        let mut s = String::from_utf8_lossy(&output.stdout).to_string();
        s.push_str(&String::from_utf8_lossy(&output.stderr));
        Some(s)
    } else {
        None
    };

    Ok(CmdResult {
        package: pkg_name.to_string(),
        cmd_name: cmd_name.to_string(),
        success,
        output: output_str,
    })
}

/// Print results summary
pub fn print_results(ctx: &AppContext, results: &[CmdResult]) {
    let succeeded: Vec<_> = results.iter().filter(|r| r.success).collect();
    let failed: Vec<_> = results.iter().filter(|r| !r.success).collect();

    println!();

    if !failed.is_empty() {
        ctx.print_warning(&format!("{} package(s) failed:", failed.len()));
        for result in &failed {
            println!("  - {}", result.package);
            if let Some(output) = &result.output {
                // Print first few lines of output
                for line in output.lines().take(10) {
                    println!("    {}", line);
                }
                if output.lines().count() > 10 {
                    println!("    ... (truncated)");
                }
            }
        }
    }

    if !succeeded.is_empty() && !ctx.quiet {
        ctx.print_success(&format!(
            "{} package(s) succeeded: {}",
            succeeded.len(),
            succeeded
                .iter()
                .map(|r| r.package.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
}

/// List all available commands across packages
pub fn list_commands(config: &Config) -> HashMap<String, Vec<String>> {
    let mut commands: HashMap<String, Vec<String>> = HashMap::new();

    for (pkg_name, pkg_config) in &config.packages {
        for cmd_name in pkg_config.cmd.keys() {
            commands
                .entry(cmd_name.clone())
                .or_default()
                .push(pkg_name.clone());
        }
    }

    commands
}
