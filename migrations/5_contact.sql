CREATE TABLE contact (
    pubkey TEXT PRIMARY KEY,
    petname TEXT,
    relay_url TEXT,
    profile_image TEXT,
    status INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    unseen_messages INTEGER NOT NULL DEFAULT 0,
    last_message_content TEXT,
    last_message_date INTEGER
);

-- -- Contact Indexes