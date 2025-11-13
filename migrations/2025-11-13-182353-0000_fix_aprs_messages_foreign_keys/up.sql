-- Fix foreign key constraints to point to partitioned aprs_messages table
-- Both fixes and receiver_statuses need composite FKs matching the composite PK

-- Step 1: Drop old FK constraints pointing to aprs_messages_old
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_aprs_message_id_fkey;
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_aprs_message_id_fkey;

-- Step 2: Add new composite FK constraints pointing to aprs_messages (id, received_at)
ALTER TABLE fixes
    ADD CONSTRAINT fixes_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id, received_at)
    REFERENCES aprs_messages(id, received_at)
    ON DELETE SET NULL;

ALTER TABLE receiver_statuses
    ADD CONSTRAINT receiver_statuses_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id, received_at)
    REFERENCES aprs_messages(id, received_at)
    ON DELETE SET NULL;
