-- Change openaip_id from INTEGER to TEXT to match OpenAIP API
-- The OpenAIP API returns MongoDB ObjectId strings (e.g., "507f1f77bcf86cd799439011")
-- not integers, so we need to store them as TEXT.

ALTER TABLE airspaces
    ALTER COLUMN openaip_id TYPE TEXT;

-- Update the column comment to clarify the format
COMMENT ON COLUMN airspaces.openaip_id IS 'OpenAIP MongoDB ObjectId (string) - used for upserts during sync';
