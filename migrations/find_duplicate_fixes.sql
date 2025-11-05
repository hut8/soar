-- Find duplicate fixes
-- Run this to see how many duplicates exist before running the deduplication migrations
--
-- Usage: psql soar -f migrations/find_duplicate_fixes.sql

-- Count of duplicate groups
SELECT
    COUNT(*) as duplicate_groups,
    SUM(duplicate_count - 1) as total_duplicates_to_delete
FROM (
    SELECT
        device_id,
        timestamp,
        COUNT(*) as duplicate_count
    FROM fixes
    GROUP BY device_id, timestamp
    HAVING COUNT(*) > 1
) duplicates;

-- Show examples of duplicate groups (limit 10)
SELECT
    device_id,
    timestamp,
    COUNT(*) as duplicate_count,
    ARRAY_AGG(id ORDER BY id) as duplicate_ids
FROM fixes
GROUP BY device_id, timestamp
HAVING COUNT(*) > 1
ORDER BY COUNT(*) DESC
LIMIT 10;
