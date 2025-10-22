-- Drop the old COALESCE-based unique index
DROP INDEX IF EXISTS locations_address_unique_idx;

-- Create simplified unique index without COALESCE
-- This allows proper NULL matching and eliminates unnecessary empty string coalescing
CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    street1,
    street2,
    city,
    state,
    zip_code,
    country_mail_code
);

-- Drop county_mail_code column - never used
ALTER TABLE locations DROP COLUMN IF EXISTS county_mail_code;
