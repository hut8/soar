-- Rename the 'destination' column to 'aprs_type' in the fixes table
-- This column stores the APRS packet destination/type field from the packet header
ALTER TABLE fixes RENAME COLUMN destination TO aprs_type;
