-- Rename note severity 'warn' to 'urgent'
UPDATE notes SET severity = 'urgent' WHERE severity = 'warn';
