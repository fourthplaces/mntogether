//! Performance benchmarking commands

use anyhow::{anyhow, Result};
use dialoguer::Select;
use std::path::PathBuf;
use std::time::Instant;

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::services;
use crate::utils::cmd_exists;

/// Find the server package from config (for benchmarks)
fn find_server_package(config: &Config) -> Option<(String, PathBuf)> {
    config
        .packages
        .values()
        .find(|pkg| pkg.release_type.as_deref() == Some("server"))
        .map(|pkg| (pkg.name.clone(), pkg.path.clone()))
}

/// Run API benchmarks
pub fn run_api_benchmarks(
    ctx: &AppContext,
    config: Option<&Config>,
    filter: Option<&str>,
) -> Result<()> {
    ctx.print_header("Running API Benchmarks");

    let (_, api_dir) = match config {
        Some(cfg) => {
            find_server_package(cfg).ok_or_else(|| anyhow!("No server package found in config"))?
        }
        None => {
            let cfg = Config::load(&ctx.repo)?;
            find_server_package(&cfg).ok_or_else(|| anyhow!("No server package found in config"))?
        }
    };

    let mut args = vec!["bench"];

    if let Some(f) = filter {
        args.push("--");
        args.push(f);
    }

    CmdBuilder::new("cargo").args(args).cwd(&api_dir).run()?;

    Ok(())
}

#[allow(dead_code)]
/// Run load tests against the API
pub fn run_load_test(
    ctx: &AppContext,
    endpoint: Option<&str>,
    requests: u32,
    concurrency: u32,
) -> Result<()> {
    run_load_test_with_config(ctx, None, endpoint, requests, concurrency)
}

/// Run load tests against the API with config
pub fn run_load_test_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    endpoint: Option<&str>,
    requests: u32,
    concurrency: u32,
) -> Result<()> {
    // Check for a load testing tool
    let tool = if cmd_exists("hey") {
        "hey"
    } else if cmd_exists("wrk") {
        "wrk"
    } else if cmd_exists("ab") {
        "ab"
    } else {
        return Err(anyhow!(
            "No load testing tool found. Install one of: hey, wrk, ab\n\
             \n\
             Recommended: brew install hey"
        ));
    };

    // Get default API URL from config using service resolution
    let default_url = config
        .map(|c| {
            let default_port = c.global.services.get_port("api", 8080);
            let port =
                services::get_service_port(&ctx.repo, "api", default_port).unwrap_or(default_port);
            format!("http://localhost:{}/health", port)
        })
        .unwrap_or_else(|| "http://localhost:8080/health".to_string());
    let url = endpoint.unwrap_or(&default_url);

    ctx.print_header(&format!("Load Testing: {}", url));
    println!("Tool: {}", tool);
    println!("Requests: {}", requests);
    println!("Concurrency: {}", concurrency);
    println!();

    match tool {
        "hey" => {
            CmdBuilder::new("hey")
                .args([
                    "-n",
                    &requests.to_string(),
                    "-c",
                    &concurrency.to_string(),
                    url,
                ])
                .cwd(&ctx.repo)
                .run()?;
        }
        "wrk" => {
            CmdBuilder::new("wrk")
                .args([
                    "-t",
                    &concurrency.to_string(),
                    "-c",
                    &concurrency.to_string(),
                    "-d",
                    "10s",
                    url,
                ])
                .cwd(&ctx.repo)
                .run()?;
        }
        "ab" => {
            CmdBuilder::new("ab")
                .args([
                    "-n",
                    &requests.to_string(),
                    "-c",
                    &concurrency.to_string(),
                    url,
                ])
                .cwd(&ctx.repo)
                .run()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Quick benchmark of common operations
pub fn quick_benchmark(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    ctx.print_header("Quick Performance Check");
    println!();

    // Get server package name from config
    let server_pkg = match config {
        Some(cfg) => find_server_package(cfg)
            .map(|(name, _)| name)
            .ok_or_else(|| anyhow!("No server package found in config"))?,
        None => {
            let cfg = Config::load(&ctx.repo)?;
            find_server_package(&cfg)
                .map(|(name, _)| name)
                .ok_or_else(|| anyhow!("No server package found in config"))?
        }
    };

    // Benchmark cargo check
    println!("Benchmarking cargo check ({})...", server_pkg);
    let start = Instant::now();
    let _ = CmdBuilder::new("cargo")
        .args(["check", "--package", &server_pkg])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();
    let check_time = start.elapsed();
    println!("  cargo check: {:.2}s", check_time.as_secs_f64());

    // Benchmark cargo build (incremental)
    println!("Benchmarking incremental build...");
    let start = Instant::now();
    let _ = CmdBuilder::new("cargo")
        .args(["build", "--package", &server_pkg])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();
    let build_time = start.elapsed();
    println!("  cargo build: {:.2}s", build_time.as_secs_f64());

    // Benchmark test suite (quick run)
    println!("Benchmarking test suite...");
    let start = Instant::now();
    let _ = CmdBuilder::new("cargo")
        .args(["test", "--package", &server_pkg, "--", "--test-threads=4"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();
    let test_time = start.elapsed();
    println!("  test suite: {:.2}s", test_time.as_secs_f64());

    println!();
    ctx.print_success("Benchmark complete!");
    println!();
    println!("Summary:");
    println!("  Check:  {:.2}s", check_time.as_secs_f64());
    println!("  Build:  {:.2}s", build_time.as_secs_f64());
    println!("  Tests:  {:.2}s", test_time.as_secs_f64());
    println!(
        "  Total:  {:.2}s",
        (check_time + build_time + test_time).as_secs_f64()
    );

    Ok(())
}

#[allow(dead_code)]
/// Interactive benchmark menu
pub fn benchmark_menu(ctx: &AppContext) -> Result<()> {
    benchmark_menu_with_config(ctx, None)
}

/// Interactive benchmark menu with config
pub fn benchmark_menu_with_config(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    let items = vec![
        "Quick benchmark (check + build + test times)",
        "Run cargo benchmarks",
        "Load test API endpoint",
        "Back",
    ];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Select benchmark type")
        .items(&items)
        .default(0)
        .interact()?;

    // Get defaults from config
    let default_requests = config
        .map(|c| c.global.defaults.load_test_requests)
        .unwrap_or(1000);
    let default_concurrency = config.map(|c| c.global.defaults.concurrency).unwrap_or(10);

    // Get default URL from config using service resolution
    let default_url = config
        .map(|c| {
            let default_port = c.global.services.get_port("api", 8080);
            let port =
                services::get_service_port(&ctx.repo, "api", default_port).unwrap_or(default_port);
            format!("http://localhost:{}/health", port)
        })
        .unwrap_or_else(|| "http://localhost:8080/health".to_string());

    match choice {
        0 => quick_benchmark(ctx, config),
        1 => run_api_benchmarks(ctx, config, None),
        2 => {
            let endpoint = if ctx.quiet {
                None
            } else {
                let input: String = dialoguer::Input::with_theme(&ctx.theme())
                    .with_prompt("Endpoint URL")
                    .default(default_url)
                    .interact_text()?;
                Some(input)
            };

            run_load_test_with_config(
                ctx,
                config,
                endpoint.as_deref(),
                default_requests,
                default_concurrency,
            )
        }
        _ => Ok(()),
    }
}
