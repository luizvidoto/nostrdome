CREATE TABLE message (
    event_id INTEGER PRIMARY KEY,
    -- base64-encoded encrypted message
    content TEXT NOT NULL,
    -- what chat it belongs to
    chat_pubkey TEXT NOT NULL,
    is_users INTEGER NOT NULL,
    -- UNIX timestamp as integer milliseconds
    created_at INTEGER NOT NULL,
    status INTEGER NOT NULL,
    relay_url TEXT NOT NULL
);

-- -- Message Indexes
CREATE INDEX IF NOT EXISTS chat_pubkey_index ON message(chat_pubkey);