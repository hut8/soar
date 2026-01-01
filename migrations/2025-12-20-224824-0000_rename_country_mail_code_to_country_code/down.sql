-- Revert the unique index back to the old column name
DROP INDEX IF EXISTS locations_address_unique_idx;

CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    street1,
    street2,
    city,
    state,
    zip_code,
    COALESCE(country_mail_code, 'US')
);

-- Rename country_code back to country_mail_code in locations table
ALTER TABLE locations RENAME COLUMN country_code TO country_mail_code;
