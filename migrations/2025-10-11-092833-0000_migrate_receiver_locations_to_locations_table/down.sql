-- Restore latitude/longitude to receivers from the locations table before removing location_id references

-- Copy geolocation data back to receivers
UPDATE receivers r
SET
    latitude = l.geolocation[1],
    longitude = l.geolocation[0],
    updated_at = now()
FROM locations l
WHERE r.location_id = l.id
  AND l.geolocation IS NOT NULL;

-- Restore the original unique constraint (without the WHERE clause)
DROP INDEX IF EXISTS locations_address_unique_idx;
CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    COALESCE(street1, ''),
    COALESCE(street2, ''),
    COALESCE(city, ''),
    COALESCE(state, ''),
    COALESCE(zip_code, ''),
    COALESCE(country_mail_code, 'US')
);

-- Note: We don't delete the location records as they might be referenced by other tables
-- Setting location_id to NULL is handled by the down migration that removes the column
