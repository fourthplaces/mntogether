//! Newsletter subscription activity: submits a signup form via headless Chrome.

use anyhow::{Context, Result};
use tracing::info;
use uuid::Uuid;

use crate::common::DetectedNewsletterFormId;
use crate::domains::source::models::{
    create_newsletter_source, DetectedNewsletterForm, NewsletterSource,
};
use crate::kernel::ServerDeps;

#[derive(Debug)]
pub struct SubscribeResult {
    pub source_id: Uuid,
    pub ingest_email: String,
    pub status: String,
}

/// Subscribe to a newsletter detected on a website.
///
/// 1. Load the detected form
/// 2. Create a newsletter source with a generated ingest email
/// 3. Submit the form via headless Chrome (Playwright)
/// 4. Update status to pending_confirmation
pub async fn subscribe_to_newsletter(
    form_id: Uuid,
    organization_id: Option<Uuid>,
    deps: &ServerDeps,
) -> Result<SubscribeResult> {
    let pool = &deps.db_pool;

    // Load the detected form
    let form = DetectedNewsletterForm::find_by_id(
        DetectedNewsletterFormId::from_uuid(form_id),
        pool,
    )
    .await
    .context("Failed to load detected newsletter form")?;

    // Check if form requires extra fields
    if form.requires_extra_fields {
        return Ok(SubscribeResult {
            source_id: Uuid::nil(),
            ingest_email: String::new(),
            status: "failed_requires_manual".to_string(),
        });
    }

    // Create the newsletter source
    let org_id = organization_id.map(crate::common::OrganizationId::from_uuid);
    let (source, newsletter_source) =
        create_newsletter_source(&form.form_url, org_id, pool).await?;

    info!(
        source_id = %source.id,
        ingest_email = %newsletter_source.ingest_email,
        form_url = %form.form_url,
        "Created newsletter source, submitting signup form"
    );

    // Update status to subscribing
    NewsletterSource::update_status(newsletter_source.id, "subscribing", pool).await?;

    // Submit the form via headless Chrome
    match submit_signup_form(&form.form_url, &newsletter_source.ingest_email).await {
        Ok(()) => {
            info!(
                source_id = %source.id,
                "Newsletter signup form submitted successfully"
            );
            NewsletterSource::update_status(newsletter_source.id, "pending_confirmation", pool)
                .await?;

            // Mark the detected form as subscribed
            DetectedNewsletterForm::update_status(form.id, "subscribed", pool).await?;

            Ok(SubscribeResult {
                source_id: source.id.into_uuid(),
                ingest_email: newsletter_source.ingest_email,
                status: "pending_confirmation".to_string(),
            })
        }
        Err(e) => {
            tracing::error!(
                source_id = %source.id,
                error = %e,
                "Failed to submit newsletter signup form"
            );
            NewsletterSource::update_status(newsletter_source.id, "failed", pool).await?;

            Ok(SubscribeResult {
                source_id: source.id.into_uuid(),
                ingest_email: newsletter_source.ingest_email,
                status: "failed".to_string(),
            })
        }
    }
}

/// Submit a newsletter signup form via Playwright CLI.
///
/// Uses `npx playwright` to navigate to the form URL,
/// fill in the email field, and submit.
async fn submit_signup_form(form_url: &str, email: &str) -> Result<()> {
    // Use a Playwright script to submit the form
    let script = format!(
        r#"
        const {{ chromium }} = require('playwright');
        (async () => {{
            const browser = await chromium.launch({{ headless: true }});
            const page = await browser.newPage();
            await page.goto('{}', {{ waitUntil: 'networkidle', timeout: 30000 }});

            // Try common email input selectors
            const selectors = [
                'input[type="email"]',
                'input[name="email"]',
                'input[name="EMAIL"]',
                'input[placeholder*="email" i]',
                'input[placeholder*="Email" i]',
            ];

            let emailInput = null;
            for (const selector of selectors) {{
                emailInput = await page.$(selector);
                if (emailInput) break;
            }}

            if (!emailInput) {{
                await browser.close();
                process.exit(1);
            }}

            await emailInput.fill('{}');

            // Try to find and click the submit button
            const submitSelectors = [
                'button[type="submit"]',
                'input[type="submit"]',
                'button:has-text("Subscribe")',
                'button:has-text("Sign Up")',
                'button:has-text("Sign up")',
                'input[value*="Subscribe" i]',
                'input[value*="Sign Up" i]',
            ];

            let submitted = false;
            for (const selector of submitSelectors) {{
                const btn = await page.$(selector);
                if (btn) {{
                    await btn.click();
                    submitted = true;
                    break;
                }}
            }}

            if (!submitted) {{
                // Try pressing Enter in the email field
                await emailInput.press('Enter');
            }}

            // Wait for navigation or response
            await page.waitForTimeout(3000);
            await browser.close();
        }})();
        "#,
        form_url, email
    );

    let output = tokio::process::Command::new("node")
        .arg("-e")
        .arg(&script)
        .output()
        .await
        .context("Failed to run Playwright script")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Playwright form submission failed: {}", stderr);
    }

    Ok(())
}
