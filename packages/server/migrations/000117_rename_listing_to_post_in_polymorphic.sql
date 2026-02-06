-- Rename 'listing' ghost discriminators to 'post' in all polymorphic columns.
-- The listings table was renamed to posts in migration 000087, but
-- taggables, contacts, and document_references still use 'listing'.

-- taggables: 'listing' → 'post'
UPDATE taggables SET taggable_type = 'post' WHERE taggable_type = 'listing';

-- contacts: 'listing' → 'post' (before we drop this table in 000118)
UPDATE contacts SET contactable_type = 'post' WHERE contactable_type = 'listing';

-- document_references: 'listing' → 'post'
UPDATE document_references SET reference_kind = 'post' WHERE reference_kind = 'listing';
