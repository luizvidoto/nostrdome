CREATE TABLE IF NOT EXISTS channel_subscription (
    id INTEGER PRIMARY KEY,
    channel_id TEXT NOT NULL UNIQUE,
    subscribed_at INTEGER NOT NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS channel_id_index ON channel_subscription(channel_id);