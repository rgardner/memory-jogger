-- Your SQL goes here
CREATE TABLE saved_items (
  id SERIAL PRIMARY KEY,
  pocket_id VARCHAR NOT NULL,
  title VARCHAR NOT NULL,
  body TEXT NOT NULL
)
