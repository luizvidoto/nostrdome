CREATE TABLE IF NOT EXISTS image_cache (
    id PRIMARY KEY,
    url TEXT NOT NULL,
    path TEXT NOT NULL,
    kind INTEGER NOT NULL
);

-- Events Indexes
CREATE UNIQUE INDEX IF NOT EXISTS url_index ON image_cache(url);