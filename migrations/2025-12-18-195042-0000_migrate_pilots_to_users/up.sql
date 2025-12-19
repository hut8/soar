-- Migrate pilot data from pilots table to users table
-- This is the critical data migration that consolidates pilots into the unified users table

-- Step 1: Update existing users with their pilot qualifications
-- For users that are already linked to pilot records via pilots.user_id
UPDATE users u
SET
  is_licensed = p.is_licensed,
  is_instructor = p.is_instructor,
  is_tow_pilot = p.is_tow_pilot,
  is_examiner = p.is_examiner,
  updated_at = NOW()
FROM pilots p
WHERE p.user_id = u.id AND p.deleted_at IS NULL;

-- Step 2: Create user records for pilots without login accounts
-- Preserve pilot UUID as user UUID to maintain flight_pilots foreign key references
-- These users will have NULL email/password (cannot log in)
INSERT INTO users (
  id,
  first_name,
  last_name,
  email,
  password_hash,
  is_admin,
  club_id,
  email_verified,
  is_licensed,
  is_instructor,
  is_tow_pilot,
  is_examiner,
  deleted_at,
  created_at,
  updated_at,
  settings
)
SELECT
  p.id,                  -- Preserve UUID to maintain flight_pilots links
  p.first_name,
  p.last_name,
  NULL,                  -- No email (cannot login)
  NULL,                  -- No password (cannot login)
  FALSE,                 -- Not admin
  p.club_id,
  FALSE,                 -- Email not verified (no email)
  p.is_licensed,
  p.is_instructor,
  p.is_tow_pilot,
  p.is_examiner,
  p.deleted_at,          -- Preserve soft delete status
  p.created_at,
  p.updated_at,
  '{}'::jsonb            -- Default empty settings
FROM pilots p
WHERE p.user_id IS NULL;  -- Only pilots without existing user accounts

-- Step 3: Verify migration succeeded
DO $$
DECLARE
  pilot_count INTEGER;
  new_user_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO pilot_count FROM pilots WHERE deleted_at IS NULL;
  SELECT COUNT(*) INTO new_user_count
  FROM users
  WHERE deleted_at IS NULL
  AND (is_licensed OR is_instructor OR is_tow_pilot OR is_examiner);

  -- Log migration stats
  RAISE NOTICE 'Migration complete: % pilots, % users with pilot qualifications',
    pilot_count, new_user_count;

  IF new_user_count < pilot_count THEN
    RAISE WARNING 'User count (%) is less than pilot count (%). Some data may not have migrated.',
      new_user_count, pilot_count;
  END IF;
END $$;
