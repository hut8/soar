-- Create LightSportType enum
CREATE TYPE light_sport_type AS ENUM (
    'airplane',
    'glider',
    'lighter_than_air',
    'power_parachute',
    'weight_shift_control'
);

-- Add new column with the enum type (nullable)
ALTER TABLE aircraft_registrations
ADD COLUMN light_sport_type light_sport_type;
