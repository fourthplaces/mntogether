-- Add foreign key constraint on edition_slots.post_template → post_template_configs.slug.
-- Prevents typos in post template slugs from creating broken slot references.
ALTER TABLE edition_slots
ADD CONSTRAINT fk_edition_slots_post_template
FOREIGN KEY (post_template) REFERENCES post_template_configs(slug);
