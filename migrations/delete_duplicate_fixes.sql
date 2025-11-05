-- Delete duplicate fixes (keeps oldest record)
-- Run find_duplicate_fixes.sql first to see what will be deleted
--
-- WARNING: This is IRREVERSIBLE. Back up your database first!
--
-- Usage: psql soar -f migrations/delete_duplicate_fixes.sql

BEGIN;

-- Show what will be deleted
SELECT
    'Will delete ' || COUNT(*) || ' duplicate fixes' as action
FROM fixes a
WHERE EXISTS (
    SELECT 1
    FROM fixes b
    WHERE b.device_id = a.device_id
      AND b.timestamp = a.timestamp
      AND b.id < a.id
);

-- Delete duplicates (keeping oldest id)
DELETE FROM fixes a
USING fixes b
WHERE a.id > b.id
  AND a.device_id = b.device_id
  AND a.timestamp = b.timestamp;

-- Show summary
SELECT
    'Deleted ' || COUNT(*) || ' rows' as summary
FROM (SELECT 1) x;

-- COMMIT or ROLLBACK
-- Remove the ROLLBACK and uncomment COMMIT when you're ready to apply changes
ROLLBACK;
-- COMMIT;
