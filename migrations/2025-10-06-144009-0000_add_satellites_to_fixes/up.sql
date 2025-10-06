-- Add satellite tracking columns to fixes table
-- These fields track GPS satellite information for position quality assessment

ALTER TABLE fixes ADD COLUMN satellites_used SMALLINT;
ALTER TABLE fixes ADD COLUMN satellites_visible SMALLINT;

-- Add comments to explain the columns
COMMENT ON COLUMN fixes.satellites_used IS 'Number of satellites used in the position fix';
COMMENT ON COLUMN fixes.satellites_visible IS 'Number of satellites visible to the receiver';
