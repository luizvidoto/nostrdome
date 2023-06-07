CREATE TABLE contact (
    id INTEGER PRIMARY KEY,
    pubkey BLOB NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    unseen_messages INTEGER NOT NULL DEFAULT 0,
    petname TEXT,
    relay_url TEXT,
    last_message_content TEXT,
    last_message_date INTEGER
);

CREATE UNIQUE INDEX IF NOT EXISTS pubkey_index ON contact (pubkey);