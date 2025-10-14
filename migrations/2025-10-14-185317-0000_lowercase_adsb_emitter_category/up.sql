-- Create new enum type with lowercase values
CREATE TYPE adsb_emitter_category_new AS ENUM (
    'a0', 'a1', 'a2', 'a3', 'a4', 'a5', 'a6', 'a7',
    'b0', 'b1', 'b2', 'b3', 'b4', 'b6', 'b7',
    'c0', 'c1', 'c2', 'c3', 'c4', 'c5'
);

-- Update the emitter_category column to use the new type
ALTER TABLE fixes
    ALTER COLUMN emitter_category TYPE adsb_emitter_category_new
    USING LOWER(emitter_category::TEXT)::adsb_emitter_category_new;

-- Drop the old enum type
DROP TYPE adsb_emitter_category;

-- Rename the new type to the original name
ALTER TYPE adsb_emitter_category_new RENAME TO adsb_emitter_category;
