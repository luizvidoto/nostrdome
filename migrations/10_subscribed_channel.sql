CREATE TABLE IF NOT EXISTS subscribed_channel (
    id INTEGER PRIMARY KEY,
    channel_id TEXT NOT NULL,
    subscribed_at INTEGER NOT NULL
);

-- Relay Responses Indexes
CREATE UNIQUE INDEX IF NOT EXISTS channel_id_index ON subscribed_channel(channel_id);