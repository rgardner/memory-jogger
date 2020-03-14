-- This file should undo anything in `up.sql`
ALTER TABLE saved_items
DROP COLUMN time_added;
