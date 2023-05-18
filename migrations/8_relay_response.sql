CREATE TABLE IF NOT EXISTS relay_response (
    relay_response_id INTEGER PRIMARY KEY,
    event_id INTEGER NOT NULL,
    event_hash BLOB NOT NULL,
    relay_url TEXT NOT NULL,
    status INTEGER NOT NULL,
    error_message TEXT,
    FOREIGN KEY (event_id) REFERENCES event(event_id) ON DELETE CASCADE
);

-- Relay Responses Indexes
-- CREATE UNIQUE INDEX IF NOT EXISTS event_hash_relay_url_index ON relay_response(event_hash, relay_url);