-- Drop the aircraft_model table and associated objects
DROP TRIGGER IF EXISTS update_aircraft_model_updated_at ON aircraft_model;
DROP TABLE IF EXISTS aircraft_model;
