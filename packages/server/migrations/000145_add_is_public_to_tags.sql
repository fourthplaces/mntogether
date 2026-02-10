ALTER TABLE tag_kinds ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT false;

COMMENT ON COLUMN tag_kinds.is_public IS 'Whether tags of this kind are visible on the public home page';
