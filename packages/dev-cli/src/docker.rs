//! Docker command execution — all async via tokio::process::Command.

use std::collections::HashMap;

use tokio::process::Command;

use crate::app::{ServiceState, Status};
use crate::services::{ServiceId, SERVICES};

// ── Refresh result ──────────────────────────────────────────────────

#[derive(Debug)]
pub struct RefreshResult {
    pub states: HashMap<ServiceId, ServiceState>,
}

// ── Engine check ────────────────────────────────────────────────────

pub async fn check_docker_engine() -> bool {
    Command::new("docker")
        .args(["info"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

// ── Full status refresh (runs concurrently) ─────────────────────────

pub async fn refresh_all() -> RefreshResult {
    // Kick off CPU stats and all container checks in parallel
    let cpu_handle = tokio::spawn(fetch_cpu_stats());

    let mut status_handles = Vec::new();
    for svc in SERVICES {
        let container = svc.container.to_string();
        let port = svc.port;
        let id = svc.id;
        status_handles.push(tokio::spawn(async move {
            let (status, hint) = get_service_status(&container, port).await;
            (id, status, hint)
        }));
    }

    let cpu_map = cpu_handle.await.unwrap_or_default();

    let mut states = HashMap::new();
    for handle in status_handles {
        if let Ok((id, status, hint)) = handle.await {
            let svc = SERVICES.iter().find(|s| s.id == id).unwrap();
            let cpu = cpu_map.get(svc.container).cloned();
            states.insert(id, ServiceState { status, cpu, hint });
        }
    }

    RefreshResult { states }
}

// ── CPU stats ───────────────────────────────────────────────────────

async fn fetch_cpu_stats() -> HashMap<String, String> {
    let output = Command::new("docker")
        .args(["stats", "--no-stream", "--format", "{{.Name}}\t{{.CPUPerc}}"])
        .output()
        .await;

    let mut map = HashMap::new();
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if let Some((name, cpu)) = line.split_once('\t') {
                map.insert(name.to_string(), cpu.to_string());
            }
        }
    }
    map
}

// ── Container status detection ──────────────────────────────────────

async fn container_inspect_state(container: &str) -> String {
    let output = Command::new("docker")
        .args(["inspect", "--format", "{{.State.Status}}", container])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => "stopped".to_string(),
    }
}

async fn container_inspect_health(container: &str) -> String {
    let output = Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}",
            container,
        ])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => "none".to_string(),
    }
}

async fn is_port_listening(port: u16) -> bool {
    Command::new("lsof")
        .args(["-i", &format!(":{port}"), "-sTCP:LISTEN"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Synthesize a display status from container state + health + port.
/// Matches the exact logic from the original dev.sh get_status().
async fn get_service_status(container: &str, port: u16) -> (Status, Option<String>) {
    let state = container_inspect_state(container).await;

    match state.as_str() {
        "running" => {
            let health = container_inspect_health(container).await;
            match health.as_str() {
                "healthy" => (Status::Ok, None),
                "starting" => (Status::Starting, Some("Health check pending...".into())),
                "unhealthy" => {
                    let svc_name = container
                        .strip_prefix("rooteditorial_")
                        .unwrap_or(container);
                    (
                        Status::Fail,
                        Some(format!("Unhealthy — docker compose logs {svc_name}")),
                    )
                }
                // No health check defined — fall back to port
                _ => {
                    if is_port_listening(port).await {
                        (Status::Ok, None)
                    } else {
                        (
                            Status::Starting,
                            Some(format!("Waiting for port {port}...")),
                        )
                    }
                }
            }
        }
        "exited" | "stopped" => {
            if is_port_listening(port).await {
                (Status::Ok, Some("(local process)".into()))
            } else {
                (Status::Stopped, None)
            }
        }
        _ => (Status::Stopped, None),
    }
}

// ── Docker compose operations ───────────────────────────────────────

pub async fn compose_up(repo_root: &str, services: &[&str]) -> anyhow::Result<String> {
    let mut args = vec!["compose", "up", "-d"];
    if services.is_empty() {
        args.push("--remove-orphans");
    }
    args.extend_from_slice(services);
    run_compose(repo_root, &args).await
}

pub async fn compose_up_build(repo_root: &str, services: &[&str]) -> anyhow::Result<String> {
    let mut args = vec!["compose", "up", "-d", "--build"];
    args.extend_from_slice(services);
    run_compose(repo_root, &args).await
}

pub async fn compose_stop(repo_root: &str, services: &[&str]) -> anyhow::Result<String> {
    let mut args = vec!["compose", "stop"];
    args.extend_from_slice(services);
    run_compose(repo_root, &args).await
}

pub async fn compose_down(repo_root: &str) -> anyhow::Result<String> {
    run_compose(repo_root, &["compose", "down"]).await
}

pub async fn compose_restart(repo_root: &str, services: &[&str]) -> anyhow::Result<String> {
    let mut args = vec!["compose", "restart"];
    args.extend_from_slice(services);
    run_compose(repo_root, &args).await
}

/// Spawn `docker compose logs -f` as a child process (takes over the terminal).
pub async fn compose_logs(
    repo_root: &str,
    services: &[&str],
) -> anyhow::Result<tokio::process::Child> {
    let mut cmd = Command::new("docker");
    cmd.args(["compose", "logs", "-f", "--tail", "50"]);
    for svc in services {
        cmd.arg(svc);
    }
    cmd.current_dir(repo_root);
    Ok(cmd.spawn()?)
}

/// Full database reset: drop → create → migrate → seed.
pub async fn reset_database(repo_root: &str) -> anyhow::Result<()> {
    // Drop + create
    run_compose(
        repo_root,
        &[
            "compose", "exec", "-T", "postgres", "psql", "-U", "postgres",
            "-c", "DROP DATABASE IF EXISTS rooteditorial;",
            "-c", "CREATE DATABASE rooteditorial;",
        ],
    )
    .await?;

    // Migrate
    run_compose(
        repo_root,
        &[
            "compose", "exec", "server", "sqlx", "migrate", "run",
            "--source", "/app/packages/server/migrations",
        ],
    )
    .await?;

    // Seed — pipe node output into psql
    let seed = std::process::Command::new("node")
        .arg("data/seed.mjs")
        .current_dir(repo_root)
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let seed_stdout = seed.stdout.expect("piped stdout");

    let psql = Command::new("docker")
        .args([
            "compose", "exec", "-T", "postgres", "psql", "-U", "postgres", "-d",
            "rooteditorial",
        ])
        .current_dir(repo_root)
        .stdin(seed_stdout)
        .output()
        .await?;

    if !psql.status.success() {
        anyhow::bail!(
            "Seed failed: {}",
            String::from_utf8_lossy(&psql.stderr)
        );
    }

    Ok(())
}

// ── Internal ────────────────────────────────────────────────────────

async fn run_compose(repo_root: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("docker")
        .args(args)
        .current_dir(repo_root)
        .output()
        .await?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    if !output.status.success() {
        anyhow::bail!("docker {} failed:\n{}", args.join(" "), combined);
    }

    Ok(combined)
}
