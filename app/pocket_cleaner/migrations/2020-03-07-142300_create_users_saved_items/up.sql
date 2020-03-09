-- Your SQL goes here
CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  email VARCHAR NOT NULL,
  pocket_access_token VARCHAR
);

CREATE TABLE saved_items (
  id SERIAL PRIMARY KEY,
  user_id SERIAL references users(id),
  pocket_id VARCHAR NOT NULL,
  title VARCHAR NOT NULL,
  body TEXT NOT NULL
);
