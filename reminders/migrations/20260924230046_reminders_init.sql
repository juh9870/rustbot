CREATE TABLE IF NOT EXISTS reminders
(
    id               INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    message          TEXT                              NOT NULL,
    author_id        BIGINT                            NOT NULL,
    channel_id       BIGINT                            NOT NULL,
    reminder_unix_ts BIGINT                            NOT NULL,
    reply_to         BIGINT
);

CREATE INDEX IF NOT EXISTS reminders_unix_ts on reminders (reminder_unix_ts);