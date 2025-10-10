-- Remove settings column from users table
ALTER TABLE users DROP COLUMN IF EXISTS settings;
