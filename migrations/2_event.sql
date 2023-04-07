CREATE TABLE IF NOT EXISTS event (
    event_id INTEGER PRIMARY KEY,
    -- 4-byte hash
    event_hash BLOB NOT NULL,
    -- author pubkey
    author BLOB NOT NULL,
    -- when the event was authored
    created_at INTEGER NOT NULL,
    -- event kind
    kind INTEGER NOT NULL,
    -- serialized json of event object 
    content TEXT NOT NULL,
    -- nip03
    ots TEXT
);