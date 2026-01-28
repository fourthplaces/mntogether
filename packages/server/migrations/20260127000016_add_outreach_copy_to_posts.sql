-- Add AI-generated outreach copy to posts
-- This stores personalized email text for members to easily contact organizations

ALTER TABLE posts
    ADD COLUMN outreach_copy TEXT;

-- Index for querying posts with outreach copy
CREATE INDEX idx_posts_outreach_copy
    ON posts(id)
    WHERE outreach_copy IS NOT NULL;

COMMENT ON COLUMN posts.outreach_copy IS
'AI-generated personalized outreach email text. Includes subject line and 3-sentence body.
Format: "Subject: {subject}\n\n{body}"
Used to pre-fill mailto: links with enthusiastic, specific, actionable copy.';
