CREATE TABLE IF NOT EXISTS user_config (
    id INTEGER PRIMARY KEY,
    has_logged_in INTEGER NOT NULL DEFAULT 0,
    profile_meta TEXT
);