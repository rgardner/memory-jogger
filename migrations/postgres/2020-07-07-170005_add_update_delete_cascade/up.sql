-- Your SQL goes here
ALTER TABLE saved_items
DROP COLUMN body,
DROP CONSTRAINT saved_items_user_id_fkey,
ADD CONSTRAINT saved_items_user_id_fkey
  FOREIGN KEY (user_id)
  REFERENCES users(id)
  ON UPDATE CASCADE ON DELETE CASCADE;
