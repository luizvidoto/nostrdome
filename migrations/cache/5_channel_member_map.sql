CREATE TABLE IF NOT EXISTS channel_member_map (
    channel_id TEXT NOT NULL,
    public_key BLOB NOT NULL,
    PRIMARY KEY (channel_id, public_key)
);