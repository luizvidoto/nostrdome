CREATE TABLE relay (
    url TEXT PRIMARY KEY NOT NULL,
    -- Milliseconds unix timestamp
    created_at INTEGER DEFAULT NULL,
    updated_at INTEGER DEFAULT NULL,
    read INTEGER NOT NULL DEFAULT 1,
    write INTEGER NOT NULL DEFAULT 1,
    advertise INTEGER NOT NULL DEFAULT 0
);