CREATE TABLE message (
    msg_id INTEGER PRIMARY KEY,
    -- base64-encoded encrypted message
    content TEXT NOT NULL,
    from_pub TEXT NOT NULL,
    to_pub TEXT NOT NULL,
    -- event_id (optional)
    event_id INTEGER,
    -- UNIX timestamp as integer milliseconds
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    status INTEGER NOT NULL,
    relay_url TEXT
);

-- -- Message Indexes
CREATE INDEX IF NOT EXISTS msg_id_event_id_index ON message(msg_id, event_id);

CREATE INDEX IF NOT EXISTS from_pub_to_pub_index ON message(from_pub, to_pub);

CREATE INDEX IF NOT EXISTS created_at_index ON message(created_at);

CREATE INDEX IF NOT EXISTS msg_id_created_at_index ON message(msg_id, created_at);

CREATE INDEX IF NOT EXISTS msg_id_relay_url_index ON message(msg_id, relay_url);