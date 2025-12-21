-- Normalize all country codes to uppercase for consistency
-- This ensures country codes are stored in a standard format (ISO 3166-1 alpha-2)
UPDATE locations
SET country_code = UPPER(country_code)
WHERE country_code IS NOT NULL
  AND country_code != UPPER(country_code);
