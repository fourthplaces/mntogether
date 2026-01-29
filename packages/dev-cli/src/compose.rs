//! Docker Compose utilities and service caching

use anyhow::Result;
use std::cell::RefCell;
use std::path::Path;

use crate::cmd_builder::CmdBuilder;
use crate::utils::docker_compose_program;

// =============================================================================
// Service Cache
// =============================================================================

thread_local! {
    static SERVICE_CACHE: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Get compose services with caching
pub fn cached_compose_services(repo: &Path) -> Result<Vec<String>> {
    SERVICE_CACHE.with(|cache| {
        if let Some(ref services) = *cache.borrow() {
            return Ok(services.clone());
        }
        let services = list_compose_services_uncached(repo)?;
        *cache.borrow_mut() = Some(services.clone());
        Ok(services)
    })
}

/// Invalidate the service cache (call after starting/stopping containers)
pub fn invalidate_service_cache() {
    SERVICE_CACHE.with(|cache| {
        *cache.borrow_mut() = None;
    });
}

// =============================================================================
// Container/Service Queries
// =============================================================================

#[derive(Debug, Clone)]
pub struct ComposeContainerChoice {
    pub label: String,
    pub id: String,
}

/// List running containers from docker compose
pub fn list_compose_running_containers(repo: &Path) -> Result<Vec<ComposeContainerChoice>> {
    let (prog, base_args) = docker_compose_program()?;

    let mut args = base_args.clone();
    args.extend(["ps", "--services", "--filter", "status=running"].map(String::from));

    let out = CmdBuilder::new(&prog)
        .args(&args)
        .cwd(repo)
        .capture_stdout()
        .run_capture()?;

    let services = out.stdout_lines();
    let mut choices: Vec<ComposeContainerChoice> = Vec::new();

    for svc in services {
        let mut args2 = base_args.clone();
        args2.extend(["ps", "-q"].map(String::from));
        args2.push(svc.clone());

        let out2 = CmdBuilder::new(&prog)
            .args(&args2)
            .cwd(repo)
            .capture_stdout()
            .run_capture()?;

        for id in out2.stdout_lines() {
            let short = id.chars().take(12).collect::<String>();
            choices.push(ComposeContainerChoice {
                label: format!("{svc} ({short})"),
                id,
            });
        }
    }

    choices.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(choices)
}

/// List all services defined in docker-compose.yml (uncached)
pub fn list_compose_services_uncached(repo: &Path) -> Result<Vec<String>> {
    let (prog, base_args) = docker_compose_program()?;

    let mut args = base_args;
    args.extend(["config", "--services"].map(String::from));

    let out = CmdBuilder::new(&prog)
        .args(&args)
        .cwd(repo)
        .capture_stdout()
        .run_capture()?;

    let mut svcs = out.stdout_lines();
    svcs.sort();
    Ok(svcs)
}

/// Run docker compose with given args
pub fn run_docker_compose(repo: &Path, args: &[&str]) -> Result<i32> {
    let (prog, base_args) = docker_compose_program()?;
    CmdBuilder::new(&prog)
        .args(base_args)
        .args(args.iter().map(|s| s.to_string()))
        .cwd(repo)
        .run()
}
