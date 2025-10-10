-- Add club_id back to fixes table
ALTER TABLE fixes
    ADD COLUMN club_id UUID REFERENCES clubs(id);

-- Remove club_id from devices table
ALTER TABLE devices
    DROP COLUMN club_id;
