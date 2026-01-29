//! Docker container management commands

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{MultiSelect, Select};

use crate::cmd_builder::CmdBuilder;
use crate::compose::{
    cached_compose_services, invalidate_service_cache, list_compose_running_containers,
    run_docker_compose,
};
use crate::context::AppContext;
use crate::utils::{cmd_exists, docker_compose_program, ensure_docker};

/// Build docker images
pub fn docker_compose_build(
    ctx: &AppContext,
    services: &[String],
    pull: bool,
    no_cache: bool,
) -> Result<()> {
    ensure_docker()?;

    let (prog, base_args) = docker_compose_program()?;
    let mut args = base_args;
    args.push("build".to_string());

    if pull {
        args.push("--pull".to_string());
    }
    if no_cache {
        args.push("--no-cache".to_string());
    }
    args.extend(services.iter().cloned());

    ctx.print_header("Build docker images");
    if !ctx.quiet {
        println!("[docker] {} {}", prog, args.join(" "));
    }

    let code = CmdBuilder::new(&prog).args(&args).cwd(&ctx.repo).run()?;
    if code != 0 {
        return Err(anyhow!("docker compose build exited with code {code}"));
    }

    invalidate_service_cache();
    Ok(())
}

/// Restart docker containers
pub fn docker_compose_restart(ctx: &AppContext, services: &[String]) -> Result<()> {
    ensure_docker()?;

    let (prog, base_args) = docker_compose_program()?;
    let mut args = base_args;
    args.push("restart".to_string());
    args.extend(services.iter().cloned());

    ctx.print_header("Restart docker containers");
    if !ctx.quiet {
        println!("[docker] {} {}", prog, args.join(" "));
    }

    let code = CmdBuilder::new(&prog).args(&args).cwd(&ctx.repo).run()?;
    if code != 0 {
        return Err(anyhow!("docker compose restart exited with code {code}"));
    }
    Ok(())
}

/// Interactive menu for building docker images
#[allow(dead_code)]
pub fn build_images_menu(ctx: &AppContext) -> Result<()> {
    let services = cached_compose_services(&ctx.repo)?;
    if services.is_empty() {
        return Err(anyhow!(
            "No docker compose services found (docker compose config --services)."
        ));
    }

    ctx.print_header("Build docker images");

    let mut items: Vec<String> = vec!["[All services]".to_string()];
    items.extend(services.clone());

    let selected = if ctx.quiet {
        vec![0] // Build all in quiet mode
    } else {
        MultiSelect::with_theme(&ctx.theme())
            .with_prompt("Which services do you want to build?")
            .items(&items)
            .interact()?
    };

    if selected.is_empty() {
        return Ok(());
    }

    let pull = ctx.confirm("Pull newer base images first? (--pull)", false)?;
    let no_cache = ctx.confirm("Disable build cache? (--no-cache)", false)?;

    let build_services: Vec<String> = if selected.contains(&0) {
        vec![]
    } else {
        selected
            .iter()
            .filter_map(|&idx| {
                if idx == 0 {
                    None
                } else {
                    items.get(idx).cloned()
                }
            })
            .collect()
    };

    docker_compose_build(ctx, &build_services, pull, no_cache)
}

/// Interactive menu for restarting containers
pub fn restart_containers_menu(ctx: &AppContext) -> Result<()> {
    let services = cached_compose_services(&ctx.repo)?;
    if services.is_empty() {
        return Err(anyhow!(
            "No docker compose services found (docker compose config --services)."
        ));
    }

    ctx.print_header("Restart docker containers");

    // Use Select for single choice which is more intuitive (like nuke_rebuild_menu)
    let mut items: Vec<String> = vec!["[All services]".to_string()];
    items.extend(services.clone());

    let selected_idx = if ctx.quiet {
        0 // All services in quiet mode
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Which service do you want to restart?")
            .items(&items)
            .default(0)
            .interact()?
    };

    let restart_services: Vec<String> = if selected_idx == 0 {
        vec![] // Empty means all services
    } else {
        vec![items[selected_idx].clone()]
    };

    docker_compose_restart(ctx, &restart_services)
}

/// Check if a container is running
fn is_container_running(container: &str) -> bool {
    let output = std::process::Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", container])
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "true",
        Err(_) => false,
    }
}

/// Attach to a container and show recent logs with auto-reconnect
pub fn attach_container_with_logs(ctx: &AppContext, container: &str) -> Result<()> {
    if !cmd_exists("docker") {
        return Err(anyhow!("docker not found. Install Docker Desktop."));
    }

    ctx.print_header(&format!("Following logs for container: {}", container));

    if !ctx.quiet {
        println!(
            "{}",
            style("Auto-reconnect enabled. Press Ctrl+C to exit.").yellow()
        );
        println!();
    }

    loop {
        // Check if container is running
        if !is_container_running(container) {
            if !ctx.quiet {
                println!(
                    "{}",
                    style("Container not running. Waiting for it to start...").yellow()
                );
            }
            // Wait before retrying
            std::thread::sleep(std::time::Duration::from_secs(2));
            continue;
        }

        // Follow logs with tail
        let code = CmdBuilder::new("docker")
            .args(["logs", "-f", "--tail", "200", container])
            .cwd(&ctx.repo)
            .inherit_io()
            .run()?;

        // Exit code 130 = Ctrl+C, exit normally
        if code == 130 {
            break;
        }

        // Container likely stopped/restarted, show message and retry
        if !ctx.quiet {
            println!();
            println!(
                "{}",
                style("Container disconnected. Reconnecting in 2 seconds...").yellow()
            );
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    Ok(())
}

/// Execute a shell in a running container
pub fn docker_shell(ctx: &AppContext, service: Option<&str>) -> Result<()> {
    if !cmd_exists("docker") {
        return Err(anyhow!("docker not found. Install Docker Desktop."));
    }

    let containers = list_compose_running_containers(&ctx.repo)?;
    if containers.is_empty() {
        return Err(anyhow!("No running containers found."));
    }

    let container = if let Some(name) = service {
        // Find container by service name
        containers
            .iter()
            .find(|c| c.label.contains(name) || c.id.contains(name))
            .ok_or_else(|| anyhow!("Container '{}' not found", name))?
    } else {
        // Interactive selection
        let idx = Select::with_theme(&ctx.theme())
            .with_prompt("Select a container")
            .items(
                &containers
                    .iter()
                    .map(|c| c.label.as_str())
                    .collect::<Vec<_>>(),
            )
            .default(0)
            .interact()?;
        &containers[idx]
    };

    ctx.print_header(&format!("Opening shell in: {}", container.label));

    // Try common shells in order
    let shells = ["bash", "sh", "ash"];
    for shell in shells {
        let code = CmdBuilder::new("docker")
            .args(["exec", "-it", &container.id, shell])
            .cwd(&ctx.repo)
            .inherit_io()
            .run()?;

        // If shell worked (exit 0) or user exited (130 = Ctrl+C), we're done
        if code == 0 || code == 130 {
            return Ok(());
        }
        // If exit code is 126 or 127, shell not found - try next
        if code != 126 && code != 127 {
            return Err(anyhow!("Shell exited with code {}", code));
        }
    }

    Err(anyhow!("No shell found in container"))
}

/// Interactive menu for following container logs
pub fn logs_menu(ctx: &AppContext) -> Result<()> {
    let containers = list_compose_running_containers(&ctx.repo)?;
    if containers.is_empty() {
        ctx.print_warning("No running docker-compose containers found.");
        return Ok(());
    }

    let idx = if ctx.quiet {
        0
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Select a container to follow (auto-reconnects)")
            .items(
                &containers
                    .iter()
                    .map(|c| c.label.as_str())
                    .collect::<Vec<_>>(),
            )
            .default(0)
            .interact()?
    };

    attach_container_with_logs(ctx, &containers[idx].id)
}

/// Build and start docker containers (up -d --build)
pub fn docker_compose_up(ctx: &AppContext, services: &[String], build: bool) -> Result<()> {
    ensure_docker()?;

    let (prog, base_args) = docker_compose_program()?;
    let mut args = base_args;
    args.push("up".to_string());
    args.push("-d".to_string());

    if build {
        args.push("--build".to_string());
    }
    args.extend(services.iter().cloned());

    ctx.print_header("Starting docker containers");
    if !ctx.quiet {
        println!("[docker] {} {}", prog, args.join(" "));
    }

    let code = CmdBuilder::new(&prog).args(&args).cwd(&ctx.repo).run()?;
    if code != 0 {
        return Err(anyhow!("docker compose up exited with code {code}"));
    }

    invalidate_service_cache();
    ctx.print_success("Docker containers started.");
    Ok(())
}

/// Interactive menu for starting containers with optional build
pub fn up_containers_menu(ctx: &AppContext) -> Result<()> {
    let services = cached_compose_services(&ctx.repo)?;
    if services.is_empty() {
        return Err(anyhow!(
            "No docker compose services found (docker compose config --services)."
        ));
    }

    ctx.print_header("Start docker containers (up)");

    let mut items: Vec<String> = vec!["[All services]".to_string()];
    items.extend(services.clone());

    let selected_idx = if ctx.quiet {
        0
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Which service do you want to start?")
            .items(&items)
            .default(0)
            .interact()?
    };

    let up_services: Vec<String> = if selected_idx == 0 {
        vec![]
    } else {
        vec![items[selected_idx].clone()]
    };

    let build = ctx.confirm("Rebuild before starting? (--build)", false)?;

    docker_compose_up(ctx, &up_services, build)
}

/// Stop all docker containers
pub fn stop_docker_containers(ctx: &AppContext) -> Result<()> {
    ensure_docker()?;

    ctx.print_header("Stop docker containers");

    if !ctx.confirm("Run `docker compose stop` now?", true)? {
        return Ok(());
    }

    let code = run_docker_compose(&ctx.repo, &["stop"])?;
    if code != 0 {
        return Err(anyhow!("docker compose exited with code {code}"));
    }

    invalidate_service_cache();
    ctx.print_success("Docker containers stopped.");
    Ok(())
}

/// Get image names for compose services
fn get_service_images(repo: &std::path::Path, services: &[String]) -> Result<Vec<String>> {
    let (prog, base_args) = docker_compose_program()?;
    let mut args = base_args;
    args.push("images".to_string());
    args.push("--format".to_string());
    args.push("{{.Image}}".to_string());
    args.extend(services.iter().cloned());

    let output = std::process::Command::new(&prog)
        .args(&args)
        .current_dir(repo)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let images: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect();

    Ok(images)
}

/// Nuke and rebuild docker images (stop, remove containers, remove images, rebuild)
pub fn docker_nuke_rebuild(ctx: &AppContext, services: &[String]) -> Result<()> {
    ensure_docker()?;

    let (prog, base_args) = docker_compose_program()?;

    ctx.print_header("Nuke and rebuild docker images");

    // Get the image names before we remove containers
    let images = get_service_images(&ctx.repo, services)?;
    if !ctx.quiet {
        if images.is_empty() {
            println!("[docker] No images found for services (will build fresh)");
        } else {
            println!("[docker] Found images: {}", images.join(", "));
        }
    }

    // Step 1: Stop and remove containers in one command (works on running containers)
    if !ctx.quiet {
        println!("[docker] Stopping and removing containers...");
    }
    let mut down_args = base_args.clone();
    down_args.push("rm".to_string());
    down_args.push("-f".to_string());
    down_args.push("-s".to_string()); // Stop containers before removing
    down_args.push("-v".to_string()); // Remove anonymous volumes
    down_args.extend(services.iter().cloned());
    let code = CmdBuilder::new(&prog)
        .args(&down_args)
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;
    if code != 0 {
        ctx.print_warning(&format!(
            "docker compose rm exited with code {code} (continuing anyway)"
        ));
    }

    // Step 2: Remove images directly using docker rmi
    if !images.is_empty() {
        if !ctx.quiet {
            println!("[docker] Removing images...");
        }
        for image in &images {
            let code = CmdBuilder::new("docker")
                .args(["rmi", "-f", image])
                .cwd(&ctx.repo)
                .inherit_io()
                .run()?;
            if code != 0 {
                ctx.print_warning(&format!(
                    "Failed to remove image {image} (may not exist or in use)"
                ));
            }
        }
    }

    // Step 3: Rebuild with --pull and --no-cache
    if !ctx.quiet {
        println!("[docker] Rebuilding from scratch...");
    }
    docker_compose_build(ctx, services, true, true)?;

    invalidate_service_cache();
    ctx.print_success("Docker images nuked and rebuilt.");
    Ok(())
}

/// Interactive menu for nuking and rebuilding docker images
pub fn nuke_rebuild_menu(ctx: &AppContext) -> Result<()> {
    let services = cached_compose_services(&ctx.repo)?;
    if services.is_empty() {
        return Err(anyhow!(
            "No docker compose services found (docker compose config --services)."
        ));
    }

    ctx.print_header("Nuke & rebuild docker images");

    // Show services - use Select for single choice which is more intuitive for this destructive operation
    let mut items: Vec<String> = services.clone();
    items.insert(0, "[All services]".to_string());

    let selected_idx = if ctx.quiet {
        0 // All services in quiet mode
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Which service do you want to nuke & rebuild?")
            .items(&items)
            .default(0)
            .interact()?
    };

    let rebuild_services: Vec<String> = if selected_idx == 0 {
        vec![] // Empty means all services
    } else {
        vec![items[selected_idx].clone()]
    };

    let target_desc = if rebuild_services.is_empty() {
        "ALL services".to_string()
    } else {
        rebuild_services.join(", ")
    };

    if !ctx.confirm(
        &format!("⚠️  This will nuke & rebuild {target_desc}. Continue?"),
        false,
    )? {
        ctx.print_warning("Cancelled.");
        return Ok(());
    }

    docker_nuke_rebuild(ctx, &rebuild_services)
}
