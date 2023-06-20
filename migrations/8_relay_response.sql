CREATE TABLE IF NOT EXISTS relay_response (
    event_id INTEGER NOT NULL,
    event_hash TEXT NOT NULL,
    relay_url TEXT NOT NULL,
    status INTEGER NOT NULL,
    error_message TEXT,
    PRIMARY KEY (event_id, event_hash, relay_url),
    FOREIGN KEY (event_id) REFERENCES event(event_id) ON DELETE CASCADE
);

-- Relay Responses Indexes
CREATE UNIQUE INDEX IF NOT EXISTS event_hash_relay_url_index ON relay_response(event_hash, relay_url);