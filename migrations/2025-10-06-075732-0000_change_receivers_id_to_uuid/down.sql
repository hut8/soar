-- Revert receivers.id from UUID back to INT4
-- This migration is destructive and will lose UUID values

-- Step 1: Drop foreign key constraints
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_receiver_id_fkey;
ALTER TABLE receivers_links DROP CONSTRAINT IF EXISTS receivers_links_receiver_id_fkey;
ALTER TABLE receivers_photos DROP CONSTRAINT IF EXISTS receivers_photos_receiver_id_fkey;

-- Step 2: Drop primary key
ALTER TABLE receivers DROP CONSTRAINT IF EXISTS receivers_pkey;

-- Step 3: Add new INT4 columns
ALTER TABLE receivers ADD COLUMN id_new SERIAL;
ALTER TABLE receiver_statuses ADD COLUMN receiver_id_new INT4;
ALTER TABLE receivers_links ADD COLUMN receiver_id_new INT4;
ALTER TABLE receivers_photos ADD COLUMN receiver_id_new INT4;

-- Step 4: Create temporary mapping table
CREATE TEMP TABLE receiver_id_mapping AS
SELECT id, row_number() OVER (ORDER BY created_at) AS new_id
FROM receivers;

-- Step 5: Populate new INT4 columns with sequential IDs
UPDATE receivers r
SET id_new = m.new_id
FROM receiver_id_mapping m
WHERE r.id = m.id;

UPDATE receiver_statuses rs
SET receiver_id_new = m.new_id
FROM receivers r
JOIN receiver_id_mapping m ON r.id = m.id
WHERE rs.receiver_id = r.id;

UPDATE receivers_links rl
SET receiver_id_new = m.new_id
FROM receivers r
JOIN receiver_id_mapping m ON r.id = m.id
WHERE rl.receiver_id = r.id;

UPDATE receivers_photos rp
SET receiver_id_new = m.new_id
FROM receivers r
JOIN receiver_id_mapping m ON r.id = m.id
WHERE rp.receiver_id = r.id;

-- Step 6: Drop old UUID columns
ALTER TABLE receivers DROP COLUMN id;
ALTER TABLE receiver_statuses DROP COLUMN receiver_id;
ALTER TABLE receivers_links DROP COLUMN receiver_id;
ALTER TABLE receivers_photos DROP COLUMN receiver_id;

-- Step 7: Rename new columns to original names
ALTER TABLE receivers RENAME COLUMN id_new TO id;
ALTER TABLE receiver_statuses RENAME COLUMN receiver_id_new TO receiver_id;
ALTER TABLE receivers_links RENAME COLUMN receiver_id_new TO receiver_id;
ALTER TABLE receivers_photos RENAME COLUMN receiver_id_new TO receiver_id;

-- Step 8: Set NOT NULL constraints and primary key
ALTER TABLE receivers ALTER COLUMN id SET NOT NULL;
ALTER TABLE receivers ADD PRIMARY KEY (id);

-- Step 9: Add foreign key constraints
ALTER TABLE receiver_statuses ALTER COLUMN receiver_id SET NOT NULL;
ALTER TABLE receiver_statuses ADD CONSTRAINT receiver_statuses_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;

ALTER TABLE receivers_links ALTER COLUMN receiver_id SET NOT NULL;
ALTER TABLE receivers_links ADD CONSTRAINT receivers_links_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;

ALTER TABLE receivers_photos ALTER COLUMN receiver_id SET NOT NULL;
ALTER TABLE receivers_photos ADD CONSTRAINT receivers_photos_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;
