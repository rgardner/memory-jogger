-- Your SQL goes here
ALTER TABLE users
ADD COLUMN last_pocket_sync_time bigint;

CREATE UNIQUE INDEX saved_items_pocket_id ON saved_items (pocket_id);

ALTER TABLE saved_items
ALTER COLUMN body DROP NOT NULL,
ADD COLUMN excerpt TEXT,
ADD COLUMN url TEXT;
