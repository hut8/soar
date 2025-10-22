-- Re-add county_mail_code column
ALTER TABLE locations ADD COLUMN IF NOT EXISTS county_mail_code TEXT;

-- Drop simplified index
DROP INDEX IF EXISTS locations_address_unique_idx;

-- Recreate old COALESCE-based unique index
CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    COALESCE(street1, ''::text),
    COALESCE(street2, ''::text),
    COALESCE(city, ''::text),
    COALESCE(state, ''::text),
    COALESCE(zip_code, ''::text),
    COALESCE(country_mail_code, 'US'::text)
);
