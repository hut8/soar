-- Revert foreign keys back to aprs_messages_old

-- Drop the composite FK constraints
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_aprs_message_id_fkey;
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_aprs_message_id_fkey;

-- Restore old FK constraints pointing to aprs_messages_old
ALTER TABLE fixes
    ADD CONSTRAINT fixes_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id)
    REFERENCES aprs_messages_old(id)
    ON DELETE SET NULL;

ALTER TABLE receiver_statuses
    ADD CONSTRAINT receiver_statuses_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id)
    REFERENCES aprs_messages_old(id)
    ON DELETE SET NULL;
