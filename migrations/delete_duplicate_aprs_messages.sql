-- Delete duplicate APRS messages (keeps oldest record)
-- Run find_duplicate_aprs_messages.sql first to see what will be deleted
--
-- WARNING: This is IRREVERSIBLE. Back up your database first!
--
-- Usage: psql soar -f migrations/delete_duplicate_aprs_messages.sql

BEGIN;

-- Show what will be deleted
SELECT
    'Will delete ' || COUNT(*) || ' duplicate APRS messages' as action
FROM aprs_messages a
WHERE EXISTS (
    SELECT 1
    FROM aprs_messages b
    WHERE b.receiver_id = a.receiver_id
      AND b.received_at = a.received_at
      AND b.raw_message_hash = a.raw_message_hash
      AND b.id < a.id
);

-- Delete duplicates (keeping oldest id)
DELETE FROM aprs_messages a
USING aprs_messages b
WHERE a.id > b.id
  AND a.receiver_id = b.receiver_id
  AND a.received_at = b.received_at
  AND a.raw_message_hash = b.raw_message_hash;

-- Show summary
SELECT
    'Deleted ' || COUNT(*) || ' rows' as summary
FROM (SELECT 1) x;

-- COMMIT or ROLLBACK
-- Remove the ROLLBACK and uncomment COMMIT when you're ready to apply changes
ROLLBACK;
-- COMMIT;
