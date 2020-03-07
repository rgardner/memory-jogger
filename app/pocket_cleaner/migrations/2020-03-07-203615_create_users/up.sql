-- Your SQL goes here
CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  pocket_user_token VARCHAR NOT NULL,
  email VARCHAR NOT NULL
)
