CREATE TABLE IF NOT EXISTS event (
    event_id INTEGER PRIMARY KEY,
    -- 4-byte hash
    event_hash BLOB NOT NULL,
    -- author pubkey
    pubkey BLOB NOT NULL,
    local_creation INTEGER NOT NULL,
    remote_creation INTEGER,
    received_at INTEGER,
    -- event kind
    kind INTEGER NOT NULL,
    -- serialized json of event object 
    content TEXT NOT NULL,
    -- serialized json vector of strings
    tags TEXT,
    -- event signature
    sig TEXT NOT NULL,
    relay_url TEXT
);

-- Events Indexes
CREATE UNIQUE INDEX IF NOT EXISTS event_hash_index ON event(event_hash);

CREATE INDEX IF NOT EXISTS pubkey_index ON event(pubkey);

CREATE INDEX IF NOT EXISTS kind_index ON event(kind);

CREATE INDEX IF NOT EXISTS kind_pubkey_index ON event(kind, pubkey);

CREATE INDEX IF NOT EXISTS pubkey_kind_index ON event(pubkey, kind);