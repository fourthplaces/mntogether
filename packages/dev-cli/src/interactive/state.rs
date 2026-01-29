//! Environment status detection for context-aware menus

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use std::path::Path;

/// Cached status information about the development environment
#[derive(Debug, Default)]
pub struct EnvironmentStatus {
    pub docker: DockerStatus,
    pub git: GitStatus,
    pub migrations: MigrationStatus,
}

/// Docker container status
#[derive(Debug, Default)]
pub struct DockerStatus {
    /// Names of running containers
    pub running: Vec<String>,
    /// Names of stopped containers
    pub stopped: Vec<String>,
    /// Whether Docker daemon is available
    pub available: bool,
}

impl DockerStatus {
    /// Total number of containers
    pub fn total(&self) -> usize {
        self.running.len() + self.stopped.len()
    }

    /// Status summary string (e.g., "3/5 running")
    pub fn summary(&self) -> String {
        if !self.available {
            return "unavailable".to_string();
        }
        if self.total() == 0 {
            return "no containers".to_string();
        }
        format!("{}/{} running", self.running.len(), self.total())
    }
}

/// Git repository status
#[derive(Debug, Default)]
pub struct GitStatus {
    /// Current branch name
    pub branch: String,
    /// Commits ahead of remote
    pub ahead: u32,
    /// Commits behind remote
    pub behind: u32,
    /// Number of uncommitted changes
    pub uncommitted: u32,
    /// Whether this is a git repository
    pub is_repo: bool,
}

impl GitStatus {
    /// Status summary string
    pub fn summary(&self) -> String {
        if !self.is_repo {
            return "not a repo".to_string();
        }
        let mut parts = vec![self.branch.clone()];
        if self.ahead > 0 {
            parts.push(format!("↑{}", self.ahead));
        }
        if self.behind > 0 {
            parts.push(format!("↓{}", self.behind));
        }
        if self.uncommitted > 0 {
            parts.push(format!("{}∆", self.uncommitted));
        }
        parts.join(" ")
    }
}

/// Database migration status
#[derive(Debug, Default)]
pub struct MigrationStatus {
    /// Number of pending migrations
    pub pending: u32,
    /// Whether the database is reachable
    pub db_available: bool,
}

impl MigrationStatus {
    /// Status summary string
    pub fn summary(&self) -> String {
        if !self.db_available {
            return "db unavailable".to_string();
        }
        if self.pending == 0 {
            "up to date".to_string()
        } else {
            format!("{} pending", self.pending)
        }
    }
}

impl EnvironmentStatus {
    /// Collect current environment status
    pub fn collect(ctx: &AppContext) -> Self {
        Self {
            docker: collect_docker_status(&ctx.repo),
            git: collect_git_status(&ctx.repo),
            migrations: collect_migration_status(&ctx.repo),
        }
    }
}

/// Check Docker status
fn collect_docker_status(repo: &Path) -> DockerStatus {
    // Check if docker is available
    let docker_available = CmdBuilder::new("docker")
        .arg("info")
        .run_capture()
        .map(|out| out.code == 0)
        .unwrap_or(false);

    if !docker_available {
        return DockerStatus {
            available: false,
            ..Default::default()
        };
    }

    // Get running containers
    let running = CmdBuilder::new("docker")
        .args(["compose", "ps", "--services", "--status", "running"])
        .cwd(repo)
        .run_capture()
        .map(|out| out.stdout_lines())
        .unwrap_or_default();

    // Get all containers
    let all = CmdBuilder::new("docker")
        .args(["compose", "ps", "--services"])
        .cwd(repo)
        .run_capture()
        .map(|out| out.stdout_lines())
        .unwrap_or_default();

    // Stopped = all - running
    let stopped: Vec<String> = all
        .iter()
        .filter(|s| !running.contains(s))
        .cloned()
        .collect();

    DockerStatus {
        running,
        stopped,
        available: true,
    }
}

/// Check Git status
fn collect_git_status(repo: &Path) -> GitStatus {
    // Check if it's a git repo
    let is_repo = repo.join(".git").exists();
    if !is_repo {
        return GitStatus::default();
    }

    // Get current branch
    let branch = CmdBuilder::new("git")
        .args(["branch", "--show-current"])
        .cwd(repo)
        .run_capture()
        .map(|out| out.stdout_string().trim().to_string())
        .unwrap_or_default();

    // Get porcelain status for ahead/behind and changes
    let status_output = CmdBuilder::new("git")
        .args(["status", "--porcelain", "-b"])
        .cwd(repo)
        .run_capture()
        .map(|out| out.stdout_string())
        .unwrap_or_default();

    let mut ahead = 0;
    let mut behind = 0;
    let mut uncommitted = 0;

    for (i, line) in status_output.lines().enumerate() {
        if i == 0 {
            // First line contains branch info: ## main...origin/main [ahead 2, behind 1]
            if let Some(bracket_start) = line.find('[') {
                let info = &line[bracket_start..];
                if let Some(ahead_pos) = info.find("ahead ") {
                    let num_str: String = info[ahead_pos + 6..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    ahead = num_str.parse().unwrap_or(0);
                }
                if let Some(behind_pos) = info.find("behind ") {
                    let num_str: String = info[behind_pos + 7..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    behind = num_str.parse().unwrap_or(0);
                }
            }
        } else if !line.is_empty() {
            uncommitted += 1;
        }
    }

    GitStatus {
        branch,
        ahead,
        behind,
        uncommitted,
        is_repo: true,
    }
}

/// Check migration status (simplified - just checks if migrations dir exists)
fn collect_migration_status(repo: &Path) -> MigrationStatus {
    // Check common migration paths
    let migrations_paths = [
        repo.join("packages/api-core/migrations"),
        repo.join("migrations"),
    ];

    let has_migrations = migrations_paths.iter().any(|p| p.exists());

    // For now, we just check if postgres is running
    // A full implementation would query the database
    let db_available = CmdBuilder::new("docker")
        .args([
            "compose",
            "exec",
            "-T",
            "postgres",
            "pg_isready",
            "-U",
            "postgres",
        ])
        .cwd(repo)
        .run_capture()
        .map(|out| out.code == 0)
        .unwrap_or(false);

    MigrationStatus {
        pending: if has_migrations { 0 } else { 0 }, // Simplified for now
        db_available,
    }
}
