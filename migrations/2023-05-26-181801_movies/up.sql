-- Your SQL goes here
CREATE TABLE movies (
  id SERIAL PRIMARY KEY,
  title VARCHAR NOT NULL,
  year INTEGER NOT NULL,
  description TEXT NOT NULL
)