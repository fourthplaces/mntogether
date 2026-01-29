//! Menu action types
//!
//! This module defines the MenuAction enum used by both the new interactive
//! system and CLI command dispatch.

/// Actions that can be performed from the menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    // Quick Access
    StartDev,
    StopDev,
    StartMobile,
    Sync,
    // Docker
    DockerUp,
    DockerStop,
    DockerRestart,
    DockerBuild,
    DockerNuke,
    Logs,
    Shell,
    // Database
    DbMenu,
    Migrate,
    // Environment
    EnvMenu,
    // Code Quality
    FmtFix,
    LintFix,
    Check,
    // Build
    BuildMenu,
    // Testing
    RunTests,
    RunCoverage,
    // Watch
    WatchMenu,
    // Git
    Release,
    // Benchmarks
    BenchmarkMenu,
    // Utilities
    OpenUrls,
    Tunnel,
    Status,
    Doctor,
    Init,
    Exit,
    // AI Tasks
    AiTasksMenu,
    // Todo
    TodoMenu,
}
