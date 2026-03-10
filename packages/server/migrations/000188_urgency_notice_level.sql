-- Replace the old urgency check constraint (low/medium/high/urgent)
-- with new clean values: notice, urgent (NULL = no urgency)
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_urgency_check;
ALTER TABLE posts ADD CONSTRAINT posts_urgency_check
  CHECK (urgency = ANY (ARRAY['notice'::text, 'urgent'::text]));
