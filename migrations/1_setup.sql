CREATE TABLE IF NOT EXISTS local_settings (
    schema_version INTEGER DEFAULT 0,
    encrypted_private_key TEXT DEFAULT NULL
);

INSERT INTO
    local_settings (schema_version, encrypted_private_key)
VALUES
    (0, "");