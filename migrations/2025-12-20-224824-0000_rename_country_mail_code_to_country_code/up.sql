-- Rename country_mail_code to country_code in locations table
ALTER TABLE locations RENAME COLUMN country_mail_code TO country_code;

-- Update the unique index to use the new column name
DROP INDEX IF EXISTS locations_address_unique_idx;

CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    street1,
    street2,
    city,
    state,
    zip_code,
    COALESCE(country_code, 'US')
);
