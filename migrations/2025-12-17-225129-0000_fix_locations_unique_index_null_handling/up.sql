-- Fix the locations unique index to properly handle NULL values
--
-- Problem: The current index uses raw columns, so NULL != NULL means
-- duplicate addresses with NULL fields are not caught by the unique constraint.
-- This has allowed ~267,000 duplicate location records to accumulate.
--
-- Solution:
-- 1. Consolidate duplicate locations - keep oldest, update FKs, delete duplicates
-- 2. Create proper unique index with COALESCE to prevent future duplicates
--
-- Example issue this fixes:
--   Row 1: street1='123 Main', street2=NULL, city='Boston', state='MA', zip=NULL
--   Row 2: street1='123 Main', street2=NULL, city='Boston', state='MA', zip=NULL
-- Without COALESCE: Both rows are allowed (NULL != NULL)
-- With COALESCE: Second row is rejected as duplicate

-- Step 1: Create a mapping table of duplicate IDs to the canonical ID we'll keep
CREATE TEMP TABLE location_canonical_mapping AS
WITH duplicates AS (
    SELECT
        id,
        FIRST_VALUE(id) OVER (
            PARTITION BY
                COALESCE(street1, ''),
                COALESCE(street2, ''),
                COALESCE(city, ''),
                COALESCE(state, ''),
                COALESCE(zip_code, ''),
                COALESCE(country_mail_code, 'US')
            ORDER BY created_at ASC, id ASC
        ) as canonical_id
    FROM locations
)
SELECT id, canonical_id
FROM duplicates
WHERE id != canonical_id;

-- Step 2: Update all foreign key references to point to canonical locations
-- Aircraft registrations
UPDATE aircraft_registrations
SET location_id = m.canonical_id
FROM location_canonical_mapping m
WHERE aircraft_registrations.location_id = m.id;

-- Clubs
UPDATE clubs
SET location_id = m.canonical_id
FROM location_canonical_mapping m
WHERE clubs.location_id = m.id;

-- Airports
UPDATE airports
SET location_id = m.canonical_id
FROM location_canonical_mapping m
WHERE airports.location_id = m.id;

-- Flights (multiple FK columns)
UPDATE flights
SET start_location_id = m.canonical_id
FROM location_canonical_mapping m
WHERE flights.start_location_id = m.id;

UPDATE flights
SET end_location_id = m.canonical_id
FROM location_canonical_mapping m
WHERE flights.end_location_id = m.id;

UPDATE flights
SET takeoff_location_id = m.canonical_id
FROM location_canonical_mapping m
WHERE flights.takeoff_location_id = m.id;

UPDATE flights
SET landing_location_id = m.canonical_id
FROM location_canonical_mapping m
WHERE flights.landing_location_id = m.id;

-- Step 3: Delete duplicate location records
DELETE FROM locations
WHERE id IN (SELECT id FROM location_canonical_mapping);

-- Step 4: Drop the broken unique index
DROP INDEX IF EXISTS locations_address_unique_idx;

-- Step 5: Create the fixed unique index with COALESCE
CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    COALESCE(street1, ''),
    COALESCE(street2, ''),
    COALESCE(city, ''),
    COALESCE(state, ''),
    COALESCE(zip_code, ''),
    COALESCE(country_mail_code, 'US')
);
