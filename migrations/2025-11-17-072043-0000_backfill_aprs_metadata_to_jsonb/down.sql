-- Rollback: Clear source_metadata JSONB column
-- Data is still in original columns so this is non-destructive

UPDATE fixes
SET source_metadata = NULL
WHERE source_metadata IS NOT NULL
  AND source_metadata->>'protocol' = 'aprs';
