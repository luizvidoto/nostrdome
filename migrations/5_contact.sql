CREATE TABLE contact (
    pubkey TEXT PRIMARY KEY,
    petname TEXT,
    relay_url TEXT,
    profile_image TEXT,
    status INTEGER NOT NULL,
    unseen_messages INTEGER NOT NULL DEFAULT 0
);