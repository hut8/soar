-- Remove h3_postgis extension (must be removed before h3)
DROP EXTENSION IF EXISTS h3_postgis CASCADE;

-- Remove h3 extension
DROP EXTENSION IF EXISTS h3 CASCADE;
