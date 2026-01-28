use anyhow::{Context, Result};
use colored::Colorize;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, MultiSelect, Select};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<()> {
    let term = Term::stdout();

    // Print banner
    print_banner(&term)?;

    // Check if this is first run (no node_modules, no cargo build)
    let project_root = get_project_root()?;
    let needs_setup = check_needs_setup(&project_root)?;

    if needs_setup {
        println!("{}", "🚀 First time setup detected!".bright_green().bold());
        println!();
        run_initial_setup(&project_root)?;
    }

    // Main interactive loop
    loop {
        println!();
        let options = vec![
            "🌐 Start web app",
            "🐳 Docker start",
            "🔄 Docker restart",
            "🔨 Docker rebuild",
            "📋 Follow docker logs",
            "🗄️  Run database migrations",
            "🔑 Check API keys status",
            "📝 Setup environment variables (wizard)",
            "🚀 Manage Fly.io environment variables",
            "👤 Manage admin users",
            "📊 Open GraphQL Playground",
            "🚁 Deploy to Fly.io",
            "🛑 Exit",
        ];

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do? (type to filter)")
            .items(&options)
            .default(0)
            .interact_on(&term)?;

        match selection {
            0 => start_web_app(&project_root)?,
            1 => docker_start(&project_root)?,
            2 => docker_restart(&project_root)?,
            3 => docker_rebuild(&project_root)?,
            4 => docker_logs(&project_root)?,
            5 => run_migrations(&project_root)?,
            6 => check_api_keys(&project_root)?,
            7 => setup_env_wizard(&project_root)?,
            8 => manage_flyctl_secrets()?,
            9 => manage_admin_users(&project_root)?,
            10 => open_graphql_playground()?,
            11 => deploy_to_flyio(&project_root)?,
            12 => {
                println!("{}", "👋 Goodbye!".bright_blue());
                break;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn print_banner(term: &Term) -> Result<()> {
    term.clear_screen()?;
    println!(
        "{}",
        "╔══════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║  Minnesota Digital Aid Dev CLI       ║".bright_cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════╝".bright_cyan()
    );
    println!();
    Ok(())
}

fn get_project_root() -> Result<PathBuf> {
    // Get the directory containing Cargo.toml at workspace root
    let current_exe = env::current_exe()?;
    let mut path = current_exe
        .parent()
        .context("Failed to get parent directory")?
        .to_path_buf();

    // Navigate up to find workspace root (contains Cargo.toml with [workspace])
    loop {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let contents = std::fs::read_to_string(&cargo_toml)?;
            if contents.contains("[workspace]") {
                return Ok(path);
            }
        }

        if !path.pop() {
            break;
        }
    }

    // Fallback to current directory
    env::current_dir().context("Failed to get current directory")
}

fn check_needs_setup(project_root: &PathBuf) -> Result<bool> {
    // Check if node_modules exists in web-app package
    let web_app_node_modules = project_root.join("packages/web-app/node_modules");

    Ok(!web_app_node_modules.exists())
}

fn run_initial_setup(project_root: &PathBuf) -> Result<()> {
    println!("{}", "Checking dependencies...".bright_yellow());

    // Check for required tools
    check_dependency("cargo", "Rust")?;
    check_dependency("docker", "Docker")?;
    check_dependency("node", "Node.js")?;
    check_dependency("npm", "npm")?;

    println!();
    println!("{}", "Installing dependencies...".bright_yellow());
    println!();

    // Install web-app dependencies
    println!("{}", "📦 Installing web-app dependencies...".bright_blue());
    let web_app_dir = project_root.join("packages/web-app");
    run_command("yarn", &["install"], &web_app_dir)?;

    // Install admin-spa dependencies
    println!("{}", "📦 Installing admin-spa dependencies...".bright_blue());
    let admin_spa_dir = project_root.join("packages/admin-spa");
    run_command("yarn", &["install"], &admin_spa_dir)?;

    // Build Rust workspace
    println!("{}", "🦀 Building Rust workspace...".bright_blue());
    run_command("cargo", &["build"], project_root)?;

    println!();
    println!("{}", "✅ Setup complete!".bright_green().bold());

    Ok(())
}

fn check_dependency(cmd: &str, name: &str) -> Result<()> {
    match which::which(cmd) {
        Ok(_) => {
            println!("  {} {}", "✓".bright_green(), name);
            Ok(())
        }
        Err(_) => {
            println!("  {} {} is not installed", "✗".bright_red(), name);
            Err(anyhow::anyhow!(
                "{} is required but not found. Please install it first.",
                name
            ))
        }
    }
}

fn run_command(cmd: &str, args: &[&str], cwd: &PathBuf) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .context(format!("Failed to execute: {} {:?}", cmd, args))?;

    if !status.success() {
        return Err(anyhow::anyhow!("Command failed: {} {:?}", cmd, args));
    }

    Ok(())
}

fn start_web_app(project_root: &PathBuf) -> Result<()> {
    println!("{}", "🌐 Starting web app...".bright_blue().bold());
    println!("{}", "   Press Ctrl+C to stop".dimmed());
    println!();

    let web_app_dir = project_root.join("packages/web-app");

    let status = Command::new("yarn")
        .args(&["dev"])
        .current_dir(&web_app_dir)
        .status()
        .context("Failed to start web app")?;

    if !status.success() {
        println!("{}", "❌ Failed to start web app".bright_red());
    }

    Ok(())
}

fn docker_start(project_root: &PathBuf) -> Result<()> {
    println!("{}", "🐳 Starting Docker services...".bright_blue().bold());

    let server_dir = project_root.join("packages/server");

    let status = Command::new("docker")
        .args(&["compose", "up", "-d"])
        .current_dir(&server_dir)
        .status()
        .context("Failed to start Docker")?;

    if status.success() {
        println!("{}", "✅ Docker services started".bright_green());
        println!();
        println!("Services available at:");
        println!("  {} http://localhost:8080", "API:".bright_yellow());
        println!("  {} localhost:5432", "PostgreSQL:".bright_yellow());
        println!("  {} localhost:6379", "Redis:".bright_yellow());
    } else {
        println!("{}", "❌ Failed to start Docker services".bright_red());
    }

    Ok(())
}

fn docker_restart(project_root: &PathBuf) -> Result<()> {
    println!(
        "{}",
        "🔄 Restarting Docker services...".bright_blue().bold()
    );
    println!();

    let server_dir = project_root.join("packages/server");
    let services = vec!["postgres", "redis", "api"];

    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select services to restart (Space to select, Enter to confirm)")
        .items(&services)
        .defaults(&[true, true, true])
        .interact()?;

    if selections.is_empty() {
        println!("{}", "No services selected".dimmed());
        return Ok(());
    }

    let selected_services: Vec<&str> = selections.iter().map(|&i| services[i]).collect();

    let mut args = vec!["compose", "restart"];
    args.extend(selected_services.clone());

    let status = Command::new("docker")
        .args(&args)
        .current_dir(&server_dir)
        .status()
        .context("Failed to restart Docker")?;

    if status.success() {
        println!(
            "{} {}",
            "✅ Restarted services:".bright_green(),
            selected_services.join(", ")
        );
    } else {
        println!("{}", "❌ Failed to restart Docker services".bright_red());
    }

    Ok(())
}

fn docker_rebuild(project_root: &PathBuf) -> Result<()> {
    println!(
        "{}",
        "🔨 Rebuilding Docker services...".bright_blue().bold()
    );
    println!();

    let server_dir = project_root.join("packages/server");
    let services = vec!["postgres", "redis", "api"];

    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select services to rebuild (Space to select, Enter to confirm)")
        .items(&services)
        .defaults(&[false, false, true]) // Default to rebuilding only API
        .interact()?;

    if selections.is_empty() {
        println!("{}", "No services selected".dimmed());
        return Ok(());
    }

    let selected_services: Vec<&str> = selections.iter().map(|&i| services[i]).collect();

    println!("{}", "   This may take a few minutes...".dimmed());
    println!();

    let mut args = vec!["compose", "up", "-d", "--build"];
    args.extend(selected_services.clone());

    let status = Command::new("docker")
        .args(&args)
        .current_dir(&server_dir)
        .status()
        .context("Failed to rebuild Docker")?;

    if status.success() {
        println!(
            "{} {}",
            "✅ Rebuilt and started services:".bright_green(),
            selected_services.join(", ")
        );
    } else {
        println!("{}", "❌ Failed to rebuild Docker services".bright_red());
    }

    Ok(())
}

fn docker_logs(project_root: &PathBuf) -> Result<()> {
    println!("{}", "📋 Following Docker logs...".bright_blue().bold());
    println!("{}", "   Logs will stream continuously".dimmed());
    println!("{}", "   Press Ctrl+C to stop and return to menu".dimmed());
    println!();

    let server_dir = project_root.join("packages/server");

    // Run docker compose logs with follow flag
    // This will stay attached and stream logs until Ctrl+C
    let status = Command::new("docker")
        .args(&["compose", "logs", "-f", "--tail=100"])
        .current_dir(&server_dir)
        .status()
        .context("Failed to follow Docker logs")?;

    // After Ctrl+C, we return to the menu
    if !status.success() {
        println!();
        println!("{}", "❌ Failed to follow Docker logs".bright_red());
        println!();
        println!("Make sure Docker services are running:");
        println!("  {} 🐳 Docker start", "→".bright_yellow());
    }

    Ok(())
}

fn run_migrations(project_root: &PathBuf) -> Result<()> {
    println!(
        "{}",
        "🗄️  Running database migrations...".bright_blue().bold()
    );

    let server_dir = project_root.join("packages/server");

    let status = Command::new("docker")
        .args(&["compose", "exec", "api", "sqlx", "migrate", "run"])
        .current_dir(&server_dir)
        .status()
        .context("Failed to run migrations")?;

    if status.success() {
        println!("{}", "✅ Migrations completed successfully".bright_green());
    } else {
        println!("{}", "❌ Failed to run migrations".bright_red());
        println!();
        println!("Make sure Docker services are running:");
        println!("  {} 🐳 Docker start", "→".bright_yellow());
    }

    Ok(())
}

fn setup_env_wizard(project_root: &PathBuf) -> Result<()> {
    println!();
    println!("{}", "📝 Environment Variables Setup Wizard".bright_cyan().bold());
    println!();
    println!("This wizard will help you set up all environment variables.");
    println!();

    // Define all variables with their descriptions
    let variables = vec![
        // Required
        ("ANTHROPIC_API_KEY", "Anthropic API key for Claude AI", true, "Get from https://console.anthropic.com"),
        ("VOYAGE_API_KEY", "Voyage AI API key for embeddings", true, "Get from https://www.voyageai.com"),
        ("FIRECRAWL_API_KEY", "Firecrawl API key for web scraping", true, "Get from https://firecrawl.dev"),
        ("TWILIO_ACCOUNT_SID", "Twilio Account SID for SMS", true, "Get from https://console.twilio.com"),
        ("TWILIO_AUTH_TOKEN", "Twilio Auth Token for SMS", true, "Get from https://console.twilio.com"),
        ("TWILIO_VERIFY_SERVICE_SID", "Twilio Verify Service SID", true, "Get from https://console.twilio.com/verify"),
        ("JWT_SECRET", "Secret key for JWT tokens (random string)", true, "Generate a random 32+ character string"),
        // Optional
        ("TAVILY_API_KEY", "Tavily API key for search (optional)", false, "Get from https://tavily.com"),
        ("EXPO_ACCESS_TOKEN", "Expo access token for push notifications (optional)", false, "Get from https://expo.dev"),
        ("CLERK_SECRET_KEY", "Clerk secret key for auth (optional)", false, "Get from https://clerk.com"),
    ];

    // Read existing .env file
    let env_file = project_root.join("packages/server/.env");
    let mut env_values: HashMap<String, String> = HashMap::new();

    if env_file.exists() {
        let content = std::fs::read_to_string(&env_file)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                env_values.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        println!("{}", "✓ Found existing .env file".bright_green());
    } else {
        println!("{}", "ℹ️  No .env file found, will create new one".bright_blue());
    }

    println!();
    println!("{}", "Let's go through each variable...".bright_yellow());
    println!();

    let mut updated = false;
    let mut new_values: HashMap<String, String> = env_values.clone();

    for (key, description, required, help) in &variables {
        let current_value = env_values.get(*key);
        let has_value = current_value.is_some();

        // Print section header
        println!("{}", "─".repeat(60).dimmed());
        if *required {
            println!("{} {}", "🔴".bright_red(), key.bright_cyan().bold());
            println!("   {} {}", "Required:".bright_red(), description);
        } else {
            println!("{} {}", "🟡".bright_yellow(), key.bright_cyan().bold());
            println!("   {} {}", "Optional:".bright_yellow(), description);
        }
        println!("   {} {}", "Help:".dimmed(), help.dimmed());

        if has_value {
            let value = current_value.unwrap();
            let masked = if value.len() > 8 {
                format!("{}...{}", &value[..4], &value[value.len()-4..])
            } else {
                "***".to_string()
            };
            println!("   {} {}", "Current:".bright_green(), masked);
        } else {
            println!("   {} Not set", "Current:".bright_red());
        }

        println!();

        // Ask what to do
        let actions = if has_value {
            vec![
                "Keep current value",
                "Update value",
                "Skip (leave as is)",
            ]
        } else {
            vec![
                "Set value now",
                "Skip (leave empty)",
            ]
        };

        let action = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("What would you like to do with {}?", key))
            .items(&actions)
            .default(0)
            .interact()?;

        match (has_value, action) {
            (true, 0) => {
                // Keep current
                println!("   {} Keeping current value", "✓".bright_green());
            }
            (true, 1) | (false, 0) => {
                // Update or Set
                let new_value: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(format!("Enter value for {}", key))
                    .allow_empty(!*required)
                    .interact_text()?;

                if !new_value.is_empty() {
                    new_values.insert(key.to_string(), new_value);
                    updated = true;
                    println!("   {} Value updated", "✓".bright_green());
                } else {
                    println!("   {} Skipped", "○".dimmed());
                }
            }
            _ => {
                // Skip
                println!("   {} Skipped", "○".dimmed());
            }
        }

        println!();
    }

    println!("{}", "─".repeat(60).dimmed());
    println!();

    // Save to file
    if updated || !env_file.exists() {
        println!("{}", "💾 Saving to .env file...".bright_blue().bold());

        let mut content = String::new();
        content.push_str("# Environment Variables\n");
        content.push_str("# Generated by dev-cli wizard\n\n");

        // Required variables
        content.push_str("# Required Variables\n");
        for (key, _, required, _) in &variables {
            if *required {
                if let Some(value) = new_values.get(*key) {
                    content.push_str(&format!("{}={}\n", key, value));
                } else {
                    content.push_str(&format!("# {}=\n", key));
                }
            }
        }

        content.push_str("\n# Optional Variables\n");
        for (key, _, required, _) in &variables {
            if !*required {
                if let Some(value) = new_values.get(*key) {
                    content.push_str(&format!("{}={}\n", key, value));
                } else {
                    content.push_str(&format!("# {}=\n", key));
                }
            }
        }

        // Create directory if it doesn't exist
        if let Some(parent) = env_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&env_file, content)?;
        println!("{}", "✅ Saved to packages/server/.env".bright_green());
    } else {
        println!("{}", "ℹ️  No changes made".bright_blue());
    }

    println!();

    // Ask about pushing to Fly.io
    if updated && which::which("flyctl").is_ok() {
        let push_to_fly = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Push these variables to Fly.io?")
            .default(false)
            .interact()?;

        if push_to_fly {
            println!();
            flyctl_push_secrets()?;
        }
    }

    println!();
    println!("{}", "✨ Setup complete!".bright_green().bold());
    println!();
    println!("Next steps:");
    println!("  1. Review the .env file at packages/server/.env");
    println!("  2. Restart Docker services to apply changes");
    println!("  3. Run 🐳 Docker restart from the main menu");

    Ok(())
}

fn manage_flyctl_secrets() -> Result<()> {
    // Check if flyctl is installed
    if which::which("flyctl").is_err() {
        println!("{}", "❌ flyctl is not installed".bright_red());
        println!();
        println!("Install it with:");
        println!("  curl -L https://fly.io/install.sh | sh");
        return Ok(());
    }

    loop {
        println!();
        println!("{}", "🚀 Fly.io Environment Variables".bright_cyan().bold());
        println!();

        let options = vec![
            "📋 List current secrets",
            "➕ Set a secret",
            "⬇️  Pull secrets to .env (from Fly.io)",
            "⬆️  Push secrets from .env (to Fly.io)",
            "🔙 Back to main menu",
        ];

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do? (type to filter)")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => flyctl_list_secrets()?,
            1 => flyctl_set_secret()?,
            2 => flyctl_pull_secrets()?,
            3 => flyctl_push_secrets()?,
            4 => break,
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn flyctl_list_secrets() -> Result<()> {
    println!("{}", "📋 Listing Fly.io secrets...".bright_blue().bold());
    println!();

    let output = Command::new("flyctl")
        .args(&["secrets", "list"])
        .output()
        .context("Failed to list secrets")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("{}", "❌ Failed to list secrets".bright_red());
        println!("{}", stderr);
    }

    Ok(())
}

fn flyctl_set_secret() -> Result<()> {
    use dialoguer::Input;

    println!("{}", "➕ Set a Fly.io secret".bright_blue().bold());
    println!();

    let key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Secret name (e.g., ANTHROPIC_API_KEY)")
        .interact_text()?;

    let value: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Secret value")
        .interact_text()?;

    println!();
    println!("{}", "Setting secret...".bright_yellow());

    let output = Command::new("flyctl")
        .args(&["secrets", "set", &format!("{}={}", key, value)])
        .output()
        .context("Failed to set secret")?;

    if output.status.success() {
        println!("{}", "✅ Secret set successfully".bright_green());
        println!();
        println!("Note: The deployment will restart to apply the new secret.");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("{}", "❌ Failed to set secret".bright_red());
        println!("{}", stderr);
    }

    Ok(())
}

fn flyctl_pull_secrets() -> Result<()> {
    println!("{}", "⬇️  Pulling secrets from Fly.io...".bright_blue().bold());
    println!();
    println!("This will fetch environment variables from your Fly.io app and save them to packages/server/.env");
    println!();
    println!("{}", "⚠️  Warning: This will OVERWRITE your local .env file".bright_yellow());
    println!();

    use dialoguer::Confirm;
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Continue?")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("{}", "Cancelled".dimmed());
        return Ok(());
    }

    println!();
    println!("{}", "Connecting to Fly.io app and fetching environment variables...".bright_yellow());
    println!();

    // Use flyctl ssh console to read environment variables from running app
    let output = Command::new("flyctl")
        .args(&["ssh", "console", "-C", "printenv"])
        .output()
        .context("Failed to connect to Fly.io app")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("{}", "❌ Failed to fetch environment variables".bright_red());
        println!("{}", stderr);
        println!();
        println!("Make sure:");
        println!("  1. Your Fly.io app is deployed and running");
        println!("  2. You're authenticated with flyctl (run: flyctl auth login)");
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse environment variables
    let mut env_vars: Vec<(String, String)> = Vec::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Filter out system variables and only keep relevant app secrets
            if key.starts_with("FLY_") ||
               key == "PATH" ||
               key == "HOME" ||
               key == "USER" ||
               key == "HOSTNAME" ||
               key == "TERM" ||
               key == "PWD" ||
               key == "SHLVL" ||
               key == "_" {
                continue;
            }

            env_vars.push((key.to_string(), value.to_string()));
        }
    }

    if env_vars.is_empty() {
        println!("{}", "❌ No environment variables found".bright_yellow());
        println!("Make sure your app has secrets configured on Fly.io");
        return Ok(());
    }

    println!("{}", format!("Found {} environment variables", env_vars.len()).bright_green());
    println!();

    // Preview variables
    println!("Variables to save:");
    for (key, _) in &env_vars {
        println!("  • {}", key.bright_cyan());
    }
    println!();

    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Save these to packages/server/.env?")
        .default(true)
        .interact()?;

    if !confirmed {
        println!("{}", "Cancelled".dimmed());
        return Ok(());
    }

    // Build .env content
    let mut env_content = String::from("# Environment Variables\n");
    env_content.push_str("# Pulled from Fly.io\n\n");

    for (key, value) in &env_vars {
        env_content.push_str(&format!("{}={}\n", key, value));
    }

    // Write to .env file
    let project_root = get_project_root()?;
    let env_path = project_root.join("packages/server/.env");

    std::fs::write(&env_path, env_content)
        .context("Failed to write .env file")?;

    println!();
    println!("{}", "✅ Environment variables saved to packages/server/.env".bright_green());
    println!();
    println!("Saved {} variables", env_vars.len());

    Ok(())
}

fn flyctl_push_secrets() -> Result<()> {
    println!("{}", "⬆️  Pushing secrets to Fly.io...".bright_blue().bold());
    println!();

    // Read .env file
    let env_file = PathBuf::from("packages/server/.env");
    if !env_file.exists() {
        println!("{}", "❌ No .env file found at packages/server/.env".bright_red());
        println!();
        println!("Create a .env file first with your secrets.");
        return Ok(());
    }

    let env_content = std::fs::read_to_string(&env_file)
        .context("Failed to read .env file")?;

    // Parse environment variables
    let mut secrets = Vec::new();
    for line in env_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            secrets.push((key.trim().to_string(), value.trim().to_string()));
        }
    }

    if secrets.is_empty() {
        println!("{}", "❌ No secrets found in .env file".bright_yellow());
        return Ok(());
    }

    println!("Found {} secret(s) in .env file:", secrets.len());
    for (key, _) in &secrets {
        println!("  • {}", key.bright_cyan());
    }
    println!();

    use dialoguer::Confirm;
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Push these secrets to Fly.io?")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("{}", "Cancelled".dimmed());
        return Ok(());
    }

    println!();
    println!("{}", "Pushing secrets...".bright_yellow());

    // Build flyctl command with all secrets
    let mut args = vec!["secrets", "set"];
    let secret_pairs: Vec<String> = secrets
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    for pair in &secret_pairs {
        args.push(pair.as_str());
    }

    let output = Command::new("flyctl")
        .args(&args)
        .output()
        .context("Failed to push secrets")?;

    if output.status.success() {
        println!("{}", "✅ Secrets pushed successfully".bright_green());
        println!();
        println!("Note: The deployment will restart to apply the new secrets.");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("{}", "❌ Failed to push secrets".bright_red());
        println!("{}", stderr);
    }

    Ok(())
}

fn check_api_keys(project_root: &PathBuf) -> Result<()> {
    println!(
        "{}",
        "🔑 Checking API keys status...".bright_blue().bold()
    );
    println!();

    // Check .env file in server directory
    let env_file = project_root.join("packages/server/.env");
    let env_exists = env_file.exists();

    if !env_exists {
        println!(
            "{}",
            "⚠️  No .env file found in packages/server/".bright_yellow()
        );
        println!();
        println!("Create a .env file with the following keys:");
        println!();
    } else {
        println!("{}", "✓ .env file found".bright_green());
        println!();
    }

    // Define required and optional keys
    let required_keys = vec![
        ("ANTHROPIC_API_KEY", "Anthropic Claude AI access"),
        ("VOYAGE_API_KEY", "Voyage AI embeddings"),
        ("FIRECRAWL_API_KEY", "Firecrawl web scraping"),
        ("TWILIO_ACCOUNT_SID", "Twilio authentication SID"),
        ("TWILIO_AUTH_TOKEN", "Twilio authentication token"),
        ("TWILIO_VERIFY_SERVICE_SID", "Twilio verify service"),
        ("JWT_SECRET", "JWT token signing secret"),
    ];

    let optional_keys = vec![
        ("TAVILY_API_KEY", "Tavily search API (optional)"),
        ("EXPO_ACCESS_TOKEN", "Expo notifications (optional)"),
        ("CLERK_SECRET_KEY", "Clerk authentication (optional)"),
    ];

    // Check required keys
    println!("{}", "Required API Keys:".bright_cyan().bold());
    let mut missing_required = 0;
    for (key, description) in &required_keys {
        let is_set = env::var(key).is_ok();
        if is_set {
            println!("  {} {} - {}", "✓".bright_green(), key, description);
        } else {
            println!("  {} {} - {}", "✗".bright_red(), key, description);
            missing_required += 1;
        }
    }

    println!();
    println!("{}", "Optional API Keys:".bright_cyan().bold());
    for (key, description) in &optional_keys {
        let is_set = env::var(key).is_ok();
        if is_set {
            println!("  {} {} - {}", "✓".bright_green(), key, description);
        } else {
            println!("  {} {} - {}", "○".dimmed(), key, description);
        }
    }

    println!();
    if missing_required > 0 {
        println!(
            "{}",
            format!("❌ {} required key(s) missing", missing_required).bright_red()
        );
        println!();
        println!("The API server will not start without these keys.");
        println!();
        println!("To fix:");
        println!("  1. Create packages/server/.env file");
        println!("  2. Add the missing keys with their values");
        println!("  3. Restart Docker services");
    } else {
        println!("{}", "✅ All required API keys are set!".bright_green());
    }

    println!();
    println!("Example .env file:");
    println!("{}", "─────────────────────────────────────".dimmed());
    println!("ANTHROPIC_API_KEY=sk-ant-...");
    println!("VOYAGE_API_KEY=pa-...");
    println!("FIRECRAWL_API_KEY=fc-...");
    println!("TWILIO_ACCOUNT_SID=AC...");
    println!("TWILIO_AUTH_TOKEN=...");
    println!("TWILIO_VERIFY_SERVICE_SID=VA...");
    println!("JWT_SECRET=your-random-secret-here");
    println!("{}", "─────────────────────────────────────".dimmed());

    Ok(())
}

fn deploy_to_flyio(project_root: &PathBuf) -> Result<()> {
    use dialoguer::Confirm;

    println!();
    println!("{}", "🚁 Deploy to Fly.io".bright_blue().bold());
    println!();

    // Check if flyctl is installed
    if which::which("flyctl").is_err() {
        println!("{}", "❌ flyctl is not installed".bright_red());
        println!();
        println!("Install it with:");
        println!("  curl -L https://fly.io/install.sh | sh");
        return Ok(());
    }

    // Check if user is authenticated
    let auth_check = Command::new("flyctl")
        .args(&["auth", "whoami"])
        .output()
        .context("Failed to check flyctl authentication")?;

    if !auth_check.status.success() {
        println!("{}", "❌ Not authenticated with Fly.io".bright_red());
        println!();
        println!("Run: flyctl auth login");
        return Ok(());
    }

    println!("{}", "Deployment Checklist:".bright_yellow().bold());
    println!();
    println!("  ✓ flyctl installed");
    println!("  ✓ Authenticated with Fly.io");
    println!();

    // Confirm deployment
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Deploy to Fly.io now?")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("{}", "Cancelled".dimmed());
        return Ok(());
    }

    println!();
    println!("{}", "🚀 Starting deployment...".bright_yellow());
    println!();

    // Run fly deploy
    let status = Command::new("flyctl")
        .args(&["deploy"])
        .current_dir(project_root)
        .status()
        .context("Failed to run fly deploy")?;

    if status.success() {
        println!();
        println!("{}", "✅ Deployment successful!".bright_green().bold());
        println!();
        println!("Useful commands:");
        println!("  {} View logs", "fly logs".bright_cyan());
        println!("  {} Check status", "fly status".bright_cyan());
        println!("  {} Open in browser", "fly open".bright_cyan());
    } else {
        println!();
        println!("{}", "❌ Deployment failed".bright_red());
        println!();
        println!("Check the output above for errors.");
        println!("Common issues:");
        println!("  • No fly.toml file found (run: fly launch)");
        println!("  • App not created yet (run: fly apps create)");
        println!("  • Docker build errors (check Dockerfile)");
    }

    Ok(())
}

fn manage_admin_users(project_root: &PathBuf) -> Result<()> {
    loop {
        println!();
        println!("{}", "👤 Admin User Management".bright_cyan().bold());
        println!();
        println!("Admin users are managed via the ADMIN_IDENTIFIERS environment variable.");
        println!("Supports emails and phone numbers (E.164: +1234567890)");
        println!();

        let options = vec![
            "📋 Show current admin identifiers",
            "➕ Add admin identifier (auto-saves)",
            "➖ Remove admin identifier (auto-saves)",
            "⬆️  Push to Fly.io (production)",
            "🔙 Back to main menu",
        ];

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do? (type to filter)")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => show_admin_emails(project_root)?,
            1 => add_admin_email(project_root)?,
            2 => remove_admin_email(project_root)?,
            3 => push_admin_emails_to_flyio(project_root)?,
            4 => break,
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn get_admin_emails(project_root: &PathBuf) -> Result<Vec<String>> {
    let env_file = project_root.join("packages/server/.env");

    if !env_file.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&env_file)?;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("ADMIN_IDENTIFIERS=") {
            let value = line.strip_prefix("ADMIN_IDENTIFIERS=").unwrap();
            return Ok(value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect());
        }
    }

    Ok(Vec::new())
}

fn show_admin_emails(project_root: &PathBuf) -> Result<()> {
    println!("{}", "📋 Current Admin Identifiers".bright_blue().bold());
    println!("{}", "  (Emails and phone numbers with admin access)".dimmed());
    println!();

    let identifiers = get_admin_emails(project_root)?;

    if identifiers.is_empty() {
        println!("{}", "  No admin identifiers configured".dimmed());
    } else {
        for (i, identifier) in identifiers.iter().enumerate() {
            println!("  {}. {}", i + 1, identifier.bright_cyan());
        }
    }

    println!();
    Ok(())
}

fn add_admin_email(project_root: &PathBuf) -> Result<()> {
    println!("{}", "➕ Add Admin Identifier".bright_blue().bold());
    println!();
    println!("{}", "Enter an email address or phone number (E.164 format: +1234567890)".dimmed());
    println!();

    let identifier: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Email or phone number")
        .validate_with(|input: &String| -> Result<(), &str> {
            // Check if it's an email (contains @ and .)
            let is_email = input.contains('@') && input.contains('.');
            // Check if it's a phone number (starts with + and has at least 10 digits)
            let is_phone = input.starts_with('+') && input.chars().filter(|c| c.is_numeric()).count() >= 10;

            if is_email || is_phone {
                Ok(())
            } else {
                Err("Please enter a valid email address or phone number (E.164: +1234567890)")
            }
        })
        .interact_text()?;

    let mut identifiers = get_admin_emails(project_root)?;

    if identifiers.contains(&identifier) {
        println!();
        println!("{}", "⚠️  Identifier already in admin list".bright_yellow());
        return Ok(());
    }

    identifiers.push(identifier.clone());

    // Save to .env file
    save_admin_emails_list(project_root, &identifiers)?;

    println!();
    println!("{}", format!("✅ Added {} and saved to .env", identifier).bright_green());
    println!();
    println!("Current admin identifiers:");
    for (i, id) in identifiers.iter().enumerate() {
        println!("  {}. {}", i + 1, id.bright_cyan());
    }

    Ok(())
}

fn remove_admin_email(project_root: &PathBuf) -> Result<()> {
    println!("{}", "➖ Remove Admin Identifier".bright_blue().bold());
    println!();

    let mut identifiers = get_admin_emails(project_root)?;

    if identifiers.is_empty() {
        println!("{}", "  No admin identifiers to remove".dimmed());
        return Ok(());
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select identifier to remove")
        .items(&identifiers)
        .interact()?;

    let removed = identifiers.remove(selection);

    // Save to .env file
    save_admin_emails_list(project_root, &identifiers)?;

    println!();
    println!("{}", format!("✅ Removed {} and saved to .env", removed).bright_green());
    println!();
    println!("Remaining admin identifiers:");
    if identifiers.is_empty() {
        println!("{}", "  No admins remaining".dimmed());
    } else {
        for (i, id) in identifiers.iter().enumerate() {
            println!("  {}. {}", i + 1, id.bright_cyan());
        }
    }

    Ok(())
}

fn save_admin_emails_list(project_root: &PathBuf, emails: &[String]) -> Result<()> {
    let env_file = project_root.join("packages/server/.env");

    // Read existing .env file
    let content = if env_file.exists() {
        std::fs::read_to_string(&env_file)?
    } else {
        String::new()
    };

    // Remove existing ADMIN_IDENTIFIERS line
    let mut lines: Vec<String> = content
        .lines()
        .filter(|line| !line.trim().starts_with("ADMIN_IDENTIFIERS="))
        .map(|s| s.to_string())
        .collect();

    // Add new ADMIN_IDENTIFIERS line
    let admin_emails_line = format!("ADMIN_IDENTIFIERS={}", emails.join(","));

    // Find a good place to insert (after JWT_ISSUER or at the end)
    let insert_pos = lines
        .iter()
        .position(|line| line.starts_with("JWT_ISSUER="))
        .map(|pos| pos + 1)
        .unwrap_or(lines.len());

    lines.insert(insert_pos, admin_emails_line);

    // Write back to file
    let new_content = lines.join("\n") + "\n";
    std::fs::write(&env_file, new_content)?;

    Ok(())
}

fn push_admin_emails_to_flyio(project_root: &PathBuf) -> Result<()> {
    // Check if flyctl is installed
    if which::which("flyctl").is_err() {
        println!("{}", "❌ flyctl is not installed".bright_red());
        println!();
        println!("Install it with:");
        println!("  curl -L https://fly.io/install.sh | sh");
        return Ok(());
    }

    println!("{}", "⬆️  Push Admin Identifiers to Fly.io".bright_blue().bold());
    println!();

    let identifiers = get_admin_emails(project_root)?;

    if identifiers.is_empty() {
        println!("{}", "⚠️  No admin identifiers configured".bright_yellow());
        return Ok(());
    }

    println!("Will set ADMIN_IDENTIFIERS to:");
    for (i, identifier) in identifiers.iter().enumerate() {
        println!("  {}. {}", i + 1, identifier.bright_cyan());
    }
    println!();

    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Push to Fly.io? (This will restart the deployment)")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("{}", "Cancelled".dimmed());
        return Ok(());
    }

    println!();
    println!("{}", "Pushing to Fly.io...".bright_yellow());

    let admin_identifiers_value = identifiers.join(",");
    let output = Command::new("flyctl")
        .args(&["secrets", "set", &format!("ADMIN_IDENTIFIERS={}", admin_identifiers_value)])
        .output()
        .context("Failed to set ADMIN_IDENTIFIERS secret")?;

    if output.status.success() {
        println!("{}", "✅ ADMIN_IDENTIFIERS updated on Fly.io".bright_green());
        println!();
        println!("The deployment will restart to apply the changes.");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("{}", "❌ Failed to update ADMIN_IDENTIFIERS".bright_red());
        println!("{}", stderr);
    }

    Ok(())
}

fn open_graphql_playground() -> Result<()> {
    let url = "http://localhost:8080/graphql";

    println!(
        "{}",
        "🌐 Opening GraphQL Playground...".bright_blue().bold()
    );
    println!("   {}", url.dimmed());

    match open::that(url) {
        Ok(_) => {
            println!("{}", "✅ Browser opened".bright_green());
            println!();
            println!("If the server isn't running, start it with:");
            println!("  {} 🐳 Docker start", "→".bright_yellow());
        }
        Err(e) => {
            println!("{}", "❌ Failed to open browser".bright_red());
            println!();
            println!("Please open this URL manually:");
            println!("  {}", url.bright_cyan());
            return Err(anyhow::anyhow!("Failed to open browser: {}", e));
        }
    }

    Ok(())
}
