CREATE TABLE IF NOT EXISTS progress
(
    channel_id  BIGINT PRIMARY KEY NOT NULL,
    progress    BIGINT,
    progress_ts BIGINT,
    done        INTEGER
);

CREATE TABLE messages
(
    message_id        BIGINT PRIMARY KEY NOT NULL,
    channel_id        BIGINT             NOT NULL,
    guild_id          BIGINT             NOT NULL,
    timestamp         BIGINT             NOT NULL,
    content           TEXT               NOT NULL,
    attachments_count INTEGER            NOT NULL
);