//! Houston development commands
//!
//! Start the Houston monitoring dashboard for development.

use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Start Houston for development
///
/// Runs both frontend and backend with hot reload:
/// - houston-web: vite dev server with HMR (port 5173)
/// - houston-server: cargo watch for auto-restart (port 8080)
///
/// The vite dev server proxies API/WS requests to houston-server.
pub fn start_houston(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Starting Houston");

    // Check prerequisites
    if !cmd_exists("cargo-watch") {
        return Err(anyhow!(
            "cargo-watch not found. Install with: cargo install cargo-watch"
        ));
    }

    let houston_web = ctx.repo.join("dev/houston-web");

    // Check node_modules
    if !houston_web.join("node_modules").exists() {
        ctx.print_info("Installing houston-web dependencies...");
        let code = CmdBuilder::new("npm")
            .arg("install")
            .cwd(&houston_web)
            .run()?;
        if code != 0 {
            return Err(anyhow!("npm install failed"));
        }
    }

    println!();
    println!("Starting Houston:");
    println!("  Frontend: http://localhost:5173 (with HMR)");
    println!("  Backend:  http://localhost:8080");
    println!();
    println!("Press Ctrl+C to stop.");
    println!();

    // Use a flag to coordinate shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .ok();

    // Start vite dev server in background
    let mut web_child = Command::new("npm")
        .args(["run", "dev"])
        .current_dir(&houston_web)
        .stdin(Stdio::null())
        .spawn()?;

    // Start cargo watch for server from repo root (watches all workspace deps)
    let mut server_child = Command::new("cargo")
        .args(["watch", "-x", "run -p houston-server"])
        .current_dir(&ctx.repo)
        .stdin(Stdio::null())
        .spawn()?;

    // Wait for either process to exit or Ctrl+C
    while running.load(Ordering::SeqCst) {
        if let Ok(Some(_)) = web_child.try_wait() {
            ctx.print_warning("Frontend process exited");
            break;
        }
        if let Ok(Some(_)) = server_child.try_wait() {
            ctx.print_warning("Server process exited");
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }

    // Cleanup
    ctx.print_info("Stopping services...");
    let _ = web_child.kill();
    let _ = server_child.kill();
    let _ = web_child.wait();
    let _ = server_child.wait();

    ctx.print_success("Houston stopped.");
    Ok(())
}
