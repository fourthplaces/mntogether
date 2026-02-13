//! Newsletter confirmation activity: follows a confirmation link via headless Chrome.

use anyhow::{Context, Result};
use tracing::info;
use uuid::Uuid;

use crate::common::SourceId;
use crate::domains::source::models::{NewsletterSource, Source};
use crate::kernel::ServerDeps;

#[derive(Debug)]
pub struct ConfirmResult {
    pub source_id: Uuid,
    pub status: String,
}

/// Confirm a newsletter subscription by following the confirmation link.
///
/// 1. Load the newsletter source
/// 2. Navigate to the confirmation link via headless Chrome
/// 3. Update status to active
pub async fn confirm_newsletter(source_id: Uuid, deps: &ServerDeps) -> Result<ConfirmResult> {
    let pool = &deps.db_pool;

    let source = Source::find_by_id(SourceId::from_uuid(source_id), pool).await?;
    let newsletter_source = NewsletterSource::find_by_source_id(source.id, pool)
        .await
        .context("Failed to load newsletter source")?;

    let confirmation_link = newsletter_source
        .confirmation_link
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("No confirmation link available"))?;

    info!(
        source_id = %source_id,
        confirmation_link = %confirmation_link,
        "Following newsletter confirmation link"
    );

    // Follow the confirmation link via headless Chrome
    match follow_confirmation_link(confirmation_link).await {
        Ok(()) => {
            info!(source_id = %source_id, "Newsletter confirmation succeeded");
            NewsletterSource::update_status(newsletter_source.id, "active", pool).await?;

            Ok(ConfirmResult {
                source_id,
                status: "active".to_string(),
            })
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "Failed to follow confirmation link"
            );
            NewsletterSource::update_status(newsletter_source.id, "confirmation_failed", pool)
                .await?;

            Ok(ConfirmResult {
                source_id,
                status: "confirmation_failed".to_string(),
            })
        }
    }
}

/// Follow a confirmation link via Playwright CLI.
async fn follow_confirmation_link(url: &str) -> Result<()> {
    let script = format!(
        r#"
        const {{ chromium }} = require('playwright');
        (async () => {{
            const browser = await chromium.launch({{ headless: true }});
            const page = await browser.newPage();
            const response = await page.goto('{}', {{ waitUntil: 'networkidle', timeout: 30000 }});

            if (response && response.status() >= 400) {{
                await browser.close();
                process.exit(1);
            }}

            // Wait for any redirects to complete
            await page.waitForTimeout(3000);
            await browser.close();
        }})();
        "#,
        url
    );

    let output = tokio::process::Command::new("node")
        .arg("-e")
        .arg(&script)
        .output()
        .await
        .context("Failed to run Playwright confirmation script")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Playwright confirmation failed: {}", stderr);
    }

    Ok(())
}
