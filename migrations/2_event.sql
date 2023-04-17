CREATE TABLE IF NOT EXISTS event (
    -- event_id INTEGER PRIMARY KEY,
    -- 4-byte hash
    event_hash BLOB PRIMARY KEY,
    -- author pubkey
    pubkey BLOB NOT NULL,
    -- when the event was authored
    created_at INTEGER NOT NULL,
    -- event kind
    kind INTEGER NOT NULL,
    -- serialized json of event object 
    content TEXT NOT NULL,
    -- event signature
    sig TEXT NOT NULL
);

-- Events Indexes
CREATE UNIQUE INDEX IF NOT EXISTS event_hash_index ON event(event_hash);

CREATE INDEX IF NOT EXISTS pubkey_index ON event(pubkey);

CREATE INDEX IF NOT EXISTS kind_index ON event(kind);

CREATE INDEX IF NOT EXISTS created_at_index ON event(created_at);

CREATE INDEX IF NOT EXISTS event_composite_index ON event(kind, created_at);

CREATE INDEX IF NOT EXISTS kind_pubkey_index ON event(kind, pubkey);

CREATE INDEX IF NOT EXISTS kind_created_at_index ON event(kind, created_at);

CREATE INDEX IF NOT EXISTS pubkey_created_at_index ON event(pubkey, created_at);

CREATE INDEX IF NOT EXISTS pubkey_kind_index ON event(pubkey, kind);