-- Tags Table
CREATE TABLE IF NOT EXISTS tags (
    tag_id INTEGER PRIMARY KEY,
    event_id BLOB NOT NULL,
    -- the tag name ("p", "e", whatever)
    kind TEXT NOT NULL,
    -- tag contents
    value BLOB,
    FOREIGN KEY(event_id) REFERENCES event(event_id) ON UPDATE CASCADE ON DELETE CASCADE
);

-- Tags Indexes
CREATE INDEX IF NOT EXISTS tag_val_index ON tags(value);

CREATE UNIQUE INDEX IF NOT EXISTS tag_composite_index ON tags(event_id, kind, value);

CREATE UNIQUE INDEX IF NOT EXISTS tag_kind_eid_index ON tags(kind, event_id, value);