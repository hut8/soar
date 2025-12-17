-- Revert openaip_id from TEXT back to INTEGER
-- NOTE: This will fail if there are any non-integer values in the column

ALTER TABLE airspaces
    ALTER COLUMN openaip_id TYPE INTEGER USING openaip_id::INTEGER;

-- Restore original comment
COMMENT ON COLUMN airspaces.openaip_id IS 'OpenAIP internal ID - used for upserts during sync';
