-- Add club_id to devices table
ALTER TABLE devices
    ADD COLUMN club_id UUID REFERENCES clubs(id);

-- Remove club_id from fixes table
ALTER TABLE fixes
    DROP COLUMN club_id;
