-- Revert LightSportType enum migration

-- Drop the light sport type column
ALTER TABLE aircraft_registrations
DROP COLUMN light_sport_type;

-- Drop the enum type
DROP TYPE light_sport_type;