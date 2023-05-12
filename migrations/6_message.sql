CREATE TABLE message (
    msg_id INTEGER PRIMARY KEY,
    -- base64-encoded encrypted message
    content TEXT NOT NULL,
    -- what chat it belongs to
    contact_pubkey TEXT NOT NULL,
    -- who sent it
    from_pubkey TEXT NOT NULL,
    -- who it's going to
    to_pubkey TEXT NOT NULL,
    -- database event_id
    event_id INTEGER NOT NULL REFERENCES event(event_id) ON UPDATE CASCADE ON DELETE CASCADE,
    event_hash BLOB NOT NULL,
    -- UNIX timestamp as integer milliseconds
    created_at INTEGER NOT NULL,
    confirmed_at INTEGER,
    status INTEGER NOT NULL,
    relay_url TEXT
);

-- -- Message Indexes
CREATE INDEX IF NOT EXISTS msg_id_event_id_index ON message(msg_id, event_id);

CREATE INDEX IF NOT EXISTS contact_pubkey_index ON message(contact_pubkey);

CREATE INDEX IF NOT EXISTS from_pubkey_to_pubkey_index ON message(from_pubkey, to_pubkey);

CREATE INDEX IF NOT EXISTS created_at_index ON message(created_at);

CREATE INDEX IF NOT EXISTS msg_id_created_at_index ON message(msg_id, created_at);

CREATE INDEX IF NOT EXISTS msg_id_relay_url_index ON message(msg_id, relay_url);