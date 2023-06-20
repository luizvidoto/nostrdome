CREATE TABLE IF NOT EXISTS event (
    event_id INTEGER PRIMARY KEY,
    event_hash TEXT NOT NULL UNIQUE,
    -- author pubkey
    pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    kind INTEGER NOT NULL,
    -- serialized json of event object 
    content TEXT NOT NULL,
    -- serialized json vector of strings
    tags TEXT,
    -- event signature
    sig TEXT NOT NULL,
    relay_url TEXT NOT NULL
);

-- Events Indexes
CREATE INDEX IF NOT EXISTS event_hash_index ON event(event_hash);

CREATE INDEX IF NOT EXISTS pubkey_index ON event(pubkey);

CREATE INDEX IF NOT EXISTS kind_index ON event(kind);

CREATE INDEX IF NOT EXISTS kind_pubkey_index ON event(kind, pubkey);