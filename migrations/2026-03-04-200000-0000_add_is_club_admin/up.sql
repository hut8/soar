-- safety-assured:start
-- ADD COLUMN with constant DEFAULT is safe on PostgreSQL 11+
ALTER TABLE users ADD COLUMN is_club_admin BOOLEAN NOT NULL DEFAULT false;
-- safety-assured:end
