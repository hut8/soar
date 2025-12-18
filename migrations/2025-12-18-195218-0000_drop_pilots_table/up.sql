-- Drop the pilots table after data has been migrated to users table
-- This is the final step in consolidating pilots and users

-- Verify no orphaned data before dropping
DO $$
DECLARE
  orphaned_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO orphaned_count
  FROM pilots p
  LEFT JOIN users u ON u.id = p.id OR u.id = p.user_id
  WHERE u.id IS NULL;

  IF orphaned_count > 0 THEN
    RAISE EXCEPTION 'Cannot drop pilots table: % orphaned records found. Check data migration.', orphaned_count;
  END IF;

  RAISE NOTICE 'Pre-drop verification passed: no orphaned pilot records found';
END $$;

-- Drop the pilots table and all its dependencies
DROP TABLE IF EXISTS pilots CASCADE;

-- Log completion
DO $$
BEGIN
  RAISE NOTICE 'Pilots table dropped successfully. All pilot data is now in the users table.';
END $$;
