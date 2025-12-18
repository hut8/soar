-- Revert to the broken index (without COALESCE)
-- This allows duplicate addresses with NULL fields, which is incorrect behavior

DROP INDEX IF EXISTS locations_address_unique_idx;

CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    street1,
    street2,
    city,
    state,
    zip_code,
    country_mail_code
);
