-- Add body_ast column to posts for storing Plate.js editor state as JSON AST.
-- Nullable: existing posts keep description_markdown only; body_ast is populated
-- on first save from the new WYSIWYG editor.
ALTER TABLE posts ADD COLUMN body_ast JSONB;
