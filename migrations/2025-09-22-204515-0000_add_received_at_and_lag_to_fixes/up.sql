-- Add received_at and lag columns to fixes table
-- received_at: When we received/processed the packet (Utc::now())
-- lag: The difference between received_at and timestamp (in milliseconds)

ALTER TABLE fixes
ADD COLUMN received_at TIMESTAMPTZ,
ADD COLUMN lag INTEGER;

-- For existing records, set received_at to the current timestamp value
-- and lag to 0 since we don't have the original received time
UPDATE fixes
SET received_at = timestamp,
    lag = 0
WHERE received_at IS NULL;

-- Make received_at NOT NULL after updating existing records
ALTER TABLE fixes
ALTER COLUMN received_at SET NOT NULL;
