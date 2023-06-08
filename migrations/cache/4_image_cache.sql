CREATE TABLE IF NOT EXISTS image_cache (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    kind INTEGER NOT NULL,
    event_hash TEXT NOT NULL
);

-- Events Indexes
CREATE UNIQUE INDEX IF NOT EXISTS event_hash_index ON image_cache(event_hash);

CREATE UNIQUE INDEX IF NOT EXISTS event_hash_kind_index ON image_cache(event_hash, kind);