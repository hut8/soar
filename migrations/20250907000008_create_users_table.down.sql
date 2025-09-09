-- Add down migration script here
-- =========================================================
-- Drop users table and related objects
-- =========================================================

DROP TRIGGER IF EXISTS update_users_updated_at ON users;
DROP FUNCTION IF EXISTS update_updated_at_column();
DROP TABLE IF EXISTS users;
DROP TYPE IF EXISTS access_level;
