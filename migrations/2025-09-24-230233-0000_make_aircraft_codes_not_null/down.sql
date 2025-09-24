-- Undo making manufacturer_code, model_code, and series_code NOT NULL
ALTER TABLE aircraft_registrations
ALTER COLUMN manufacturer_code DROP NOT NULL;

ALTER TABLE aircraft_registrations
ALTER COLUMN model_code DROP NOT NULL;

ALTER TABLE aircraft_registrations
ALTER COLUMN series_code DROP NOT NULL;
