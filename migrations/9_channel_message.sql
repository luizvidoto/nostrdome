CREATE TABLE channel_message (
    event_id INTEGER PRIMARY KEY,
    channel_id TEXT NOT NULL,
    author TEXT NOT NULL,
    is_users INTEGER NOT NULL,
    -- UNIX timestamp as integer milliseconds
    created_at INTEGER NOT NULL,
    relay_url TEXT NOT NULL,
    content TEXT NOT NULL
);

-- -- Message Indexes
CREATE INDEX IF NOT EXISTS channel_id_index ON channel_message(channel_id);