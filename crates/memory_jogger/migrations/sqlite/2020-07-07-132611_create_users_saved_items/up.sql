-- Your SQL goes here
CREATE TABLE users (
  id INTEGER NOT NULL PRIMARY KEY,
  email TEXT NOT NULL,
  pocket_access_token TEXT,
  last_pocket_sync_time BIGINT
);

CREATE TABLE saved_items (
  id INTEGER NOT NULL PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
  pocket_id TEXT NOT NULL,
  title TEXT NOT NULL,
  excerpt TEXT,
  url TEXT,
  time_added TIMESTAMP
);

CREATE UNIQUE INDEX saved_items_pocket_id ON saved_items (pocket_id);
