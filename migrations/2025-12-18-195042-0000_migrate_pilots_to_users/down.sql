-- Rollback: Remove pilot data that was migrated to users table
-- WARNING: This will delete users that were created from pilots without login accounts

-- Delete users that were created from pilots (those without email/password)
DELETE FROM users
WHERE email IS NULL
  AND password_hash IS NULL
  AND (is_licensed OR is_instructor OR is_tow_pilot OR is_examiner);

-- Reset pilot qualifications for users that had linked pilot records
UPDATE users
SET
  is_licensed = FALSE,
  is_instructor = FALSE,
  is_tow_pilot = FALSE,
  is_examiner = FALSE
WHERE id IN (
  SELECT user_id FROM pilots WHERE user_id IS NOT NULL
);
