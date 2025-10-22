-- Revert changes: restore location_id foreign key and make receiver_id nullable

-- Step 1: Drop indexes we created
DROP INDEX IF EXISTS idx_aprs_messages_receiver_id;
DROP INDEX IF EXISTS idx_receivers_location;

-- Step 2: Make receiver_id nullable again in aprs_messages
ALTER TABLE aprs_messages ALTER COLUMN receiver_id DROP NOT NULL;

-- Step 3: Add back location_id column to receivers
ALTER TABLE receivers ADD COLUMN location_id UUID;

-- Step 4: Add back the foreign key constraint
ALTER TABLE receivers ADD CONSTRAINT receivers_location_id_fkey
    FOREIGN KEY (location_id) REFERENCES locations(id);

-- Step 5: Drop the location column from receivers
ALTER TABLE receivers DROP COLUMN location;
