CREATE TABLE relay (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL UNIQUE,
    read INTEGER NOT NULL DEFAULT 1,
    write INTEGER NOT NULL DEFAULT 1,
    advertise INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX url ON relay (url);