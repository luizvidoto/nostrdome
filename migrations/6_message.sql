CREATE TABLE message (
    msg_id INTEGER PRIMARY KEY,
    -- base64-encoded encrypted message
    content TEXT NOT NULL,
    -- what chat it belongs to
    contact_pubkey TEXT NOT NULL,
    -- bool
    is_users INTEGER NOT NULL,
    -- UNIX timestamp as integer milliseconds
    created_at INTEGER NOT NULL,
    status INTEGER NOT NULL,
    event_hash BLOB NOT NULL,
    -- only confirmed has values below
    event_id INTEGER,
    confirmed_at INTEGER,
    relay_url TEXT
);

-- -- Message Indexes
CREATE UNIQUE INDEX IF NOT EXISTS event_hash_index ON message(event_hash);

CREATE INDEX IF NOT EXISTS contact_pubkey_index ON message(contact_pubkey);