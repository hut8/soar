-- Add pending_registration column to aircraft table.
-- This stores a registration that could not be set because another aircraft
-- already owns it. A background task will periodically merge these duplicates
-- by reassigning fixes/flights and consolidating the records.
ALTER TABLE aircraft ADD COLUMN pending_registration TEXT;
