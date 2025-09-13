-- Add back user_id column to devices table (rollback migration)
ALTER TABLE devices ADD COLUMN user_id UUID REFERENCES users(id);