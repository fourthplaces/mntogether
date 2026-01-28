-- Notifications table for tracking member notifications

CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- References
    need_id UUID NOT NULL REFERENCES organization_needs(id) ON DELETE CASCADE,
    member_id UUID NOT NULL REFERENCES members(id) ON DELETE CASCADE,

    -- Relevance explanation (AI-generated)
    why_relevant TEXT NOT NULL,

    -- Tracking
    clicked BOOLEAN DEFAULT false,
    responded BOOLEAN DEFAULT false,
    sent_at TIMESTAMPTZ DEFAULT NOW(),

    -- Prevent duplicate notifications
    UNIQUE(need_id, member_id)
);

-- Indexes
CREATE INDEX idx_notifications_member ON notifications(member_id);
CREATE INDEX idx_notifications_need ON notifications(need_id);
CREATE INDEX idx_notifications_sent_at ON notifications(sent_at);
CREATE INDEX idx_notifications_clicked ON notifications(clicked) WHERE clicked = true;

COMMENT ON TABLE notifications IS 'Tracks which members were notified about which needs';
COMMENT ON COLUMN notifications.why_relevant IS 'AI-generated explanation of why this need is relevant to this member';
COMMENT ON COLUMN notifications.clicked IS 'Whether member clicked/viewed the notification';
COMMENT ON COLUMN notifications.responded IS 'Whether member indicated they would help (optional tracking)';
