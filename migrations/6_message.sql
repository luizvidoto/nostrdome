CREATE TABLE message (
    msg_id INTEGER PRIMARY KEY,
    -- base64-encoded encrypted message
    content TEXT NOT NULL,
    -- base64-encoded initialization vector
    iv TEXT NOT NULL,
    from_pub TEXT NOT NULL REFERENCES contact (pub_key),
    to_pub TEXT NOT NULL REFERENCES contact (pub_key),
    -- event_id (optional)
    event_id INTEGER,
    -- UNIX timestamp as integer
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    status INTEGER NOT NULL,
    relay_url TEXT NOT NULL REFERENCES relay (url)
);