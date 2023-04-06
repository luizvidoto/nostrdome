CREATE TABLE IF NOT EXISTS event (
    event_id INTEGER PRIMARY KEY,
    -- 4-byte hash
    event_hash BLOB NOT NULL,
    -- when the event was first seen (not authored!) (seconds since 1970)
    first_seen INTEGER NOT NULL,
    -- when the event was authored
    created_at INTEGER NOT NULL,
    -- when the event expires and may be deleted
    expires_at INTEGER,
    -- author pubkey
    author BLOB NOT NULL,
    -- delegator pubkey (NIP-26)
    delegated_by BLOB,
    -- event kind
    kind INTEGER NOT NULL,
    content TEXT NOT NULL -- serialized json of event object
);