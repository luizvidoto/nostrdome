CREATE TABLE IF NOT EXISTS channel (
    channel_id INTEGER PRIMARY KEY,
    -- database event_id
    event_id INTEGER NOT NULL REFERENCES event(event_id) ON UPDATE CASCADE ON DELETE CASCADE,
    event_hash TEXT NOT NULL,
    -- UNIX timestamp as integer milliseconds
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    about TEXT,
    banner TEXT,
    display_name TEXT,
    name TEXT,
    picture TEXT,
    website TEXT
);

-- Indexes
CREATE UNIQUE INDEX IF NOT EXISTS channel_event_hash_index ON channel(event_hash);

CREATE INDEX IF NOT EXISTS channel_event_id_index ON channel(event_id);