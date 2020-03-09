-- This file should undo anything in `up.sql`
ALTER TABLE saved_items
DROP COLUMN url,
DROP COLUMN excerpt,
ALTER COLUMN body SET NOT NULL;

DROP INDEX saved_items_pocket_id;

ALTER TABLE users DROP COLUMN last_pocket_sync_time;
