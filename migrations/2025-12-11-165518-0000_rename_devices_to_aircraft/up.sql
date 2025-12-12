-- ================================================================================
-- RENAME DEVICES TO AIRCRAFT THROUGHOUT DATABASE
-- ================================================================================
-- This migration renames all "device" terminology to "aircraft" for clarity.
-- Devices are actually aircraft/transponders, so this improves semantic accuracy.
--
-- Changes:
-- - Tables: devices → aircraft, device_analytics → aircraft_analytics
-- - Columns: device_id → aircraft_id in multiple tables
-- - Indexes: All device-related indexes renamed
-- - Constraints: All device-related constraints renamed
--
-- IMPORTANT: This is a breaking change for the API
-- The backend code must be updated simultaneously
-- ================================================================================

-- ============================================================================
-- STEP 1: RENAME MAIN TABLES
-- ============================================================================

ALTER TABLE devices RENAME TO aircraft;
ALTER TABLE device_analytics RENAME TO aircraft_analytics;

-- ============================================================================
-- STEP 2: RENAME COLUMNS IN RELATED TABLES
-- ============================================================================

-- Fixes table (partitioned)
ALTER TABLE fixes RENAME COLUMN device_id TO aircraft_id;

-- Fixes old table (pre-partitioning)
ALTER TABLE fixes_old RENAME COLUMN device_id TO aircraft_id;

-- Flights table
ALTER TABLE flights RENAME COLUMN device_id TO aircraft_id;
ALTER TABLE flights RENAME COLUMN towed_by_device_id TO towed_by_aircraft_id;

-- Aircraft registrations table
ALTER TABLE aircraft_registrations RENAME COLUMN device_id TO aircraft_id;

-- Aircraft analytics table
ALTER TABLE aircraft_analytics RENAME COLUMN device_id TO aircraft_id;

-- ============================================================================
-- STEP 3: RENAME PRIMARY KEYS AND UNIQUE CONSTRAINTS
-- ============================================================================

-- Aircraft table primary key
ALTER INDEX devices_pkey RENAME TO aircraft_pkey;

-- Aircraft table unique constraint
ALTER INDEX devices_address_type_address_unique RENAME TO aircraft_address_type_address_unique;

-- Aircraft analytics primary key
ALTER INDEX device_analytics_pkey RENAME TO aircraft_analytics_pkey;

-- ============================================================================
-- STEP 4: RENAME INDEXES ON AIRCRAFT TABLE
-- ============================================================================

ALTER INDEX idx_devices_aircraft_model RENAME TO idx_aircraft_aircraft_model;
ALTER INDEX idx_devices_country_code RENAME TO idx_aircraft_country_code;
ALTER INDEX idx_devices_from_ddb RENAME TO idx_aircraft_from_ddb;
ALTER INDEX idx_devices_icao_model_code RENAME TO idx_aircraft_icao_model_code;
ALTER INDEX idx_devices_identified RENAME TO idx_aircraft_identified;
ALTER INDEX idx_devices_last_fix_at RENAME TO idx_aircraft_last_fix_at;
ALTER INDEX idx_devices_registration RENAME TO idx_aircraft_registration;
ALTER INDEX idx_devices_tracked RENAME TO idx_aircraft_tracked;
ALTER INDEX idx_devices_tracker_device_type RENAME TO idx_aircraft_tracker_device_type;

-- ============================================================================
-- STEP 5: RENAME INDEXES ON AIRCRAFT_ANALYTICS TABLE
-- ============================================================================

ALTER INDEX idx_device_analytics_flight_count_30d RENAME TO idx_aircraft_analytics_flight_count_30d;
ALTER INDEX idx_device_analytics_last_flight RENAME TO idx_aircraft_analytics_last_flight;
ALTER INDEX idx_device_analytics_z_score RENAME TO idx_aircraft_analytics_z_score;

-- ============================================================================
-- STEP 6: RENAME INDEXES ON RELATED TABLES
-- ============================================================================

ALTER INDEX idx_aircraft_registrations_device_id RENAME TO idx_aircraft_registrations_aircraft_id;
ALTER INDEX idx_fixes_device_received_at RENAME TO idx_fixes_aircraft_received_at;
ALTER INDEX idx_flights_device_id RENAME TO idx_flights_aircraft_id;
ALTER INDEX idx_flights_towed_by_device RENAME TO idx_flights_towed_by_aircraft;

-- ============================================================================
-- STEP 7: RENAME FOREIGN KEY CONSTRAINTS
-- ============================================================================

-- Aircraft table foreign key to clubs
ALTER TABLE aircraft RENAME CONSTRAINT devices_club_id_fkey TO aircraft_club_id_fkey;

-- Aircraft registrations foreign key to aircraft
ALTER TABLE aircraft_registrations
    DROP CONSTRAINT aircraft_registrations_device_id_fkey;
ALTER TABLE aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_aircraft_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) ON DELETE SET NULL;

-- Fixes foreign key to aircraft
ALTER TABLE fixes
    DROP CONSTRAINT fixes_device_id_fkey;
ALTER TABLE fixes
    ADD CONSTRAINT fixes_aircraft_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) ON DELETE SET NULL;

-- Flights foreign key to aircraft
ALTER TABLE flights
    DROP CONSTRAINT flights_device_id_fkey;
ALTER TABLE flights
    ADD CONSTRAINT flights_aircraft_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) ON DELETE SET NULL;

-- Flights towed_by foreign key to aircraft
ALTER TABLE flights
    DROP CONSTRAINT flights_towed_by_device_id_fkey;
ALTER TABLE flights
    ADD CONSTRAINT flights_towed_by_aircraft_id_fkey
    FOREIGN KEY (towed_by_aircraft_id) REFERENCES aircraft(id);

-- ============================================================================
-- STEP 8: RENAME TRIGGERS
-- ============================================================================

DROP TRIGGER IF EXISTS update_devices_updated_at ON aircraft;
CREATE TRIGGER update_aircraft_updated_at
    BEFORE UPDATE ON aircraft
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- MIGRATION COMPLETE
-- ============================================================================
-- All devices → aircraft renames complete
-- Remember to regenerate Diesel schema: diesel print-schema > src/schema.rs
-- ================================================================================
