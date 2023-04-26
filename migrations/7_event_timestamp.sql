CREATE TABLE IF NOT EXISTS last_event_received (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL
);

-- -- Insira ou atualize o registro com o ID 1
-- INSERT
--     OR REPLACE INTO last_event_received (id, timestamp)
-- VALUES
--     (
--         1,
--         (
--             strftime('%s', 'now') * 1000 + strftime('%f', 'now') * 1000
--         )
--     );