//! Root Editorial Dev Dashboard — Ratatui TUI for managing Docker services.

mod apikey;
mod app;
mod docker;
mod events;
mod services;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};

use app::App;
use events::AppEvent;
use services::{ServiceId, SERVICES};

// ── CLI ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "dev", about = "Root Editorial Dev Dashboard")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start all services
    Start,
    /// Stop all services
    Stop,
    /// Restart all services
    Restart,
    /// One-shot status check (no TUI)
    Status,
    /// Follow logs for all or a specific service
    Logs {
        /// Docker compose service name (e.g. server, admin-app)
        service: Option<String>,
    },
    /// Manage service-client API keys (Root Signal ingest tokens).
    Apikey {
        #[command(subcommand)]
        cmd: apikey::ApikeyCommand,
    },
}

// ── Entry point ─────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // `apikey` talks to Postgres directly — it doesn't need Docker running,
    // and skipping the engine check lets it work from any terminal including
    // CI / remote shells.
    let command = match cli.command {
        Some(Commands::Apikey { cmd }) => return apikey::run(cmd).await,
        other => other,
    };

    let repo_root = find_repo_root()?;

    if !docker::check_docker_engine().await {
        eprintln!("\n  Docker is not running.");
        eprintln!("  Start Docker Desktop and try again.\n");
        std::process::exit(1);
    }

    match command {
        Some(Commands::Apikey { .. }) => unreachable!("handled above"),
        Some(Commands::Start) => {
            println!("  Starting services...");
            docker::compose_up(&repo_root, &[]).await?;
            println!("  Services started. Backend may take 1-2 min to compile on first run.");
        }
        Some(Commands::Stop) => {
            println!("  Stopping services...");
            docker::compose_down(&repo_root).await?;
            println!("  Done.");
        }
        Some(Commands::Restart) => {
            println!("  Restarting services...");
            docker::compose_down(&repo_root).await?;
            docker::compose_up(&repo_root, &[]).await?;
            println!("  Services restarted.");
        }
        Some(Commands::Status) => {
            let result = docker::refresh_all().await;
            print_status_table(&result);
        }
        Some(Commands::Logs { service }) => {
            let svcs: Vec<&str> = match &service {
                Some(s) => vec![s.as_str()],
                None => vec![],
            };
            let mut child = docker::compose_logs(&repo_root, &svcs).await?;
            child.wait().await?;
        }
        None => {
            run_dashboard(&repo_root).await?;
        }
    }

    Ok(())
}

// ── Interactive TUI dashboard ───────────────────────────────────────

async fn run_dashboard(repo_root: &str) -> Result<()> {
    // Auto-start if postgres is stopped
    let initial = docker::refresh_all().await;
    let pg_stopped = initial
        .states
        .get(&ServiceId::Postgres)
        .map(|s| s.status == app::Status::Stopped)
        .unwrap_or(true);

    if pg_stopped {
        println!("  Starting services...");
        docker::compose_up(repo_root, &[]).await?;
        println!("  Launching dashboard...");
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    // Enter TUI mode
    let mut terminal = ratatui::init();
    let mut app = App::new(repo_root.to_string());

    // Load initial state
    let initial = docker::refresh_all().await;
    app.apply_refresh(initial);

    // Start event loop
    let mut events = events::EventLoop::new();
    let tx = events.sender();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        let Some(event) = events.next().await else {
            break;
        };

        match event {
            AppEvent::Key(key) => {
                app.handle_key(key, &tx);
            }
            AppEvent::Tick => {
                app.tick();
            }
            AppEvent::DockerRefresh(result) => {
                app.apply_refresh(result);
            }
            AppEvent::OpComplete { success, message } => {
                app.complete_op(success, message);
            }
            AppEvent::Resize => {
                // Ratatui handles resize on next draw
            }
        }

        if app.should_quit {
            break;
        }

        // Log viewing: temporarily leave TUI, run docker compose logs, then return
        if let Some(svcs) = app.wants_logs.take() {
            ratatui::restore();

            let svc_refs: Vec<&str> = svcs.iter().map(|s| s.as_str()).collect();
            println!("\n  Following logs... (Ctrl+C to return to dashboard)\n");
            if let Ok(mut child) = docker::compose_logs(repo_root, &svc_refs).await {
                let _ = child.wait().await;
            }

            terminal = ratatui::init();
            // Trigger a refresh to get updated state after returning from logs
            let result = docker::refresh_all().await;
            app.apply_refresh(result);
        }
    }

    ratatui::restore();
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────

fn find_repo_root() -> Result<String> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("docker-compose.yml").exists() {
            return Ok(dir.to_string_lossy().to_string());
        }
        if !dir.pop() {
            anyhow::bail!("Could not find repo root (no docker-compose.yml found)");
        }
    }
}

fn print_status_table(result: &docker::RefreshResult) {
    println!();
    println!("  Root Editorial Dev — Status");
    println!("  ─────────────────────────────────────────────");

    for layer in services::Layer::ALL {
        println!();
        println!("  {}", layer.label());

        for svc in SERVICES.iter().filter(|s| s.layer == *layer) {
            let state = result.states.get(&svc.id);
            let (indicator, status_str) = match state.map(|s| s.status) {
                Some(app::Status::Ok) => ("●", " OK "),
                Some(app::Status::Starting) => ("●", " .. "),
                Some(app::Status::Fail) => ("●", "FAIL"),
                Some(app::Status::Stopped) | None => ("○", " -- "),
            };
            let cpu = state
                .and_then(|s| s.cpu.as_deref())
                .unwrap_or("--");
            println!(
                "  {indicator} {status_str} {:18} localhost:{:<5}  cpu: {cpu}",
                svc.label, svc.display_port
            );
        }
    }
    println!();
}
