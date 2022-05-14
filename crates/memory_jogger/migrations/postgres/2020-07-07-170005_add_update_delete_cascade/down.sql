-- This file should undo anything in `up.sql`
ALTER TABLE saved_items
DROP CONSTRAINT saved_items_user_id_fkey,
ADD CONSTRAINT saved_items_user_id_fkey
  FOREIGN KEY (user_id)
  REFERENCES users(id);
ALTER TABLE saved_items ADD COLUMN IF NOT EXISTS body TEXT;
