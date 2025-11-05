-- Find duplicate APRS messages
-- Run this to see how many duplicates exist before running the deduplication migrations
--
-- Usage: psql soar -f migrations/find_duplicate_aprs_messages.sql

-- Count of duplicate groups
SELECT
    COUNT(*) as duplicate_groups,
    SUM(duplicate_count - 1) as total_duplicates_to_delete
FROM (
    SELECT
        receiver_id,
        received_at,
        raw_message_hash,
        COUNT(*) as duplicate_count
    FROM aprs_messages
    GROUP BY receiver_id, received_at, raw_message_hash
    HAVING COUNT(*) > 1
) duplicates;

-- Show examples of duplicate groups (limit 10)
SELECT
    receiver_id,
    received_at,
    raw_message_hash,
    COUNT(*) as duplicate_count,
    ARRAY_AGG(id ORDER BY id) as duplicate_ids
FROM aprs_messages
GROUP BY receiver_id, received_at, raw_message_hash
HAVING COUNT(*) > 1
ORDER BY COUNT(*) DESC
LIMIT 10;
