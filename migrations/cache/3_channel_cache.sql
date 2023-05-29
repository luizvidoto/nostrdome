CREATE TABLE IF NOT EXISTS channel_cache (
    -- channel_id is the hash of the channel's first event
    creation_event_hash BLOB PRIMARY KEY,
    creator_pubkey BLOB NOT NULL,
    -- UNIX milliseconds
    created_at INTEGER NOT NULL,
    updated_event_hash BLOB,
    -- UNIX milliseconds
    updated_at INTEGER,
    -- METADATA JSON CONTENT (name, about, picture)
    metadata TEXT NOT NULL,
    image_path TEXT
);

-- Channel Cache Indexes
CREATE UNIQUE INDEX IF NOT EXISTS creator_pubkey_index ON channel_cache(creator_pubkey);

CREATE UNIQUE INDEX IF NOT EXISTS updated_event_hash_index ON channel_cache(updated_event_hash);