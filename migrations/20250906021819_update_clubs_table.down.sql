-- Add down migration script here
-- =========================================================
-- Revert clubs table updates
-- =========================================================

-- Drop indexes
DROP INDEX IF EXISTS clubs_base_location_idx;
DROP INDEX IF EXISTS clubs_home_base_airport_id_idx;
DROP INDEX IF EXISTS clubs_is_soaring_idx;

-- Drop columns in reverse order
DROP COLUMN IF EXISTS country_mail_code;
DROP COLUMN IF EXISTS county_mail_code;
DROP COLUMN IF EXISTS region_code;
DROP COLUMN IF EXISTS zip_code;
DROP COLUMN IF EXISTS state;
DROP COLUMN IF EXISTS city;
DROP COLUMN IF EXISTS street2;
DROP COLUMN IF EXISTS street1;
DROP COLUMN IF EXISTS base_location;
DROP COLUMN IF EXISTS home_base_airport_id;
DROP COLUMN IF EXISTS is_soaring;