//! Code coverage commands

use anyhow::{anyhow, Result};
use dialoguer::Select;

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use crate::utils::{
    cmd_exists, ensure_cargo, ensure_cargo_tool, list_rust_packages, open_in_browser,
};

/// Output format for coverage reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageFormat {
    Html,
    Lcov,
    Summary,
}

impl CoverageFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            CoverageFormat::Html => "html",
            CoverageFormat::Lcov => "lcov",
            CoverageFormat::Summary => "summary",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "html" => Some(CoverageFormat::Html),
            "lcov" => Some(CoverageFormat::Lcov),
            "summary" => Some(CoverageFormat::Summary),
            _ => None,
        }
    }
}

fn ensure_cargo_llvm_cov(ctx: &AppContext) -> Result<()> {
    ensure_cargo_tool(
        ctx,
        "cargo-llvm-cov",
        "It is required for code coverage.",
        &[],
    )?;

    // Also ensure llvm-tools-preview is installed if we just installed cargo-llvm-cov
    if !cmd_exists("llvm-cov") {
        ctx.print_header("Installing llvm-tools-preview component...");
        let code = CmdBuilder::new("rustup")
            .args(["component", "add", "llvm-tools-preview"])
            .cwd(&ctx.repo)
            .run()?;

        if code != 0 {
            ctx.print_warning(
                "Failed to install llvm-tools-preview. Coverage may not work correctly.",
            );
        }
    }

    Ok(())
}

/// Run code coverage
pub fn run_coverage(
    ctx: &AppContext,
    package: Option<&str>,
    test_filter: Option<&str>,
    format: CoverageFormat,
    open_report: bool,
) -> Result<()> {
    ensure_cargo()?;
    ensure_cargo_llvm_cov(ctx)?;

    let mut args = vec!["llvm-cov".to_string()];

    // Add package filter if specified
    if let Some(pkg) = package {
        args.push("-p".to_string());
        args.push(pkg.to_string());
    } else {
        args.push("--workspace".to_string());
    }

    // Ignore test files and mocks (same as CI)
    args.push("--ignore-filename-regex".to_string());
    args.push("(tests/|test_|_test\\.rs|mock)".to_string());

    // Add format-specific arguments
    let output_path = match format {
        CoverageFormat::Html => {
            args.push("--html".to_string());
            args.push("--output-dir".to_string());
            args.push("target/coverage/html".to_string());
            Some(ctx.repo.join("target/coverage/html/index.html"))
        }
        CoverageFormat::Lcov => {
            args.push("--lcov".to_string());
            args.push("--output-path".to_string());
            args.push("target/coverage/lcov.info".to_string());
            Some(ctx.repo.join("target/coverage/lcov.info"))
        }
        CoverageFormat::Summary => {
            // Default output is summary to stdout
            None
        }
    };

    // Add test filter if specified
    if let Some(filter) = test_filter {
        args.push("--".to_string());
        args.push("--test".to_string());
        args.push(filter.to_string());
    }

    ctx.print_header(&format!(
        "Generating {} coverage: cargo {}",
        format.as_str(),
        args.join(" ")
    ));

    let code = CmdBuilder::new("cargo")
        .args(&args)
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;

    if code != 0 {
        return Err(anyhow!("cargo llvm-cov exited with code {code}"));
    }

    // Print success message with output location
    if let Some(path) = &output_path {
        ctx.print_success(&format!("Coverage report generated: {}", path.display()));
    }

    // Open HTML report in browser if requested
    if open_report && format == CoverageFormat::Html {
        if let Some(path) = output_path {
            let url = format!("file://{}", path.display());
            ctx.print_header("Opening coverage report in browser...");
            open_in_browser(&url)?;
        }
    }

    Ok(())
}

/// Interactive menu for code coverage
pub fn coverage_menu(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Code Coverage");

    let packages = list_rust_packages(&ctx.repo)?;

    let mut pkg_items: Vec<String> = vec!["[All packages]".to_string()];
    pkg_items.extend(packages.clone());

    let pkg_choice = if ctx.quiet {
        0
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Which package to measure coverage for?")
            .items(&pkg_items)
            .default(0)
            .interact()?
    };

    let selected_package = if pkg_choice == 0 {
        None
    } else {
        Some(packages[pkg_choice - 1].as_str())
    };

    let test_filter = if ctx.quiet {
        None
    } else {
        let filter: String = dialoguer::Input::with_theme(&ctx.theme())
            .with_prompt("Test name filter (leave empty for all)")
            .allow_empty(true)
            .interact_text()?;
        if filter.trim().is_empty() {
            None
        } else {
            Some(filter.trim().to_string())
        }
    };

    let format_items = vec![
        "HTML (visual report)",
        "LCOV (for CI/tools)",
        "Summary (stdout)",
    ];
    let format_choice = if ctx.quiet {
        0
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Output format?")
            .items(&format_items)
            .default(0)
            .interact()?
    };

    let format = match format_choice {
        0 => CoverageFormat::Html,
        1 => CoverageFormat::Lcov,
        _ => CoverageFormat::Summary,
    };

    let open_report = if format == CoverageFormat::Html && !ctx.quiet {
        ctx.confirm("Open HTML report in browser after generation?", true)?
    } else {
        false
    };

    run_coverage(
        ctx,
        selected_package,
        test_filter.as_deref(),
        format,
        open_report,
    )
}
