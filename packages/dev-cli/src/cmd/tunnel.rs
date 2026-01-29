//! HTTP tunnel commands

use anyhow::{anyhow, Result};

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Start HTTP tunnel via ngrok
pub fn start_http_tunnel(ctx: &AppContext) -> Result<()> {
    if !cmd_exists("ngrok") {
        return Err(anyhow!(
            "ngrok not found. Install ngrok to start the HTTP tunnel."
        ));
    }

    let config = ctx
        .repo
        .join("packages")
        .join("mobile-app")
        .join("ngrok.yml");
    if !config.exists() {
        return Err(anyhow!("Expected ngrok config at {}", config.display()));
    }

    ctx.print_header("Starting ngrok endpoints...");

    let code = CmdBuilder::new("ngrok")
        .args([
            "start",
            "--all",
            "--config",
            config.to_string_lossy().as_ref(),
        ])
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("ngrok exited with code {code}"));
    }
    Ok(())
}
