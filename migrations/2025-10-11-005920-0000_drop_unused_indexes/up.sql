-- Drop unused indexes to improve performance and reduce database overhead

DROP INDEX IF EXISTS idx_aircraft_model_manufacturer_code;
DROP INDEX IF EXISTS idx_airports_ident;
DROP INDEX IF EXISTS idx_devices_device_type;
DROP INDEX IF EXISTS fixes_registration_idx;
DROP INDEX IF EXISTS flights_aircraft_id_idx;
DROP INDEX IF EXISTS idx_receivers_callsign;
DROP INDEX IF EXISTS users_email_idx;
