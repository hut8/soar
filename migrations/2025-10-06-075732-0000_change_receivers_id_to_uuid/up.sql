-- Change receivers.id from INT4 to UUID
-- Also update all foreign keys that reference receivers

-- Step 1: Drop foreign key constraints
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_receiver_id_fkey;
ALTER TABLE receivers_links DROP CONSTRAINT IF EXISTS receivers_links_receiver_id_fkey;
ALTER TABLE receivers_photos DROP CONSTRAINT IF EXISTS receivers_photos_receiver_id_fkey;

-- Step 2: Add new UUID columns
ALTER TABLE receivers ADD COLUMN id_new UUID DEFAULT gen_random_uuid();
ALTER TABLE receiver_statuses ADD COLUMN receiver_id_new UUID;
ALTER TABLE receivers_links ADD COLUMN receiver_id_new UUID;
ALTER TABLE receivers_photos ADD COLUMN receiver_id_new UUID;

-- Step 3: Populate new UUID columns with mapping from old IDs
-- For receivers, generate new UUIDs for existing rows
UPDATE receivers SET id_new = gen_random_uuid() WHERE id_new IS NULL;

-- For foreign key tables, map old IDs to new UUIDs
UPDATE receiver_statuses rs
SET receiver_id_new = r.id_new
FROM receivers r
WHERE rs.receiver_id = r.id;

UPDATE receivers_links rl
SET receiver_id_new = r.id_new
FROM receivers r
WHERE rl.receiver_id = r.id;

UPDATE receivers_photos rp
SET receiver_id_new = r.id_new
FROM receivers r
WHERE rp.receiver_id = r.id;

-- Step 4: Drop old columns
ALTER TABLE receivers DROP COLUMN id;
ALTER TABLE receiver_statuses DROP COLUMN receiver_id;
ALTER TABLE receivers_links DROP COLUMN receiver_id;
ALTER TABLE receivers_photos DROP COLUMN receiver_id;

-- Step 5: Rename new columns to original names
ALTER TABLE receivers RENAME COLUMN id_new TO id;
ALTER TABLE receiver_statuses RENAME COLUMN receiver_id_new TO receiver_id;
ALTER TABLE receivers_links RENAME COLUMN receiver_id_new TO receiver_id;
ALTER TABLE receivers_photos RENAME COLUMN receiver_id_new TO receiver_id;

-- Step 6: Set NOT NULL constraints and primary key
ALTER TABLE receivers ALTER COLUMN id SET NOT NULL;
ALTER TABLE receivers ADD PRIMARY KEY (id);

-- Step 7: Make foreign key columns NOT NULL where appropriate and add foreign key constraints
ALTER TABLE receiver_statuses ALTER COLUMN receiver_id SET NOT NULL;
ALTER TABLE receiver_statuses ADD CONSTRAINT receiver_statuses_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;

ALTER TABLE receivers_links ALTER COLUMN receiver_id SET NOT NULL;
ALTER TABLE receivers_links ADD CONSTRAINT receivers_links_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;

ALTER TABLE receivers_photos ALTER COLUMN receiver_id SET NOT NULL;
ALTER TABLE receivers_photos ADD CONSTRAINT receivers_photos_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;
