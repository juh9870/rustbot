ALTER TABLE messages
    ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0;

CREATE INDEX messages_deleted_idx on messages (deleted);
CREATE INDEX messages_deleted_timestamp on messages (timestamp);
CREATE INDEX messages_deleted_attachments_count on messages (attachments_count);

CREATE TABLE deletion_priority
(
    message_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    priority   BIGINT NOT NULL
);

CREATE INDEX deletion_priority_prio on deletion_priority (priority);