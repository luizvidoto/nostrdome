-- Tag Table
-- Tag values are stored as either a BLOB (if they come in as a
-- hex-string), or TEXT otherwise.
-- This means that searches need to select the appropriate column.
-- We duplicate the kind/created_at to make indexes much more efficient.
CREATE TABLE IF NOT EXISTS tag (
    tag_id INTEGER PRIMARY KEY,
    -- an event ID that contains a tag
    event_id INTEGER NOT NULL,
    -- the tag name ("p", "e", whatever)
    name TEXT,
    -- the tag value, if not hex.
    value TEXT,
    -- the tag value, if it can be interpreted as a lowercase hex string.
    value_hex BLOB,
    -- when the event was authored
    created_at INTEGER NOT NULL,
    -- event kind
    kind INTEGER NOT NULL,
    FOREIGN KEY(event_id) REFERENCES event(event_id) ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS tag_val_index ON tag(value);

CREATE INDEX IF NOT EXISTS tag_composite_index ON tag(event_id, name, value);

CREATE INDEX IF NOT EXISTS tag_name_eid_index ON tag(name, event_id, value);

CREATE INDEX IF NOT EXISTS tag_covering_index ON tag(name, kind, value, created_at, event_id);