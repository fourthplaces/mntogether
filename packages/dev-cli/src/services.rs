//! Docker service port resolution utilities.
//!
//! Resolves host ports from container ports using `docker compose port`.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Get the host port for a service's container port.
///
/// Uses `docker compose port <service> <container_port>` to resolve the mapping.
/// Returns the container port as fallback if the service isn't running.
pub fn get_service_port(repo_root: &Path, service: &str, container_port: u16) -> Result<u16> {
    let output = Command::new("docker")
        .args(["compose", "port", service, &container_port.to_string()])
        .current_dir(repo_root)
        .output()
        .context("Failed to run docker compose port")?;

    if !output.status.success() {
        // Service not running, return container port as fallback
        return Ok(container_port);
    }

    // Output format: "0.0.0.0:5432" -> extract host port
    let stdout = String::from_utf8_lossy(&output.stdout);
    let port = stdout
        .trim()
        .split(':')
        .next_back()
        .and_then(|p| p.parse().ok())
        .unwrap_or(container_port);

    Ok(port)
}

/// Get a service URL with resolved host port.
///
/// # Arguments
/// * `repo_root` - Repository root path
/// * `service` - Docker compose service name
/// * `container_port` - Container port to resolve
/// * `path` - URL path (should start with '/')
///
/// # Example
/// ```ignore
/// let url = get_service_url(&repo_root, "api", 8080, "/graphql")?;
/// // Returns: "http://localhost:8080/graphql"
/// ```
pub fn get_service_url(
    repo_root: &Path,
    service: &str,
    container_port: u16,
    path: &str,
) -> Result<String> {
    let port = get_service_port(repo_root, service, container_port)?;
    Ok(format!("http://localhost:{}{}", port, path))
}

/// Check if a docker compose service is running.
pub fn is_service_running(repo_root: &Path, service: &str) -> bool {
    let output = Command::new("docker")
        .args(["compose", "ps", "-q", service])
        .current_dir(repo_root)
        .output();

    match output {
        Ok(out) => !out.stdout.is_empty() && out.status.success(),
        Err(_) => false,
    }
}

/// Get all running docker compose services.
pub fn get_running_services(repo_root: &Path) -> Result<Vec<String>> {
    let output = Command::new("docker")
        .args(["compose", "ps", "--services", "--status", "running"])
        .current_dir(repo_root)
        .output()
        .context("Failed to list running services")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_port_parsing() {
        // Test that port parsing works on typical docker compose output
        let output = "0.0.0.0:5432";
        let port: u16 = output
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);
        assert_eq!(port, 5432);
    }
}
