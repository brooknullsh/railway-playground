CREATE TABLE IF NOT EXISTS users (
  id            SERIAL PRIMARY KEY,
  first_name    TEXT NOT NULL,
  last_name     TEXT NOT NULL,
  age           INTEGER NOT NULL,
  mobile        TEXT NOT NULL,
  is_pro        BOOLEAN NOT NULL
);

INSERT INTO users (first_name, last_name, age, mobile, is_pro) VALUES
  ('Alice','Anderson', 34, '+1-202-555-0123', TRUE),
  ('Bob','Brown', 28, '+1-203-555-0146', FALSE),
  ('Charlie','Clark', 41, '+1-204-555-0189', TRUE),
  ('Diana','Davis', 25, '+1-205-555-0111', FALSE),
  ('Ethan','Edwards', 37, '+1-206-555-0177', TRUE),
  ('Fiona','Foster', 30, '+1-207-555-0199', FALSE),
  ('George','Gomez', 45, '+1-208-555-0122', TRUE),
  ('Hannah','Hill', 22, '+1-209-555-0100', FALSE),
  ('Ian','Ingram', 29, '+1-210-555-0180', FALSE),
  ('Julia','Jones', 33, '+1-211-555-0133', TRUE),
  ('Kevin','Kim', 38, '+1-212-555-0155', TRUE),
  ('Laura','Lee', 27, '+1-213-555-0166', FALSE),
  ('Michael','Mason', 50, '+1-214-555-0170', TRUE),
  ('Nina','Nash', 31, '+1-215-555-0190', FALSE),
  ('Oliver','Owens', 26, '+1-216-555-0102', TRUE),
  ('Paula','Parker', 39, '+1-217-555-0124', FALSE),
  ('Quentin','Quinn', 44, '+1-218-555-0112', TRUE),
  ('Rachel','Reed', 23, '+1-219-555-0183', FALSE),
  ('Steve','Stone', 36, '+1-220-555-0144', TRUE),
  ('Tina','Taylor', 40, '+1-221-555-0150', TRUE);
