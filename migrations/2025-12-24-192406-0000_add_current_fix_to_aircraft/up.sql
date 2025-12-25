-- Add current_fix JSONB column to store the latest fix for each aircraft
ALTER TABLE aircraft ADD COLUMN current_fix JSONB;
