-- Generalize chatrooms into containers to support multiple use cases
-- (AI chat, listing comments, organization discussions, etc.)

-- Step 1: Rename chatrooms to containers
ALTER TABLE chatrooms RENAME TO containers;
ALTER INDEX idx_chatrooms_activity RENAME TO idx_containers_activity;

-- Step 2: Add polymorphic fields to containers
ALTER TABLE containers
    ADD COLUMN container_type TEXT,
    ADD COLUMN entity_id UUID;

-- Backfill existing containers as 'ai_chat'
UPDATE containers SET container_type = 'ai_chat' WHERE container_type IS NULL;

-- Make container_type NOT NULL after backfill
ALTER TABLE containers ALTER COLUMN container_type SET NOT NULL;

-- Add index for entity lookups
CREATE INDEX idx_containers_entity ON containers(container_type, entity_id);

-- Update comments
COMMENT ON TABLE containers IS 'Generic message containers for AI chat, listing comments, org discussions, etc.';
COMMENT ON COLUMN containers.container_type IS 'Type of container: ai_chat, listing_comments, org_discussion';
COMMENT ON COLUMN containers.entity_id IS 'ID of related entity (listing_id, organization_id, etc.) - null for standalone chats';

-- Step 3: Update foreign key references from chatrooms to containers
ALTER TABLE messages DROP CONSTRAINT messages_chatroom_id_fkey;
ALTER TABLE messages RENAME COLUMN chatroom_id TO container_id;
ALTER TABLE messages ADD CONSTRAINT messages_container_id_fkey
    FOREIGN KEY (container_id) REFERENCES containers(id) ON DELETE CASCADE;

ALTER TABLE referral_documents DROP CONSTRAINT referral_documents_chatroom_id_fkey;
ALTER TABLE referral_documents RENAME COLUMN chatroom_id TO container_id;
ALTER TABLE referral_documents ADD CONSTRAINT referral_documents_container_id_fkey
    FOREIGN KEY (container_id) REFERENCES containers(id);

-- Update indexes
DROP INDEX idx_messages_chatroom;
CREATE INDEX idx_messages_container ON messages(container_id, sequence_number);

DROP INDEX idx_documents_chatroom;
CREATE INDEX idx_documents_container ON referral_documents(container_id);

-- Step 4: Enhance messages table for public comments
ALTER TABLE messages
    ADD COLUMN author_id UUID REFERENCES members(id) ON DELETE SET NULL,
    ADD COLUMN moderation_status TEXT DEFAULT 'approved',
    ADD COLUMN parent_message_id UUID REFERENCES messages(id) ON DELETE CASCADE,
    ADD COLUMN updated_at TIMESTAMPTZ DEFAULT NOW(),
    ADD COLUMN edited_at TIMESTAMPTZ;

-- Add constraint for moderation_status
ALTER TABLE messages ADD CONSTRAINT messages_moderation_status_check
    CHECK (moderation_status IN ('approved', 'pending', 'flagged', 'removed'));

-- Update role constraint to include 'comment'
ALTER TABLE messages DROP CONSTRAINT messages_role_check;
ALTER TABLE messages ADD CONSTRAINT messages_role_check
    CHECK (role IN ('user', 'assistant', 'comment'));

-- Add indexes for new fields
CREATE INDEX idx_messages_author ON messages(author_id);
CREATE INDEX idx_messages_parent ON messages(parent_message_id);
CREATE INDEX idx_messages_moderation ON messages(moderation_status);

-- Update comments
COMMENT ON COLUMN messages.role IS 'Message role: user, assistant (for AI chat), comment (for public discussions)';
COMMENT ON COLUMN messages.author_id IS 'Optional member ID - null for anonymous comments or AI messages';
COMMENT ON COLUMN messages.moderation_status IS 'Moderation status: approved, pending, flagged, removed';
COMMENT ON COLUMN messages.parent_message_id IS 'For threaded discussions - parent message ID';

-- Step 5: Create containers for all existing listings
INSERT INTO containers (container_type, entity_id, language, created_at, last_activity_at)
SELECT
    'listing_comments' AS container_type,
    l.id AS entity_id,
    l.source_language AS language,
    l.created_at,
    l.created_at AS last_activity_at
FROM listings l
WHERE NOT EXISTS (
    SELECT 1 FROM containers c
    WHERE c.container_type = 'listing_comments'
    AND c.entity_id = l.id
);

COMMENT ON TABLE messages IS 'Messages in containers (AI chat messages, public comments, etc.)';
