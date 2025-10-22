-- Replace location_id foreign key with direct geography column in receivers
-- Make receiver_id NOT NULL in aprs_messages

-- Step 1: Add location geography column to receivers
ALTER TABLE receivers ADD COLUMN location geography(Point, 4326);

-- Step 2: Copy existing location data from locations table to receivers
UPDATE receivers r
SET location = (
    SELECT ST_SetSRID(ST_MakePoint(
        ST_X(l.geolocation::geometry),
        ST_Y(l.geolocation::geometry)
    ), 4326)::geography
    FROM locations l
    WHERE l.id = r.location_id
    AND l.geolocation IS NOT NULL
);

-- Step 3: Drop the foreign key constraint
ALTER TABLE receivers DROP CONSTRAINT IF EXISTS receivers_location_id_fkey;

-- Step 4: Drop the location_id column
ALTER TABLE receivers DROP COLUMN location_id;

-- Step 5: Make receiver_id NOT NULL in aprs_messages
-- First delete any orphaned messages
DELETE FROM aprs_messages WHERE receiver_id IS NULL;

-- Make it NOT NULL
ALTER TABLE aprs_messages ALTER COLUMN receiver_id SET NOT NULL;

-- Step 6: Create spatial index on receivers.location
CREATE INDEX idx_receivers_location ON receivers USING GIST (location);

-- Step 7: Create index on aprs_messages.receiver_id for faster lookups
CREATE INDEX IF NOT EXISTS idx_aprs_messages_receiver_id ON aprs_messages (receiver_id);
