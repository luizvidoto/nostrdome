CREATE TABLE IF NOT EXISTS cache_history (
    public_key BLOB PRIMARY KEY,
    -- UNIX milliseconds
    updated_at INTEGER NOT NULL,
    profile_image_url TEXT,
    profile_image_path TEXT,
    banner_image_url TEXT,
    banner_image_path TEXT
);