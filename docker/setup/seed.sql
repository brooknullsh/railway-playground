CREATE TABLE IF NOT EXISTS users (
  id            SERIAL PRIMARY KEY,
  first_name    TEXT NOT NULL,
  last_name     TEXT NOT NULL,
  age           INTEGER NOT NULL,
  mobile        TEXT NOT NULL,
  is_pro        BOOLEAN NOT NULL,
  refresh_token TEXT,
  created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users (first_name, last_name, age, mobile, is_pro, refresh_token) VALUES
  ('Alice','Anderson', 34, '+1-202-555-0123', TRUE, NULL),
  ('Bob','Brown', 28, '+1-203-555-0146', FALSE, NULL),
  ('Charlie','Clark', 41, '+1-204-555-0189', TRUE, NULL),
  ('Diana','Davis', 25, '+1-205-555-0111', FALSE, NULL),
  ('Ethan','Edwards', 37, '+1-206-555-0177', TRUE, NULL),
  ('Fiona','Foster', 30, '+1-207-555-0199', FALSE, NULL),
  ('George','Gomez', 45, '+1-208-555-0122', TRUE, NULL),
  ('Hannah','Hill', 22, '+1-209-555-0100', FALSE, NULL),
  ('Ian','Ingram', 29, '+1-210-555-0180', FALSE, NULL),
  ('Julia','Jones', 33, '+1-211-555-0133', TRUE, NULL),
  ('Kevin','Kim', 38, '+1-212-555-0155', TRUE, NULL),
  ('Laura','Lee', 27, '+1-213-555-0166', FALSE, NULL),
  ('Michael','Mason', 50, '+1-214-555-0170', TRUE, NULL),
  ('Nina','Nash', 31, '+1-215-555-0190', FALSE, NULL),
  ('Oliver','Owens', 26, '+1-216-555-0102', TRUE, NULL),
  ('Paula','Parker', 39, '+1-217-555-0124', FALSE, NULL),
  ('Quentin','Quinn', 44, '+1-218-555-0112', TRUE, NULL),
  ('Rachel','Reed', 23, '+1-219-555-0183', FALSE, NULL),
  ('Steve','Stone', 36, '+1-220-555-0144', TRUE, NULL),
  ('Tina','Taylor', 40, '+1-221-555-0150', TRUE, NULL);

CREATE INDEX IF NOT EXISTS idx_users_refresh_token
ON users (refresh_token)
WHERE refresh_token IS NOT NULL;
