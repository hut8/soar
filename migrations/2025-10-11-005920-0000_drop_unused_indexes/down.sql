-- Recreate indexes that were dropped

CREATE INDEX idx_aircraft_model_manufacturer_code ON aircraft_models (manufacturer_code);
CREATE INDEX idx_airports_ident ON airports (ident);
CREATE INDEX idx_devices_device_type ON devices (address_type);
CREATE INDEX fixes_registration_idx ON fixes (registration);
CREATE INDEX flights_aircraft_id_idx ON flights (device_id);
CREATE INDEX idx_receivers_callsign ON receivers (callsign);
CREATE INDEX users_email_idx ON users (email);
