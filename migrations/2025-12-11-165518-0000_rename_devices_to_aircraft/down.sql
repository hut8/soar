-- ================================================================================
-- ROLLBACK: RENAME AIRCRAFT BACK TO DEVICES
-- ================================================================================
-- This migration rolls back the device → aircraft rename
-- ================================================================================

-- ============================================================================
-- STEP 1: RENAME TRIGGERS
-- ============================================================================

DROP TRIGGER IF EXISTS update_aircraft_updated_at ON aircraft;
CREATE TRIGGER update_devices_updated_at
    BEFORE UPDATE ON aircraft
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- STEP 2: RENAME FOREIGN KEY CONSTRAINTS
-- ============================================================================

-- Flights towed_by foreign key
ALTER TABLE flights
    DROP CONSTRAINT flights_towed_by_aircraft_id_fkey;
ALTER TABLE flights
    ADD CONSTRAINT flights_towed_by_device_id_fkey
    FOREIGN KEY (towed_by_aircraft_id) REFERENCES aircraft(id);

-- Flights foreign key
ALTER TABLE flights
    DROP CONSTRAINT flights_aircraft_id_fkey;
ALTER TABLE flights
    ADD CONSTRAINT flights_device_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) ON DELETE SET NULL;

-- Fixes foreign key
ALTER TABLE fixes
    DROP CONSTRAINT fixes_aircraft_id_fkey;
ALTER TABLE fixes
    ADD CONSTRAINT fixes_device_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) ON DELETE SET NULL;

-- Aircraft registrations foreign key
ALTER TABLE aircraft_registrations
    DROP CONSTRAINT aircraft_registrations_aircraft_id_fkey;
ALTER TABLE aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_device_id_fkey
    FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) ON DELETE SET NULL;

-- Aircraft table foreign key to clubs
ALTER TABLE aircraft RENAME CONSTRAINT aircraft_club_id_fkey TO devices_club_id_fkey;

-- ============================================================================
-- STEP 3: RENAME INDEXES ON RELATED TABLES
-- ============================================================================

ALTER INDEX idx_flights_towed_by_aircraft RENAME TO idx_flights_towed_by_device;
ALTER INDEX idx_flights_aircraft_id RENAME TO idx_flights_device_id;
ALTER INDEX idx_fixes_aircraft_received_at RENAME TO idx_fixes_device_received_at;
ALTER INDEX idx_aircraft_registrations_aircraft_id RENAME TO idx_aircraft_registrations_device_id;

-- ============================================================================
-- STEP 4: RENAME INDEXES ON AIRCRAFT_ANALYTICS TABLE
-- ============================================================================

ALTER INDEX idx_aircraft_analytics_z_score RENAME TO idx_device_analytics_z_score;
ALTER INDEX idx_aircraft_analytics_last_flight RENAME TO idx_device_analytics_last_flight;
ALTER INDEX idx_aircraft_analytics_flight_count_30d RENAME TO idx_device_analytics_flight_count_30d;

-- ============================================================================
-- STEP 5: RENAME INDEXES ON AIRCRAFT TABLE
-- ============================================================================

ALTER INDEX idx_aircraft_tracker_device_type RENAME TO idx_devices_tracker_device_type;
ALTER INDEX idx_aircraft_tracked RENAME TO idx_devices_tracked;
ALTER INDEX idx_aircraft_registration RENAME TO idx_devices_registration;
ALTER INDEX idx_aircraft_last_fix_at RENAME TO idx_devices_last_fix_at;
ALTER INDEX idx_aircraft_identified RENAME TO idx_devices_identified;
ALTER INDEX idx_aircraft_icao_model_code RENAME TO idx_devices_icao_model_code;
ALTER INDEX idx_aircraft_from_ddb RENAME TO idx_devices_from_ddb;
ALTER INDEX idx_aircraft_country_code RENAME TO idx_devices_country_code;
ALTER INDEX idx_aircraft_aircraft_model RENAME TO idx_devices_aircraft_model;

-- ============================================================================
-- STEP 6: RENAME PRIMARY KEYS AND UNIQUE CONSTRAINTS
-- ============================================================================

ALTER INDEX aircraft_analytics_pkey RENAME TO device_analytics_pkey;
ALTER INDEX aircraft_address_type_address_unique RENAME TO devices_address_type_address_unique;
ALTER INDEX aircraft_pkey RENAME TO devices_pkey;

-- ============================================================================
-- STEP 7: RENAME COLUMNS IN RELATED TABLES
-- ============================================================================

ALTER TABLE aircraft_analytics RENAME COLUMN aircraft_id TO device_id;
ALTER TABLE aircraft_registrations RENAME COLUMN aircraft_id TO device_id;
ALTER TABLE flights RENAME COLUMN towed_by_aircraft_id TO towed_by_device_id;
ALTER TABLE flights RENAME COLUMN aircraft_id TO device_id;
ALTER TABLE fixes_old RENAME COLUMN aircraft_id TO device_id;
ALTER TABLE fixes RENAME COLUMN aircraft_id TO device_id;

-- ============================================================================
-- STEP 8: RENAME MAIN TABLES
-- ============================================================================

ALTER TABLE aircraft_analytics RENAME TO device_analytics;
ALTER TABLE aircraft RENAME TO devices;

-- ================================================================================
-- ROLLBACK COMPLETE
-- ================================================================================
