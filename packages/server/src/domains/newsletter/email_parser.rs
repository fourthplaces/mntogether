//! Email parsing utilities for newsletter ingestion.
//!
//! Handles HTML-to-markdown conversion for newsletter content and
//! confirmation link extraction from subscription confirmation emails.

/// Extract the sender's domain from an email address.
/// e.g., "newsletter@example.org" → "example.org"
pub fn extract_sender_domain(from: &str) -> Option<String> {
    // Handle formats like "Name <email@domain.com>" and plain "email@domain.com"
    let email = if let Some(start) = from.find('<') {
        let end = from.find('>')?;
        &from[start + 1..end]
    } else {
        from.trim()
    };

    email.split('@').nth(1).map(|d| d.to_lowercase())
}

/// Check if a sender domain matches the expected domain.
/// Allows subdomains: "mail.example.org" matches expected "example.org".
pub fn sender_domain_matches(sender_domain: &str, expected_domain: &str) -> bool {
    let sender = sender_domain.to_lowercase();
    let expected = expected_domain.to_lowercase();

    sender == expected || sender.ends_with(&format!(".{}", expected))
}

/// Convert HTML email body to clean markdown suitable for the extraction pipeline.
///
/// Strips tracking pixels, template boilerplate, unsubscribe footers,
/// style blocks, and inline CSS. Converts remaining HTML to markdown.
pub fn html_to_markdown(html: &str) -> String {
    // Remove style blocks
    let mut result = remove_between_tags(html, "style");
    // Remove script blocks
    result = remove_between_tags(&result, "script");

    // Remove tracking pixels (1x1 images, hidden images)
    result = remove_tracking_pixels(&result);

    // Remove common unsubscribe footer patterns
    result = remove_unsubscribe_footer(&result);

    // Convert HTML to plain text with basic markdown formatting
    html_to_text(&result)
}

/// Extract confirmation links from an email body.
///
/// Looks for anchor tags containing keywords like "confirm", "verify", "activate".
/// Returns the first matching URL.
pub fn extract_confirmation_link(html: &str) -> Option<String> {
    let html_lower = html.to_lowercase();
    let confirm_keywords = [
        "confirm",
        "verify",
        "activate",
        "yes, subscribe",
        "complete your subscription",
        "confirm subscription",
    ];

    // Find all href attributes in anchor tags
    let mut pos = 0;
    while let Some(href_start) = html_lower[pos..].find("href=\"") {
        let abs_start = pos + href_start + 6; // Skip past 'href="'
        if let Some(href_end) = html[abs_start..].find('"') {
            let url = &html[abs_start..abs_start + href_end];

            // Check surrounding context (100 chars before and after) for confirm keywords
            let context_start = pos + href_start.saturating_sub(100);
            let context_end = (abs_start + href_end + 100).min(html_lower.len());
            let context = &html_lower[context_start..context_end];

            for keyword in &confirm_keywords {
                if context.contains(keyword) && url.starts_with("http") {
                    return Some(url.to_string());
                }
            }

            pos = abs_start + href_end;
        } else {
            break;
        }
    }

    None
}

/// Remove content between opening and closing tags of a given type.
fn remove_between_tags(html: &str, tag: &str) -> String {
    let open_tag = format!("<{}", tag);
    let close_tag = format!("</{}>", tag);
    let mut result = html.to_string();

    while let Some(start) = result.to_lowercase().find(&open_tag) {
        if let Some(end) = result.to_lowercase()[start..].find(&close_tag) {
            let remove_end = start + end + close_tag.len();
            result = format!("{}{}", &result[..start], &result[remove_end..]);
        } else {
            break;
        }
    }

    result
}

/// Remove tracking pixels (1x1 images, images with tracking-related src patterns).
fn remove_tracking_pixels(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let html_lower = html.to_lowercase();
    let mut pos = 0;

    while pos < html.len() {
        if html_lower[pos..].starts_with("<img") {
            // Find the end of this img tag
            if let Some(end) = html[pos..].find('>') {
                let tag = &html_lower[pos..pos + end + 1];
                // Skip if it's a tracking pixel
                let is_tracking = tag.contains("width=\"1\"")
                    || tag.contains("width='1'")
                    || tag.contains("height=\"1\"")
                    || tag.contains("height='1'")
                    || tag.contains("tracking")
                    || tag.contains("beacon")
                    || tag.contains("pixel");

                if is_tracking {
                    pos += end + 1;
                    continue;
                }
            }
        }

        if let Some(ch) = html.get(pos..pos + 1) {
            result.push_str(ch);
        }
        pos += 1;
    }
    result
}

/// Remove common unsubscribe footer content.
fn remove_unsubscribe_footer(html: &str) -> String {
    let html_lower = html.to_lowercase();

    // Find common footer markers and truncate
    let footer_markers = [
        "unsubscribe from this list",
        "update your preferences",
        "you are receiving this email because",
        "to stop receiving these emails",
        "click here to unsubscribe",
        "manage your subscription",
    ];

    for marker in &footer_markers {
        if let Some(pos) = html_lower.find(marker) {
            // Find the nearest parent block element before this marker
            let before = &html[..pos];
            if let Some(block_start) = before.rfind("<tr") {
                return html[..block_start].to_string();
            }
            if let Some(block_start) = before.rfind("<div") {
                return html[..block_start].to_string();
            }
            // If no block element found, just truncate at the marker
            return html[..pos].to_string();
        }
    }

    html.to_string()
}

/// Basic HTML to text conversion with markdown formatting.
fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    // Replace common block elements with newlines
    for tag in &["</p>", "</div>", "</tr>", "<br>", "<br/>", "<br />"] {
        text = text.replace(tag, "\n");
    }

    // Replace headings with markdown
    for level in 1..=6 {
        let open = format!("<h{}", level);
        let close = format!("</h{}>", level);
        let prefix = "#".repeat(level);

        while let Some(start) = text.to_lowercase().find(&open) {
            if let Some(tag_end) = text[start..].find('>') {
                let content_start = start + tag_end + 1;
                if let Some(close_pos) = text.to_lowercase()[content_start..].find(&close) {
                    let content = text[content_start..content_start + close_pos].trim();
                    let replacement = format!("\n{} {}\n", prefix, content);
                    text = format!(
                        "{}{}{}",
                        &text[..start],
                        replacement,
                        &text[content_start + close_pos + close.len()..]
                    );
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Convert links to markdown
    // Simple approach: extract href and text from <a> tags
    while let Some(start) = text.to_lowercase().find("<a ") {
        if let Some(tag_end) = text[start..].find('>') {
            let tag = &text[start..start + tag_end + 1];
            let close_pos = text.to_lowercase()[start + tag_end + 1..]
                .find("</a>")
                .map(|p| start + tag_end + 1 + p);

            if let Some(close) = close_pos {
                let link_text = text[start + tag_end + 1..close].trim();
                let href = extract_href_from_tag(tag);

                let replacement = if let Some(url) = href {
                    if link_text.is_empty() {
                        url
                    } else {
                        format!("[{}]({})", link_text, url)
                    }
                } else {
                    link_text.to_string()
                };

                text = format!(
                    "{}{}{}",
                    &text[..start],
                    replacement,
                    &text[close + 4..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Strip remaining HTML tags
    let mut clean = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => clean.push(ch),
            _ => {}
        }
    }

    // Decode common HTML entities
    clean = clean
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Collapse multiple newlines into at most two
    let mut result = String::with_capacity(clean.len());
    let mut newline_count = 0;
    for ch in clean.chars() {
        if ch == '\n' {
            newline_count += 1;
            if newline_count <= 2 {
                result.push(ch);
            }
        } else {
            newline_count = 0;
            result.push(ch);
        }
    }

    result.trim().to_string()
}

/// Extract href value from an anchor tag string.
fn extract_href_from_tag(tag: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let href_start = lower.find("href=\"")? + 6;
    let href_end = tag[href_start..].find('"')?;
    Some(tag[href_start..href_start + href_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_sender_domain() {
        assert_eq!(
            extract_sender_domain("newsletter@example.org"),
            Some("example.org".to_string())
        );
        assert_eq!(
            extract_sender_domain("Name <newsletter@example.org>"),
            Some("example.org".to_string())
        );
        assert_eq!(extract_sender_domain("invalid"), None);
    }

    #[test]
    fn test_sender_domain_matches() {
        assert!(sender_domain_matches("example.org", "example.org"));
        assert!(sender_domain_matches("mail.example.org", "example.org"));
        assert!(!sender_domain_matches("example.org", "other.org"));
        assert!(!sender_domain_matches("notexample.org", "example.org"));
    }

    #[test]
    fn test_extract_confirmation_link() {
        let html = r#"
            <p>Thanks for subscribing!</p>
            <a href="https://example.org/confirm?token=abc123">Confirm your subscription</a>
            <a href="https://example.org/other">Other link</a>
        "#;
        assert_eq!(
            extract_confirmation_link(html),
            Some("https://example.org/confirm?token=abc123".to_string())
        );
    }

    #[test]
    fn test_extract_confirmation_link_no_match() {
        let html = r#"
            <a href="https://example.org/page">Read more</a>
        "#;
        assert_eq!(extract_confirmation_link(html), None);
    }
}
