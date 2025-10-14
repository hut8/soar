-- Revert back to uppercase enum values
CREATE TYPE adsb_emitter_category_old AS ENUM (
    'A0', 'A1', 'A2', 'A3', 'A4', 'A5', 'A6', 'A7',
    'B0', 'B1', 'B2', 'B3', 'B4', 'B6', 'B7',
    'C0', 'C1', 'C2', 'C3', 'C4', 'C5'
);

-- Update the emitter_category column to use the old type
ALTER TABLE fixes
    ALTER COLUMN emitter_category TYPE adsb_emitter_category_old
    USING UPPER(emitter_category::TEXT)::adsb_emitter_category_old;

-- Drop the lowercase enum type
DROP TYPE adsb_emitter_category;

-- Rename the old type back to the original name
ALTER TYPE adsb_emitter_category_old RENAME TO adsb_emitter_category;
