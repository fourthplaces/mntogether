ALTER TABLE tag_kinds ADD COLUMN required BOOLEAN NOT NULL DEFAULT false;
UPDATE tag_kinds SET required = true WHERE slug IN ('audience_role', 'post_type');
