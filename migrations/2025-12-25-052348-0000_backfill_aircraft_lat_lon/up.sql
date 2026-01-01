-- Backfill aircraft latitude/longitude from most recent fix
-- This populates the latitude/longitude columns needed for the generated location_geom column
-- which is used for spatial bounding box queries
--
-- Optimized approach: Only scan fixes from the last 7 days using partition pruning
-- This is much faster than scanning the entire fixes table

WITH latest_fixes AS (
    SELECT DISTINCT ON (aircraft_id)
        aircraft_id,
        latitude,
        longitude
    FROM fixes
    WHERE latitude IS NOT NULL
      AND longitude IS NOT NULL
      AND timestamp >= NOW() - INTERVAL '7 days'  -- Use partition pruning
    ORDER BY aircraft_id, timestamp DESC
)
UPDATE aircraft
SET
    latitude = latest_fixes.latitude,
    longitude = latest_fixes.longitude
FROM latest_fixes
WHERE aircraft.id = latest_fixes.aircraft_id;
