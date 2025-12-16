--
-- PostgreSQL database dump
--

\restrict SOAR

-- Dumped from database version 17.7 (Ubuntu 17.7-3.pgdg22.04+1)
-- Dumped by pg_dump version 17.7 (Ubuntu 17.7-3.pgdg22.04+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: partman; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA partman;


--
-- Name: pg_partman; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_partman WITH SCHEMA partman;


--
-- Name: pg_trgm; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public;


--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: postgis; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS postgis WITH SCHEMA public;


--
-- Name: uuid-ossp; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;


--
-- Name: address_type; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.address_type AS ENUM (
    'flarm',
    'ogn',
    'icao',
    'unknown'
);


--
-- Name: adsb_emitter_category; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.adsb_emitter_category AS ENUM (
    'a0',
    'a1',
    'a2',
    'a3',
    'a4',
    'a5',
    'a6',
    'a7',
    'b0',
    'b1',
    'b2',
    'b3',
    'b4',
    'b6',
    'b7',
    'c0',
    'c1',
    'c2',
    'c3',
    'c4',
    'c5'
);


--
-- Name: aircraft_type; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.aircraft_type AS ENUM (
    'glider',
    'balloon',
    'blimp_dirigible',
    'fixed_wing_single_engine',
    'fixed_wing_multi_engine',
    'rotorcraft',
    'weight_shift_control',
    'powered_parachute',
    'gyroplane',
    'hybrid_lift',
    'other'
);


--
-- Name: aircraft_type_ogn; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.aircraft_type_ogn AS ENUM (
    'reserved',
    'glider',
    'tow_tug',
    'helicopter_gyro',
    'skydiver_parachute',
    'drop_plane',
    'hang_glider',
    'paraglider',
    'recip_engine',
    'jet_turboprop',
    'unknown',
    'balloon',
    'airship',
    'uav',
    'static_obstacle'
);


--
-- Name: airspace_class; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.airspace_class AS ENUM (
    'A',
    'B',
    'C',
    'D',
    'E',
    'F',
    'G',
    'SUA'
);


--
-- Name: airspace_type; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.airspace_type AS ENUM (
    'Restricted',
    'Danger',
    'Prohibited',
    'CTR',
    'TMZ',
    'RMZ',
    'TMA',
    'ATZ',
    'MATZ',
    'Airway',
    'MTR',
    'AlertArea',
    'WarningArea',
    'ProtectedArea',
    'HTZ',
    'GliderProhibited',
    'GliderSector',
    'NoGliders',
    'WaveWindow',
    'Other',
    'FIR',
    'UIR',
    'ADIZ',
    'ATZ_P',
    'ATZ_MBZ',
    'TFR',
    'TRA',
    'TSA',
    'FIS',
    'UAS',
    'RFFS',
    'Sport',
    'DropZone',
    'Gliding',
    'MilitaryOps',
    'NotAssigned'
);


--
-- Name: airworthiness_class; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.airworthiness_class AS ENUM (
    'standard',
    'limited',
    'restricted',
    'experimental',
    'provisional',
    'multiple',
    'primary',
    'special_flight_permit',
    'light_sport'
);


--
-- Name: altitude_reference; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.altitude_reference AS ENUM (
    'MSL',
    'AGL',
    'STD',
    'GND',
    'UNL'
);


--
-- Name: light_sport_type; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.light_sport_type AS ENUM (
    'airplane',
    'glider',
    'lighter_than_air',
    'power_parachute',
    'weight_shift_control'
);


--
-- Name: message_source; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.message_source AS ENUM (
    'ogn',
    'adsb'
);


--
-- Name: registrant_type; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.registrant_type AS ENUM (
    'individual',
    'partnership',
    'corporation',
    'co_owned',
    'government',
    'llc',
    'non_citizen_corporation',
    'non_citizen_co_owned',
    'unknown'
);


--
-- Name: compute_aprs_message_hash(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.compute_aprs_message_hash() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF NEW.raw_message_hash IS NULL THEN
        NEW.raw_message_hash := digest(NEW.raw_message, 'sha256');
    END IF;
    RETURN NEW;
END;
$$;


--
-- Name: get_duration_bucket(integer); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.get_duration_bucket(duration_seconds integer) RETURNS character varying
    LANGUAGE plpgsql IMMUTABLE
    AS $$
DECLARE
    duration_minutes INT;
BEGIN
    duration_minutes := duration_seconds / 60;

    IF duration_minutes < 5 THEN RETURN '0-5min';
    ELSIF duration_minutes < 10 THEN RETURN '5-10min';
    ELSIF duration_minutes < 15 THEN RETURN '10-15min';
    ELSIF duration_minutes < 30 THEN RETURN '15-30min';
    ELSIF duration_minutes < 60 THEN RETURN '30-60min';
    ELSIF duration_minutes < 90 THEN RETURN '60-90min';
    ELSIF duration_minutes < 120 THEN RETURN '90-120min';
    ELSIF duration_minutes < 150 THEN RETURN '120-150min';
    ELSIF duration_minutes < 180 THEN RETURN '150-180min';
    ELSIF duration_minutes < 210 THEN RETURN '180-210min';
    ELSIF duration_minutes < 240 THEN RETURN '210-240min';
    ELSIF duration_minutes < 270 THEN RETURN '240-270min';
    ELSIF duration_minutes < 300 THEN RETURN '270-300min';
    ELSIF duration_minutes < 330 THEN RETURN '300-330min';
    ELSIF duration_minutes < 360 THEN RETURN '330-360min';
    ELSE RETURN '360+min';
    END IF;
END;
$$;


--
-- Name: get_flight_duration_seconds(timestamp with time zone, timestamp with time zone); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.get_flight_duration_seconds(takeoff timestamp with time zone, landing timestamp with time zone) RETURNS integer
    LANGUAGE plpgsql IMMUTABLE
    AS $$
BEGIN
    IF takeoff IS NULL OR landing IS NULL THEN
        RETURN 0;
    END IF;
    RETURN EXTRACT(EPOCH FROM (landing - takeoff))::INT;
END;
$$;


--
-- Name: update_airport_analytics_daily(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_airport_analytics_daily() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    affected_date DATE;
    old_date DATE;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.departure_airport_id IS NOT DISTINCT FROM NEW.departure_airport_id AND
       OLD.arrival_airport_id IS NOT DISTINCT FROM NEW.arrival_airport_id
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_date := DATE(NEW.takeoff_time);

        -- Update departure airport
        IF NEW.departure_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.departure_airport_id,
                affected_date,
                a.ident,
                a.name,
                1,
                0
            FROM airports a
            WHERE a.id = NEW.departure_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                departure_count = airport_analytics_daily.departure_count + 1,
                updated_at = NOW();
        END IF;

        -- Update arrival airport
        IF NEW.arrival_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.arrival_airport_id,
                affected_date,
                a.ident,
                a.name,
                0,
                1
            FROM airports a
            WHERE a.id = NEW.arrival_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                arrival_count = airport_analytics_daily.arrival_count + 1,
                updated_at = NOW();
        END IF;

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_date := DATE(OLD.takeoff_time);
        affected_date := DATE(NEW.takeoff_time);

        -- Remove old departure
        IF OLD.departure_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                departure_count = GREATEST(0, departure_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.departure_airport_id AND date = old_date;
        END IF;

        -- Remove old arrival
        IF OLD.arrival_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                arrival_count = GREATEST(0, arrival_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.arrival_airport_id AND date = old_date;
        END IF;

        -- Add new departure
        IF NEW.departure_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.departure_airport_id,
                affected_date,
                a.ident,
                a.name,
                1,
                0
            FROM airports a
            WHERE a.id = NEW.departure_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                departure_count = airport_analytics_daily.departure_count + 1,
                updated_at = NOW();
        END IF;

        -- Add new arrival
        IF NEW.arrival_airport_id IS NOT NULL THEN
            INSERT INTO airport_analytics_daily (airport_id, date, airport_ident, airport_name, departure_count, arrival_count)
            SELECT
                NEW.arrival_airport_id,
                affected_date,
                a.ident,
                a.name,
                0,
                1
            FROM airports a
            WHERE a.id = NEW.arrival_airport_id
            ON CONFLICT (airport_id, date) DO UPDATE SET
                arrival_count = airport_analytics_daily.arrival_count + 1,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_date := DATE(OLD.takeoff_time);

        -- Remove departure
        IF OLD.departure_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                departure_count = GREATEST(0, departure_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.departure_airport_id AND date = old_date;
        END IF;

        -- Remove arrival
        IF OLD.arrival_airport_id IS NOT NULL THEN
            UPDATE airport_analytics_daily SET
                arrival_count = GREATEST(0, arrival_count - 1),
                updated_at = NOW()
            WHERE airport_id = OLD.arrival_airport_id AND date = old_date;
        END IF;
    END IF;

    RETURN NEW;
END;
$$;


--
-- Name: update_airport_location(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_airport_location() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF NEW.latitude_deg IS NOT NULL AND NEW.longitude_deg IS NOT NULL THEN
        NEW.location = ST_SetSRID(ST_MakePoint(NEW.longitude_deg, NEW.latitude_deg), 4326)::geography;
    ELSE
        NEW.location = NULL;
    END IF;
    RETURN NEW;
END;
$$;


--
-- Name: update_club_analytics_daily(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_club_analytics_daily() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    affected_date DATE;
    old_date DATE;
    old_club UUID;
    new_club UUID;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.club_id IS NOT DISTINCT FROM NEW.club_id AND
       OLD.towed_by_aircraft_id IS NOT DISTINCT FROM NEW.towed_by_aircraft_id
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time or club
        IF NEW.takeoff_time IS NULL OR NEW.club_id IS NULL THEN
            RETURN NEW;
        END IF;

        affected_date := DATE(NEW.takeoff_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, total_airtime_seconds, tow_count)
        SELECT
            NEW.club_id,
            affected_date,
            c.name,
            1,
            flight_duration,
            CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END
        FROM clubs c
        WHERE c.id = NEW.club_id
        ON CONFLICT (club_id, date) DO UPDATE SET
            flight_count = club_analytics_daily.flight_count + 1,
            total_airtime_seconds = club_analytics_daily.total_airtime_seconds + flight_duration,
            tow_count = club_analytics_daily.tow_count + CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new are missing required fields
        IF (OLD.takeoff_time IS NULL OR OLD.club_id IS NULL) AND (NEW.takeoff_time IS NULL OR NEW.club_id IS NULL) THEN
            RETURN NEW;
        END IF;

        old_club := OLD.club_id;
        new_club := NEW.club_id;
        old_date := DATE(OLD.takeoff_time);
        affected_date := DATE(NEW.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- Remove old values if club was set
        IF OLD.club_id IS NOT NULL THEN
            UPDATE club_analytics_daily SET
                flight_count = GREATEST(0, flight_count - 1),
                total_airtime_seconds = GREATEST(0, total_airtime_seconds - old_duration),
                tow_count = GREATEST(0, tow_count - CASE WHEN OLD.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END),
                updated_at = NOW()
            WHERE club_id = old_club AND date = old_date;
        END IF;

        -- Add new values if club is set
        IF NEW.club_id IS NOT NULL THEN
            INSERT INTO club_analytics_daily (club_id, date, club_name, flight_count, total_airtime_seconds, tow_count)
            SELECT
                NEW.club_id,
                affected_date,
                c.name,
                1,
                flight_duration,
                CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END
            FROM clubs c
            WHERE c.id = NEW.club_id
            ON CONFLICT (club_id, date) DO UPDATE SET
                flight_count = club_analytics_daily.flight_count + 1,
                total_airtime_seconds = club_analytics_daily.total_airtime_seconds + flight_duration,
                tow_count = club_analytics_daily.tow_count + CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time or club
        IF OLD.takeoff_time IS NULL OR OLD.club_id IS NULL THEN
            RETURN OLD;
        END IF;

        old_club := OLD.club_id;
        old_date := DATE(OLD.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE club_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_airtime_seconds = GREATEST(0, total_airtime_seconds - old_duration),
            tow_count = GREATEST(0, tow_count - CASE WHEN OLD.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END),
            updated_at = NOW()
        WHERE club_id = old_club AND date = old_date;
    END IF;

    RETURN NEW;
END;
$$;


--
-- Name: update_device_analytics(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_device_analytics() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    old_device UUID;
    new_device UUID;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.aircraft_id IS NOT DISTINCT FROM NEW.aircraft_id AND
       OLD.total_distance_meters IS NOT DISTINCT FROM NEW.total_distance_meters
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        new_device := NEW.aircraft_id;
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
        SELECT
            NEW.aircraft_id,
            d.registration,
            d.aircraft_model,
            1,
            NEW.takeoff_time,
            COALESCE(NEW.total_distance_meters, 0)
        FROM devices d
        WHERE d.id = NEW.aircraft_id
        ON CONFLICT (device_id) DO UPDATE SET
            flight_count_total = device_analytics.flight_count_total + 1,
            last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
            total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            avg_flight_duration_seconds = CASE WHEN device_analytics.flight_count_total + 1 > 0
                THEN ((device_analytics.avg_flight_duration_seconds * device_analytics.flight_count_total) + flight_duration) / (device_analytics.flight_count_total + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_device := OLD.aircraft_id;
        new_device := NEW.aircraft_id;
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- If device changed, update both
        IF old_device != new_device THEN
            -- Remove from old device
            UPDATE device_analytics SET
                flight_count_total = GREATEST(0, flight_count_total - 1),
                total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
                updated_at = NOW()
            WHERE device_id = old_device;

            -- Add to new device
            INSERT INTO device_analytics (device_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
            SELECT
                NEW.aircraft_id,
                d.registration,
                d.aircraft_model,
                1,
                NEW.takeoff_time,
                COALESCE(NEW.total_distance_meters, 0)
            FROM devices d
            WHERE d.id = NEW.aircraft_id
            ON CONFLICT (device_id) DO UPDATE SET
                flight_count_total = device_analytics.flight_count_total + 1,
                last_flight_at = GREATEST(device_analytics.last_flight_at, NEW.takeoff_time),
                total_distance_meters = device_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
                avg_flight_duration_seconds = CASE WHEN device_analytics.flight_count_total + 1 > 0
                    THEN ((device_analytics.avg_flight_duration_seconds * device_analytics.flight_count_total) + flight_duration) / (device_analytics.flight_count_total + 1)
                    ELSE 0 END,
                updated_at = NOW();
        ELSE
            -- Same device, just update distance if changed
            IF OLD.total_distance_meters IS DISTINCT FROM NEW.total_distance_meters THEN
                UPDATE device_analytics SET
                    total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0) + COALESCE(NEW.total_distance_meters, 0)),
                    updated_at = NOW()
                WHERE device_id = new_device;
            END IF;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_device := OLD.aircraft_id;

        UPDATE device_analytics SET
            flight_count_total = GREATEST(0, flight_count_total - 1),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            updated_at = NOW()
        WHERE device_id = old_device;
    END IF;

    RETURN NEW;
END;
$$;


--
-- Name: update_flight_analytics_daily(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_flight_analytics_daily() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    affected_date DATE;
    old_date DATE;
    flight_duration INT;
    old_duration INT;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.aircraft_id IS NOT DISTINCT FROM NEW.aircraft_id AND
       OLD.club_id IS NOT DISTINCT FROM NEW.club_id AND
       OLD.departure_airport_id IS NOT DISTINCT FROM NEW.departure_airport_id AND
       OLD.arrival_airport_id IS NOT DISTINCT FROM NEW.arrival_airport_id AND
       OLD.towed_by_aircraft_id IS NOT DISTINCT FROM NEW.towed_by_aircraft_id AND
       OLD.total_distance_meters IS NOT DISTINCT FROM NEW.total_distance_meters
    THEN
        RETURN NEW;  -- Skip analytics update - only last_fix_at or other non-analytics fields changed
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_date := DATE(NEW.takeoff_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, total_distance_meters, tow_flight_count, cross_country_count)
        VALUES (
            affected_date,
            1,
            flight_duration,
            COALESCE(NEW.total_distance_meters, 0),
            CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END
        )
        ON CONFLICT (date) DO UPDATE SET
            flight_count = flight_analytics_daily.flight_count + 1,
            total_duration_seconds = flight_analytics_daily.total_duration_seconds + flight_duration,
            total_distance_meters = flight_analytics_daily.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            tow_flight_count = flight_analytics_daily.tow_flight_count + CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            cross_country_count = flight_analytics_daily.cross_country_count + CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END,
            avg_duration_seconds = CASE WHEN flight_analytics_daily.flight_count + 1 > 0
                THEN (flight_analytics_daily.total_duration_seconds + flight_duration) / (flight_analytics_daily.flight_count + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_date := DATE(OLD.takeoff_time);
        affected_date := DATE(NEW.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);

        -- Remove old values
        UPDATE flight_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_duration_seconds = GREATEST(0, total_duration_seconds - old_duration),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            tow_flight_count = GREATEST(0, tow_flight_count - CASE WHEN OLD.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END),
            cross_country_count = GREATEST(0, cross_country_count - CASE WHEN OLD.departure_airport_id IS DISTINCT FROM OLD.arrival_airport_id THEN 1 ELSE 0 END),
            avg_duration_seconds = CASE WHEN flight_count - 1 > 0
                THEN (total_duration_seconds - old_duration) / (flight_count - 1)
                ELSE 0 END,
            updated_at = NOW()
        WHERE date = old_date;

        -- Add new values
        INSERT INTO flight_analytics_daily (date, flight_count, total_duration_seconds, total_distance_meters, tow_flight_count, cross_country_count)
        VALUES (
            affected_date,
            1,
            flight_duration,
            COALESCE(NEW.total_distance_meters, 0),
            CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END
        )
        ON CONFLICT (date) DO UPDATE SET
            flight_count = flight_analytics_daily.flight_count + 1,
            total_duration_seconds = flight_analytics_daily.total_duration_seconds + flight_duration,
            total_distance_meters = flight_analytics_daily.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            tow_flight_count = flight_analytics_daily.tow_flight_count + CASE WHEN NEW.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END,
            cross_country_count = flight_analytics_daily.cross_country_count + CASE WHEN NEW.departure_airport_id IS DISTINCT FROM NEW.arrival_airport_id THEN 1 ELSE 0 END,
            avg_duration_seconds = CASE WHEN flight_analytics_daily.flight_count + 1 > 0
                THEN (flight_analytics_daily.total_duration_seconds + flight_duration) / (flight_analytics_daily.flight_count + 1)
                ELSE 0 END,
            updated_at = NOW();

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        affected_date := DATE(OLD.takeoff_time);
        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);

        UPDATE flight_analytics_daily SET
            flight_count = GREATEST(0, flight_count - 1),
            total_duration_seconds = GREATEST(0, total_duration_seconds - old_duration),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            tow_flight_count = GREATEST(0, tow_flight_count - CASE WHEN OLD.towed_by_aircraft_id IS NOT NULL THEN 1 ELSE 0 END),
            cross_country_count = GREATEST(0, cross_country_count - CASE WHEN OLD.departure_airport_id IS DISTINCT FROM OLD.arrival_airport_id THEN 1 ELSE 0 END),
            avg_duration_seconds = CASE WHEN flight_count - 1 > 0
                THEN (total_duration_seconds - old_duration) / (flight_count - 1)
                ELSE 0 END,
            updated_at = NOW()
        WHERE date = affected_date;
    END IF;

    RETURN NEW;
END;
$$;


--
-- Name: update_flight_analytics_hourly(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_flight_analytics_hourly() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    affected_hour TIMESTAMPTZ;
    old_hour TIMESTAMPTZ;
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time AND
       OLD.aircraft_id IS NOT DISTINCT FROM NEW.aircraft_id AND
       OLD.club_id IS NOT DISTINCT FROM NEW.club_id
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        affected_hour := DATE_TRUNC('hour', NEW.takeoff_time);

        INSERT INTO flight_analytics_hourly (hour, flight_count, active_devices, active_clubs)
        VALUES (
            affected_hour,
            1,
            1,
            CASE WHEN NEW.club_id IS NOT NULL THEN 1 ELSE 0 END
        )
        ON CONFLICT (hour) DO UPDATE SET
            flight_count = flight_analytics_hourly.flight_count + 1,
            updated_at = NOW();

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_hour := DATE_TRUNC('hour', OLD.takeoff_time);
        affected_hour := DATE_TRUNC('hour', NEW.takeoff_time);

        -- If hour changed, remove from old and add to new
        IF old_hour != affected_hour THEN
            UPDATE flight_analytics_hourly SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE hour = old_hour;

            INSERT INTO flight_analytics_hourly (hour, flight_count, active_devices, active_clubs)
            VALUES (
                affected_hour,
                1,
                1,
                CASE WHEN NEW.club_id IS NOT NULL THEN 1 ELSE 0 END
            )
            ON CONFLICT (hour) DO UPDATE SET
                flight_count = flight_analytics_hourly.flight_count + 1,
                updated_at = NOW();
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_hour := DATE_TRUNC('hour', OLD.takeoff_time);

        UPDATE flight_analytics_hourly SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE hour = old_hour;
    END IF;

    RETURN NEW;
END;
$$;


--
-- Name: update_flight_duration_buckets(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_flight_duration_buckets() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    flight_duration INT;
    old_duration INT;
    bucket VARCHAR(20);
    old_bucket VARCHAR(20);
BEGIN
    -- OPTIMIZATION: Skip if UPDATE only changed non-analytics fields
    IF TG_OP = 'UPDATE' AND
       OLD.takeoff_time IS NOT DISTINCT FROM NEW.takeoff_time AND
       OLD.landing_time IS NOT DISTINCT FROM NEW.landing_time
    THEN
        RETURN NEW;  -- Skip analytics update
    END IF;

    -- Handle INSERT
    IF TG_OP = 'INSERT' THEN
        -- Skip flights without takeoff_time
        IF NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);
        bucket := get_flight_duration_bucket(flight_duration);

        UPDATE flight_duration_buckets SET
            flight_count = flight_count + 1,
            updated_at = NOW()
        WHERE bucket_name = bucket;

    -- Handle UPDATE (only if analytics-relevant fields changed, checked above)
    ELSIF TG_OP = 'UPDATE' THEN
        -- Skip if both old and new takeoff_time are NULL
        IF OLD.takeoff_time IS NULL AND NEW.takeoff_time IS NULL THEN
            RETURN NEW;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        flight_duration := get_flight_duration_seconds(NEW.takeoff_time, NEW.landing_time);
        old_bucket := get_flight_duration_bucket(old_duration);
        bucket := get_flight_duration_bucket(flight_duration);

        -- If bucket changed, remove from old and add to new
        IF old_bucket != bucket THEN
            UPDATE flight_duration_buckets SET
                flight_count = GREATEST(0, flight_count - 1),
                updated_at = NOW()
            WHERE bucket_name = old_bucket;

            UPDATE flight_duration_buckets SET
                flight_count = flight_count + 1,
                updated_at = NOW()
            WHERE bucket_name = bucket;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_duration := get_flight_duration_seconds(OLD.takeoff_time, OLD.landing_time);
        old_bucket := get_flight_duration_bucket(old_duration);

        UPDATE flight_duration_buckets SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE bucket_name = old_bucket;
    END IF;

    RETURN NEW;
END;
$$;


--
-- Name: update_runway_locations(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_runway_locations() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    -- Update low end location
    IF NEW.le_latitude_deg IS NOT NULL AND NEW.le_longitude_deg IS NOT NULL THEN
        NEW.le_location = ST_SetSRID(ST_MakePoint(NEW.le_longitude_deg, NEW.le_latitude_deg), 4326)::geography;
    ELSE
        NEW.le_location = NULL;
    END IF;

    -- Update high end location
    IF NEW.he_latitude_deg IS NOT NULL AND NEW.he_longitude_deg IS NOT NULL THEN
        NEW.he_location = ST_SetSRID(ST_MakePoint(NEW.he_longitude_deg, NEW.he_latitude_deg), 4326)::geography;
    ELSE
        NEW.he_location = NULL;
    END IF;

    RETURN NEW;
END;
$$;


--
-- Name: update_updated_at_column(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_updated_at_column() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


SET default_table_access_method = heap;

--
-- Name: template_public_fixes; Type: TABLE; Schema: partman; Owner: -
--

CREATE TABLE partman.template_public_fixes (
    id uuid NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326),
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    snr_db real,
    bit_errors_corrected integer,
    freq_offset_khz real,
    flight_id uuid,
    device_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    gnss_horizontal_resolution smallint,
    gnss_vertical_resolution smallint,
    aprs_message_id uuid NOT NULL,
    altitude_agl_valid boolean NOT NULL,
    location_geom public.geometry(Point,4326),
    time_gap_seconds integer
);


--
-- Name: template_public_raw_messages; Type: TABLE; Schema: partman; Owner: -
--

CREATE TABLE partman.template_public_raw_messages (
    id uuid NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.__diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: aircraft; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aircraft (
    address integer NOT NULL,
    address_type public.address_type NOT NULL,
    aircraft_model text NOT NULL,
    registration text NOT NULL,
    competition_number text NOT NULL,
    tracked boolean NOT NULL,
    identified boolean NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    from_ddb boolean DEFAULT true NOT NULL,
    frequency_mhz numeric(6,3),
    pilot_name text,
    home_base_airport_ident text,
    aircraft_type_ogn public.aircraft_type_ogn,
    last_fix_at timestamp with time zone,
    club_id uuid,
    icao_model_code text,
    adsb_emitter_category public.adsb_emitter_category,
    tracker_device_type text,
    country_code character(2)
);


--
-- Name: aircraft_analytics; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aircraft_analytics (
    aircraft_id uuid NOT NULL,
    registration character varying,
    aircraft_model character varying,
    flight_count_total integer DEFAULT 0 NOT NULL,
    flight_count_30d integer DEFAULT 0 NOT NULL,
    flight_count_7d integer DEFAULT 0 NOT NULL,
    last_flight_at timestamp with time zone,
    avg_flight_duration_seconds integer DEFAULT 0 NOT NULL,
    total_distance_meters bigint DEFAULT 0 NOT NULL,
    z_score_30d numeric(10,2) DEFAULT 0,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: aircraft_approved_operations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aircraft_approved_operations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    aircraft_registration_id character varying(6) NOT NULL,
    operation character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: aircraft_models; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aircraft_models (
    manufacturer_code text NOT NULL,
    model_code text NOT NULL,
    series_code text NOT NULL,
    manufacturer_name text NOT NULL,
    model_name text NOT NULL,
    aircraft_type text,
    engine_type text,
    aircraft_category text,
    builder_certification text,
    number_of_engines smallint,
    number_of_seats smallint,
    weight_class text,
    cruising_speed smallint,
    type_certificate_data_sheet text,
    type_certificate_data_holder text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: aircraft_other_names; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aircraft_other_names (
    registration_number character varying(7) NOT NULL,
    seq smallint NOT NULL,
    other_name text NOT NULL,
    CONSTRAINT aircraft_other_names_seq_check CHECK (((seq >= 1) AND (seq <= 5)))
);


--
-- Name: aircraft_registrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aircraft_registrations (
    registration_number character varying(6) NOT NULL,
    serial_number character varying(30) NOT NULL,
    year_mfr integer,
    registrant_name character varying(50),
    last_action_date date,
    certificate_issue_date date,
    type_engine_code smallint,
    status_code text,
    transponder_code bigint,
    fractional_owner boolean,
    airworthiness_date date,
    expiration_date date,
    unique_id character(8),
    kit_mfr_name character varying(30),
    kit_model_name character varying(20),
    approved_operations_raw character varying(9),
    home_base_airport_id integer,
    location_id uuid,
    airworthiness_class public.airworthiness_class,
    aircraft_id uuid,
    manufacturer_code character varying(3) NOT NULL,
    model_code character varying(2) NOT NULL,
    series_code character varying(2) NOT NULL,
    engine_manufacturer_code character varying(3),
    engine_model_code character varying(2),
    registrant_type_code public.registrant_type,
    light_sport_type public.light_sport_type,
    aircraft_type public.aircraft_type,
    club_id uuid,
    CONSTRAINT aircraft_registrations_year_mfr_check CHECK (((year_mfr >= 1800) AND (year_mfr <= 2999)))
);


--
-- Name: airport_analytics_daily; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.airport_analytics_daily (
    airport_id integer NOT NULL,
    date date NOT NULL,
    airport_ident character varying,
    airport_name character varying,
    departure_count integer DEFAULT 0 NOT NULL,
    arrival_count integer DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: airports; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.airports (
    id integer NOT NULL,
    ident character varying(16) NOT NULL,
    type character varying(50) NOT NULL,
    name character varying(255) NOT NULL,
    latitude_deg numeric(10,8),
    longitude_deg numeric(11,8),
    location public.geography(Point,4326),
    elevation_ft integer,
    continent character varying(2),
    iso_country character varying(2),
    iso_region character varying(7),
    municipality character varying(255),
    scheduled_service boolean DEFAULT false NOT NULL,
    gps_code character varying(4),
    icao_code character varying(4),
    iata_code character varying(3),
    local_code character varying(7),
    home_link text,
    wikipedia_link text,
    keywords text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: airspace_sync_log; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.airspace_sync_log (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    success boolean,
    airspaces_fetched integer DEFAULT 0,
    airspaces_inserted integer DEFAULT 0,
    airspaces_updated integer DEFAULT 0,
    error_message text,
    countries_filter text[],
    updated_after timestamp with time zone
);


--
-- Name: airspaces; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.airspaces (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    openaip_id integer NOT NULL,
    name text NOT NULL,
    airspace_class public.airspace_class,
    airspace_type public.airspace_type NOT NULL,
    country_code character(2),
    lower_value integer,
    lower_unit text,
    lower_reference public.altitude_reference,
    upper_value integer,
    upper_unit text,
    upper_reference public.altitude_reference,
    geometry public.geography(MultiPolygon,4326) NOT NULL,
    geometry_geom public.geometry(MultiPolygon,4326) GENERATED ALWAYS AS ((geometry)::public.geometry) STORED,
    remarks text,
    activity_type text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    openaip_updated_at timestamp with time zone
);


--
-- Name: club_analytics_daily; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.club_analytics_daily (
    club_id uuid NOT NULL,
    date date NOT NULL,
    club_name character varying,
    flight_count integer DEFAULT 0 NOT NULL,
    active_devices integer DEFAULT 0 NOT NULL,
    total_airtime_seconds bigint DEFAULT 0 NOT NULL,
    tow_count integer DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: clubs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.clubs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name character varying(255) NOT NULL,
    is_soaring boolean DEFAULT false,
    home_base_airport_id integer,
    location_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: countries; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.countries (
    code character(2) NOT NULL,
    name text NOT NULL
);


--
-- Name: data_quality_metrics_daily; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.data_quality_metrics_daily (
    metric_date date NOT NULL,
    total_fixes bigint DEFAULT 0 NOT NULL,
    fixes_with_gaps_60s integer DEFAULT 0 NOT NULL,
    fixes_with_gaps_300s integer DEFAULT 0 NOT NULL,
    unparsed_aprs_messages integer DEFAULT 0 NOT NULL,
    flights_timed_out integer DEFAULT 0 NOT NULL,
    avg_fixes_per_flight numeric(10,2) DEFAULT 0 NOT NULL,
    quality_score numeric(5,2) DEFAULT 100.0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: fixes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
)
PARTITION BY RANGE (received_at);


--
-- Name: fixes_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_default (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_old; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_old (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    snr_db real,
    bit_errors_corrected integer,
    freq_offset_khz real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    gnss_horizontal_resolution smallint,
    gnss_vertical_resolution smallint,
    aprs_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    CONSTRAINT fixes_track_degrees_check CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251211; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251211 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251212; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251212 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251213; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251213 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251214; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251214 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251215; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251215 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251216; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251216 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251217; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251217 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    aprs_type character varying(9) NOT NULL,
    via text[] NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_msl_feet integer,
    flight_number character varying(20),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    flight_id uuid,
    aircraft_id uuid NOT NULL,
    received_at timestamp with time zone NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    altitude_agl_feet integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: flight_analytics_daily; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.flight_analytics_daily (
    date date NOT NULL,
    flight_count integer DEFAULT 0 NOT NULL,
    total_duration_seconds bigint DEFAULT 0 NOT NULL,
    avg_duration_seconds integer DEFAULT 0 NOT NULL,
    total_distance_meters bigint DEFAULT 0 NOT NULL,
    tow_flight_count integer DEFAULT 0 NOT NULL,
    cross_country_count integer DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: flight_analytics_hourly; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.flight_analytics_hourly (
    hour timestamp with time zone NOT NULL,
    flight_count integer DEFAULT 0 NOT NULL,
    active_devices integer DEFAULT 0 NOT NULL,
    active_clubs integer DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: flight_duration_buckets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.flight_duration_buckets (
    bucket_name character varying(20) NOT NULL,
    bucket_order integer NOT NULL,
    min_minutes integer NOT NULL,
    max_minutes integer,
    flight_count integer DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: flight_pilots; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.flight_pilots (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    flight_id uuid NOT NULL,
    pilot_id uuid NOT NULL,
    is_tow_pilot boolean DEFAULT false NOT NULL,
    is_student boolean DEFAULT false NOT NULL,
    is_instructor boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: flights; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.flights (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    device_address character varying(20) NOT NULL,
    takeoff_time timestamp with time zone,
    landing_time timestamp with time zone,
    club_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    device_address_type public.address_type NOT NULL,
    aircraft_id uuid,
    takeoff_altitude_offset_ft integer,
    landing_altitude_offset_ft integer,
    takeoff_runway_ident text,
    landing_runway_ident text,
    total_distance_meters double precision,
    maximum_displacement_meters double precision,
    departure_airport_id integer,
    arrival_airport_id integer,
    towed_by_aircraft_id uuid,
    towed_by_flight_id uuid,
    tow_release_altitude_msl_ft integer,
    tow_release_time timestamp with time zone,
    runways_inferred boolean,
    takeoff_location_id uuid,
    landing_location_id uuid,
    timed_out_at timestamp with time zone,
    last_fix_at timestamp with time zone NOT NULL,
    callsign text,
    tow_release_height_delta_ft integer,
    min_latitude double precision,
    max_latitude double precision,
    min_longitude double precision,
    max_longitude double precision,
    CONSTRAINT check_landing_near_last_fix CHECK (((landing_time IS NULL) OR (last_fix_at >= (landing_time - '00:10:00'::interval)))),
    CONSTRAINT check_last_fix_after_created CHECK ((last_fix_at >= created_at)),
    CONSTRAINT check_takeoff_before_landing CHECK (((takeoff_time IS NULL) OR (landing_time IS NULL) OR (takeoff_time < landing_time))),
    CONSTRAINT check_takeoff_before_tow_release CHECK (((takeoff_time IS NULL) OR (tow_release_time IS NULL) OR (takeoff_time < tow_release_time))),
    CONSTRAINT check_timed_out_or_landed CHECK (((timed_out_at IS NULL) OR (landing_time IS NULL))),
    CONSTRAINT check_timeout_after_last_fix CHECK (((timed_out_at IS NULL) OR (timed_out_at >= last_fix_at))),
    CONSTRAINT check_timeout_reasonable CHECK (((timed_out_at IS NULL) OR (timed_out_at <= (last_fix_at + '24:00:00'::interval)))),
    CONSTRAINT check_tow_release_before_landing CHECK (((tow_release_time IS NULL) OR (landing_time IS NULL) OR (tow_release_time < landing_time)))
);


--
-- Name: locations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.locations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    street1 text,
    street2 text,
    city text,
    state text,
    zip_code text,
    region_code text,
    country_mail_code text,
    geolocation point,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: pilots; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.pilots (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    first_name text NOT NULL,
    last_name text NOT NULL,
    is_licensed boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    club_id uuid,
    is_instructor boolean DEFAULT false NOT NULL,
    is_tow_pilot boolean DEFAULT false NOT NULL,
    is_examiner boolean DEFAULT false NOT NULL,
    deleted_at timestamp with time zone,
    user_id uuid
);


--
-- Name: raw_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
)
PARTITION BY RANGE (received_at);


--
-- Name: raw_messages_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages_default (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
);


--
-- Name: raw_messages_p20251211; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages_p20251211 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
);


--
-- Name: raw_messages_p20251212; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages_p20251212 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
);


--
-- Name: raw_messages_p20251213; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages_p20251213 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
);


--
-- Name: raw_messages_p20251214; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages_p20251214 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
);


--
-- Name: raw_messages_p20251215; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages_p20251215 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
);


--
-- Name: raw_messages_p20251216; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages_p20251216 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
);


--
-- Name: raw_messages_p20251217; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages_p20251217 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'ogn'::public.message_source NOT NULL
);


--
-- Name: receiver_statuses; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.receiver_statuses (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone DEFAULT now() NOT NULL,
    version text,
    platform text,
    cpu_load numeric,
    ram_free numeric,
    ram_total numeric,
    ntp_offset numeric,
    ntp_correction numeric,
    voltage numeric,
    amperage numeric,
    cpu_temperature numeric,
    visible_senders smallint,
    latency numeric,
    senders smallint,
    rf_correction_manual smallint,
    rf_correction_automatic numeric,
    noise numeric,
    senders_signal_quality numeric,
    senders_messages integer,
    good_senders_signal_quality numeric,
    good_senders smallint,
    good_and_bad_senders smallint,
    geoid_offset smallint,
    name text,
    demodulation_snr_db numeric,
    ognr_pilotaware_version text,
    unparsed_data text,
    lag integer,
    receiver_id uuid NOT NULL,
    raw_message_id uuid
);


--
-- Name: receivers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.receivers (
    callsign text NOT NULL,
    description text,
    contact text,
    email text,
    ogn_db_country text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    latest_packet_at timestamp with time zone,
    from_ogn_db boolean DEFAULT false NOT NULL,
    location public.geography(Point,4326),
    latitude double precision,
    longitude double precision,
    street_address text,
    city text,
    region text,
    country text,
    postal_code text,
    geocoded boolean DEFAULT false NOT NULL
);


--
-- Name: receivers_links; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.receivers_links (
    id integer NOT NULL,
    rel text,
    href text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    receiver_id uuid NOT NULL
);


--
-- Name: receivers_links_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.receivers_links_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: receivers_links_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.receivers_links_id_seq OWNED BY public.receivers_links.id;


--
-- Name: receivers_photos; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.receivers_photos (
    id integer NOT NULL,
    photo_url text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    receiver_id uuid NOT NULL
);


--
-- Name: receivers_photos_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.receivers_photos_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: receivers_photos_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.receivers_photos_id_seq OWNED BY public.receivers_photos.id;


--
-- Name: regions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.regions (
    code character(1) NOT NULL,
    description text NOT NULL
);


--
-- Name: runways; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.runways (
    id integer NOT NULL,
    airport_ref integer NOT NULL,
    airport_ident character varying(16) NOT NULL,
    length_ft integer,
    width_ft integer,
    surface text,
    lighted boolean DEFAULT false NOT NULL,
    closed boolean DEFAULT false NOT NULL,
    le_ident text,
    le_latitude_deg numeric(10,8),
    le_longitude_deg numeric(11,8),
    le_location public.geography(Point,4326),
    le_elevation_ft integer,
    le_heading_degt numeric(5,2),
    le_displaced_threshold_ft integer,
    he_ident text,
    he_latitude_deg numeric(10,8),
    he_longitude_deg numeric(11,8),
    he_location public.geography(Point,4326),
    he_elevation_ft integer,
    he_heading_degt numeric(5,2),
    he_displaced_threshold_ft integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: server_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.server_messages (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    software text NOT NULL,
    server_timestamp timestamp with time zone NOT NULL,
    received_at timestamp with time zone NOT NULL,
    server_name text NOT NULL,
    server_endpoint text NOT NULL,
    lag integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: states; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.states (
    code character(2) NOT NULL,
    name text NOT NULL
);


--
-- Name: status_codes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.status_codes (
    code text NOT NULL,
    description text NOT NULL
);


--
-- Name: type_aircraft; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.type_aircraft (
    code character(1) NOT NULL,
    description text NOT NULL
);


--
-- Name: type_engines; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.type_engines (
    code smallint NOT NULL,
    description text NOT NULL
);


--
-- Name: type_registrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.type_registrations (
    code character(1) NOT NULL,
    description text NOT NULL
);


--
-- Name: user_fixes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_fixes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    heading double precision,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    location_geog public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    raw jsonb,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    first_name character varying(255) NOT NULL,
    last_name character varying(255) NOT NULL,
    email character varying(320) NOT NULL,
    password_hash character varying(255) NOT NULL,
    is_admin boolean DEFAULT false NOT NULL,
    club_id uuid,
    email_verified boolean DEFAULT false NOT NULL,
    password_reset_token character varying(255),
    password_reset_expires_at timestamp with time zone,
    email_verification_token character varying(255),
    email_verification_expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    settings jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: fixes_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_default DEFAULT;


--
-- Name: fixes_p20251211; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251211 FOR VALUES FROM ('2025-12-11 00:00:00+00') TO ('2025-12-12 00:00:00+00');


--
-- Name: fixes_p20251212; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251212 FOR VALUES FROM ('2025-12-12 00:00:00+00') TO ('2025-12-13 00:00:00+00');


--
-- Name: fixes_p20251213; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251213 FOR VALUES FROM ('2025-12-13 00:00:00+00') TO ('2025-12-14 00:00:00+00');


--
-- Name: fixes_p20251214; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251214 FOR VALUES FROM ('2025-12-14 00:00:00+00') TO ('2025-12-15 00:00:00+00');


--
-- Name: fixes_p20251215; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251215 FOR VALUES FROM ('2025-12-15 00:00:00+00') TO ('2025-12-16 00:00:00+00');


--
-- Name: fixes_p20251216; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251216 FOR VALUES FROM ('2025-12-16 00:00:00+00') TO ('2025-12-17 00:00:00+00');


--
-- Name: fixes_p20251217; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251217 FOR VALUES FROM ('2025-12-17 00:00:00+00') TO ('2025-12-18 00:00:00+00');


--
-- Name: raw_messages_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages ATTACH PARTITION public.raw_messages_default DEFAULT;


--
-- Name: raw_messages_p20251211; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages ATTACH PARTITION public.raw_messages_p20251211 FOR VALUES FROM ('2025-12-11 00:00:00+00') TO ('2025-12-12 00:00:00+00');


--
-- Name: raw_messages_p20251212; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages ATTACH PARTITION public.raw_messages_p20251212 FOR VALUES FROM ('2025-12-12 00:00:00+00') TO ('2025-12-13 00:00:00+00');


--
-- Name: raw_messages_p20251213; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages ATTACH PARTITION public.raw_messages_p20251213 FOR VALUES FROM ('2025-12-13 00:00:00+00') TO ('2025-12-14 00:00:00+00');


--
-- Name: raw_messages_p20251214; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages ATTACH PARTITION public.raw_messages_p20251214 FOR VALUES FROM ('2025-12-14 00:00:00+00') TO ('2025-12-15 00:00:00+00');


--
-- Name: raw_messages_p20251215; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages ATTACH PARTITION public.raw_messages_p20251215 FOR VALUES FROM ('2025-12-15 00:00:00+00') TO ('2025-12-16 00:00:00+00');


--
-- Name: raw_messages_p20251216; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages ATTACH PARTITION public.raw_messages_p20251216 FOR VALUES FROM ('2025-12-16 00:00:00+00') TO ('2025-12-17 00:00:00+00');


--
-- Name: raw_messages_p20251217; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages ATTACH PARTITION public.raw_messages_p20251217 FOR VALUES FROM ('2025-12-17 00:00:00+00') TO ('2025-12-18 00:00:00+00');


--
-- Name: receivers_links id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers_links ALTER COLUMN id SET DEFAULT nextval('public.receivers_links_id_seq'::regclass);


--
-- Name: receivers_photos id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers_photos ALTER COLUMN id SET DEFAULT nextval('public.receivers_photos_id_seq'::regclass);


--
-- Name: __diesel_schema_migrations __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.__diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: aircraft aircraft_address_type_address_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft
    ADD CONSTRAINT aircraft_address_type_address_unique UNIQUE (address_type, address);


--
-- Name: aircraft_analytics aircraft_analytics_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_analytics
    ADD CONSTRAINT aircraft_analytics_pkey PRIMARY KEY (aircraft_id);


--
-- Name: aircraft_approved_operations aircraft_approved_operations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_approved_operations
    ADD CONSTRAINT aircraft_approved_operations_pkey PRIMARY KEY (id);


--
-- Name: aircraft_models aircraft_model_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_models
    ADD CONSTRAINT aircraft_model_pkey PRIMARY KEY (manufacturer_code, model_code, series_code);


--
-- Name: aircraft_other_names aircraft_other_names_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_other_names
    ADD CONSTRAINT aircraft_other_names_pkey PRIMARY KEY (registration_number, seq);


--
-- Name: aircraft aircraft_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft
    ADD CONSTRAINT aircraft_pkey PRIMARY KEY (id);


--
-- Name: aircraft_registrations aircraft_registrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_pkey PRIMARY KEY (registration_number);


--
-- Name: airport_analytics_daily airport_analytics_daily_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.airport_analytics_daily
    ADD CONSTRAINT airport_analytics_daily_pkey PRIMARY KEY (airport_id, date);


--
-- Name: airports airports_ident_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.airports
    ADD CONSTRAINT airports_ident_key UNIQUE (ident);


--
-- Name: airports airports_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.airports
    ADD CONSTRAINT airports_pkey PRIMARY KEY (id);


--
-- Name: airspace_sync_log airspace_sync_log_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.airspace_sync_log
    ADD CONSTRAINT airspace_sync_log_pkey PRIMARY KEY (id);


--
-- Name: airspaces airspaces_openaip_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.airspaces
    ADD CONSTRAINT airspaces_openaip_id_key UNIQUE (openaip_id);


--
-- Name: airspaces airspaces_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.airspaces
    ADD CONSTRAINT airspaces_pkey PRIMARY KEY (id);


--
-- Name: raw_messages raw_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages
    ADD CONSTRAINT raw_messages_pkey PRIMARY KEY (id, received_at);


--
-- Name: raw_messages_default aprs_messages_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages_default
    ADD CONSTRAINT aprs_messages_default_pkey PRIMARY KEY (id, received_at);


--
-- Name: raw_messages_p20251211 aprs_messages_p20251211_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages_p20251211
    ADD CONSTRAINT aprs_messages_p20251211_pkey PRIMARY KEY (id, received_at);


--
-- Name: raw_messages_p20251212 aprs_messages_p20251212_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages_p20251212
    ADD CONSTRAINT aprs_messages_p20251212_pkey PRIMARY KEY (id, received_at);


--
-- Name: raw_messages_p20251213 aprs_messages_p20251213_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages_p20251213
    ADD CONSTRAINT aprs_messages_p20251213_pkey PRIMARY KEY (id, received_at);


--
-- Name: raw_messages_p20251214 aprs_messages_p20251214_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages_p20251214
    ADD CONSTRAINT aprs_messages_p20251214_pkey PRIMARY KEY (id, received_at);


--
-- Name: raw_messages_p20251215 aprs_messages_p20251215_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages_p20251215
    ADD CONSTRAINT aprs_messages_p20251215_pkey PRIMARY KEY (id, received_at);


--
-- Name: raw_messages_p20251216 aprs_messages_p20251216_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages_p20251216
    ADD CONSTRAINT aprs_messages_p20251216_pkey PRIMARY KEY (id, received_at);


--
-- Name: raw_messages_p20251217 aprs_messages_p20251217_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages_p20251217
    ADD CONSTRAINT aprs_messages_p20251217_pkey PRIMARY KEY (id, received_at);


--
-- Name: club_analytics_daily club_analytics_daily_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.club_analytics_daily
    ADD CONSTRAINT club_analytics_daily_pkey PRIMARY KEY (club_id, date);


--
-- Name: pilots club_pilots_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.pilots
    ADD CONSTRAINT club_pilots_pkey PRIMARY KEY (id);


--
-- Name: clubs clubs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_pkey PRIMARY KEY (id);


--
-- Name: countries countries_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.countries
    ADD CONSTRAINT countries_pkey PRIMARY KEY (code);


--
-- Name: data_quality_metrics_daily data_quality_metrics_daily_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.data_quality_metrics_daily
    ADD CONSTRAINT data_quality_metrics_daily_pkey PRIMARY KEY (metric_date);


--
-- Name: fixes fixes_pkey1; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_pkey1 PRIMARY KEY (id, received_at);


--
-- Name: fixes_default fixes_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_default
    ADD CONSTRAINT fixes_default_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251211 fixes_p20251211_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251211
    ADD CONSTRAINT fixes_p20251211_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251212 fixes_p20251212_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251212
    ADD CONSTRAINT fixes_p20251212_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251213 fixes_p20251213_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251213
    ADD CONSTRAINT fixes_p20251213_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251214 fixes_p20251214_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251214
    ADD CONSTRAINT fixes_p20251214_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251215 fixes_p20251215_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251215
    ADD CONSTRAINT fixes_p20251215_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251216 fixes_p20251216_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251216
    ADD CONSTRAINT fixes_p20251216_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251217 fixes_p20251217_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251217
    ADD CONSTRAINT fixes_p20251217_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_old fixes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_pkey PRIMARY KEY (id);


--
-- Name: flight_analytics_daily flight_analytics_daily_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_analytics_daily
    ADD CONSTRAINT flight_analytics_daily_pkey PRIMARY KEY (date);


--
-- Name: flight_analytics_hourly flight_analytics_hourly_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_analytics_hourly
    ADD CONSTRAINT flight_analytics_hourly_pkey PRIMARY KEY (hour);


--
-- Name: flight_duration_buckets flight_duration_buckets_bucket_order_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_duration_buckets
    ADD CONSTRAINT flight_duration_buckets_bucket_order_key UNIQUE (bucket_order);


--
-- Name: flight_duration_buckets flight_duration_buckets_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_duration_buckets
    ADD CONSTRAINT flight_duration_buckets_pkey PRIMARY KEY (bucket_name);


--
-- Name: flight_pilots flight_pilots_flight_id_pilot_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_pilots
    ADD CONSTRAINT flight_pilots_flight_id_pilot_id_key UNIQUE (flight_id, pilot_id);


--
-- Name: flight_pilots flight_pilots_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_pilots
    ADD CONSTRAINT flight_pilots_pkey PRIMARY KEY (id);


--
-- Name: flights flights_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_pkey PRIMARY KEY (id);


--
-- Name: locations locations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.locations
    ADD CONSTRAINT locations_pkey PRIMARY KEY (id);


--
-- Name: receiver_statuses receiver_statuses_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_statuses
    ADD CONSTRAINT receiver_statuses_pkey PRIMARY KEY (id);


--
-- Name: receivers receivers_callsign_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers
    ADD CONSTRAINT receivers_callsign_key UNIQUE (callsign);


--
-- Name: receivers_links receivers_links_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers_links
    ADD CONSTRAINT receivers_links_pkey PRIMARY KEY (id);


--
-- Name: receivers_photos receivers_photos_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers_photos
    ADD CONSTRAINT receivers_photos_pkey PRIMARY KEY (id);


--
-- Name: receivers receivers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers
    ADD CONSTRAINT receivers_pkey PRIMARY KEY (id);


--
-- Name: regions regions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.regions
    ADD CONSTRAINT regions_pkey PRIMARY KEY (code);


--
-- Name: runways runways_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.runways
    ADD CONSTRAINT runways_pkey PRIMARY KEY (id);


--
-- Name: server_messages server_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.server_messages
    ADD CONSTRAINT server_messages_pkey PRIMARY KEY (id);


--
-- Name: states states_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.states
    ADD CONSTRAINT states_pkey PRIMARY KEY (code);


--
-- Name: status_codes status_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.status_codes
    ADD CONSTRAINT status_codes_pkey PRIMARY KEY (code);


--
-- Name: type_aircraft type_aircraft_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.type_aircraft
    ADD CONSTRAINT type_aircraft_pkey PRIMARY KEY (code);


--
-- Name: type_engines type_engines_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.type_engines
    ADD CONSTRAINT type_engines_pkey PRIMARY KEY (code);


--
-- Name: type_registrations type_registrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.type_registrations
    ADD CONSTRAINT type_registrations_pkey PRIMARY KEY (code);


--
-- Name: user_fixes user_fixes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_fixes
    ADD CONSTRAINT user_fixes_pkey PRIMARY KEY (id);


--
-- Name: users users_email_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: aircraft_registrations_aw_class_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX aircraft_registrations_aw_class_idx ON public.aircraft_registrations USING btree (airworthiness_class);


--
-- Name: aircraft_registrations_home_base_airport_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX aircraft_registrations_home_base_airport_id_idx ON public.aircraft_registrations USING btree (home_base_airport_id);


--
-- Name: aircraft_registrations_location_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX aircraft_registrations_location_id_idx ON public.aircraft_registrations USING btree (location_id);


--
-- Name: aircraft_registrations_serial_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX aircraft_registrations_serial_idx ON public.aircraft_registrations USING btree (serial_number);


--
-- Name: aircraft_registrations_status_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX aircraft_registrations_status_idx ON public.aircraft_registrations USING btree (status_code);


--
-- Name: aircraft_registrations_transponder_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX aircraft_registrations_transponder_idx ON public.aircraft_registrations USING btree (transponder_code);


--
-- Name: airports_iata_trgm_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX airports_iata_trgm_idx ON public.airports USING gin (iata_code public.gin_trgm_ops) WHERE (iata_code IS NOT NULL);


--
-- Name: airports_icao_trgm_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX airports_icao_trgm_idx ON public.airports USING gin (icao_code public.gin_trgm_ops) WHERE (icao_code IS NOT NULL);


--
-- Name: airports_ident_trgm_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX airports_ident_trgm_idx ON public.airports USING gin (ident public.gin_trgm_ops);


--
-- Name: airports_name_trgm_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX airports_name_trgm_idx ON public.airports USING gin (name public.gin_trgm_ops);


--
-- Name: clubs_home_base_airport_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX clubs_home_base_airport_id_idx ON public.clubs USING btree (home_base_airport_id);


--
-- Name: clubs_is_soaring_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX clubs_is_soaring_idx ON public.clubs USING btree (is_soaring);


--
-- Name: clubs_location_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX clubs_location_id_idx ON public.clubs USING btree (location_id);


--
-- Name: clubs_name_trgm_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX clubs_name_trgm_idx ON public.clubs USING gin (name public.gin_trgm_ops);


--
-- Name: idx_fixes_aircraft_received_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_aircraft_received_at ON ONLY public.fixes USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_default_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_default_device_id_received_at_idx ON public.fixes_default USING btree (aircraft_id, received_at DESC);


--
-- Name: idx_fixes_protocol; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_protocol ON ONLY public.fixes USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_default_expr_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_default_expr_idx ON public.fixes_default USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_flight_id_idx ON ONLY public.fixes USING btree (flight_id);


--
-- Name: fixes_default_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_default_flight_id_idx ON public.fixes_default USING btree (flight_id);


--
-- Name: idx_fixes_location_geom; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_location_geom ON ONLY public.fixes USING gist (location_geom);


--
-- Name: fixes_default_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_default_location_geom_idx ON public.fixes_default USING gist (location_geom);


--
-- Name: idx_fixes_location; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_location ON ONLY public.fixes USING gist (location);


--
-- Name: fixes_default_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_default_location_idx ON public.fixes_default USING gist (location);


--
-- Name: idx_fixes_source; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_source ON ONLY public.fixes USING btree (source);


--
-- Name: fixes_default_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_default_source_idx ON public.fixes_default USING btree (source);


--
-- Name: idx_fixes_source_metadata; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_source_metadata ON ONLY public.fixes USING gin (source_metadata);


--
-- Name: fixes_default_source_metadata_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_default_source_metadata_idx ON public.fixes_default USING gin (source_metadata);


--
-- Name: fixes_device_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_device_received_at_idx ON public.fixes_old USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_location_geom_idx ON public.fixes_old USING gist (location_geom);


--
-- Name: fixes_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_location_idx ON public.fixes_old USING gist (location);


--
-- Name: fixes_p20251211_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251211_device_id_received_at_idx ON public.fixes_p20251211 USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_p20251211_expr_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251211_expr_idx ON public.fixes_p20251211 USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_p20251211_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251211_flight_id_idx ON public.fixes_p20251211 USING btree (flight_id);


--
-- Name: fixes_p20251211_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251211_location_geom_idx ON public.fixes_p20251211 USING gist (location_geom);


--
-- Name: fixes_p20251211_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251211_location_idx ON public.fixes_p20251211 USING gist (location);


--
-- Name: fixes_p20251211_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251211_source_idx ON public.fixes_p20251211 USING btree (source);


--
-- Name: fixes_p20251211_source_metadata_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251211_source_metadata_idx ON public.fixes_p20251211 USING gin (source_metadata);


--
-- Name: fixes_p20251212_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251212_device_id_received_at_idx ON public.fixes_p20251212 USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_p20251212_expr_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251212_expr_idx ON public.fixes_p20251212 USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_p20251212_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251212_flight_id_idx ON public.fixes_p20251212 USING btree (flight_id);


--
-- Name: fixes_p20251212_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251212_location_geom_idx ON public.fixes_p20251212 USING gist (location_geom);


--
-- Name: fixes_p20251212_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251212_location_idx ON public.fixes_p20251212 USING gist (location);


--
-- Name: fixes_p20251212_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251212_source_idx ON public.fixes_p20251212 USING btree (source);


--
-- Name: fixes_p20251212_source_metadata_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251212_source_metadata_idx ON public.fixes_p20251212 USING gin (source_metadata);


--
-- Name: fixes_p20251213_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251213_device_id_received_at_idx ON public.fixes_p20251213 USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_p20251213_expr_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251213_expr_idx ON public.fixes_p20251213 USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_p20251213_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251213_flight_id_idx ON public.fixes_p20251213 USING btree (flight_id);


--
-- Name: fixes_p20251213_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251213_location_geom_idx ON public.fixes_p20251213 USING gist (location_geom);


--
-- Name: fixes_p20251213_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251213_location_idx ON public.fixes_p20251213 USING gist (location);


--
-- Name: fixes_p20251213_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251213_source_idx ON public.fixes_p20251213 USING btree (source);


--
-- Name: fixes_p20251213_source_metadata_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251213_source_metadata_idx ON public.fixes_p20251213 USING gin (source_metadata);


--
-- Name: fixes_p20251214_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251214_device_id_received_at_idx ON public.fixes_p20251214 USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_p20251214_expr_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251214_expr_idx ON public.fixes_p20251214 USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_p20251214_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251214_flight_id_idx ON public.fixes_p20251214 USING btree (flight_id);


--
-- Name: fixes_p20251214_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251214_location_geom_idx ON public.fixes_p20251214 USING gist (location_geom);


--
-- Name: fixes_p20251214_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251214_location_idx ON public.fixes_p20251214 USING gist (location);


--
-- Name: fixes_p20251214_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251214_source_idx ON public.fixes_p20251214 USING btree (source);


--
-- Name: fixes_p20251214_source_metadata_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251214_source_metadata_idx ON public.fixes_p20251214 USING gin (source_metadata);


--
-- Name: fixes_p20251215_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251215_device_id_received_at_idx ON public.fixes_p20251215 USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_p20251215_expr_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251215_expr_idx ON public.fixes_p20251215 USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_p20251215_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251215_flight_id_idx ON public.fixes_p20251215 USING btree (flight_id);


--
-- Name: fixes_p20251215_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251215_location_geom_idx ON public.fixes_p20251215 USING gist (location_geom);


--
-- Name: fixes_p20251215_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251215_location_idx ON public.fixes_p20251215 USING gist (location);


--
-- Name: fixes_p20251215_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251215_source_idx ON public.fixes_p20251215 USING btree (source);


--
-- Name: fixes_p20251215_source_metadata_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251215_source_metadata_idx ON public.fixes_p20251215 USING gin (source_metadata);


--
-- Name: fixes_p20251216_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251216_device_id_received_at_idx ON public.fixes_p20251216 USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_p20251216_expr_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251216_expr_idx ON public.fixes_p20251216 USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_p20251216_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251216_flight_id_idx ON public.fixes_p20251216 USING btree (flight_id);


--
-- Name: fixes_p20251216_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251216_location_geom_idx ON public.fixes_p20251216 USING gist (location_geom);


--
-- Name: fixes_p20251216_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251216_location_idx ON public.fixes_p20251216 USING gist (location);


--
-- Name: fixes_p20251216_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251216_source_idx ON public.fixes_p20251216 USING btree (source);


--
-- Name: fixes_p20251216_source_metadata_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251216_source_metadata_idx ON public.fixes_p20251216 USING gin (source_metadata);


--
-- Name: fixes_p20251217_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251217_device_id_received_at_idx ON public.fixes_p20251217 USING btree (aircraft_id, received_at DESC);


--
-- Name: fixes_p20251217_expr_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251217_expr_idx ON public.fixes_p20251217 USING btree (((source_metadata ->> 'protocol'::text))) WHERE (source_metadata IS NOT NULL);


--
-- Name: fixes_p20251217_flight_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251217_flight_id_idx ON public.fixes_p20251217 USING btree (flight_id);


--
-- Name: fixes_p20251217_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251217_location_geom_idx ON public.fixes_p20251217 USING gist (location_geom);


--
-- Name: fixes_p20251217_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251217_location_idx ON public.fixes_p20251217 USING gist (location);


--
-- Name: fixes_p20251217_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251217_source_idx ON public.fixes_p20251217 USING btree (source);


--
-- Name: fixes_p20251217_source_metadata_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251217_source_metadata_idx ON public.fixes_p20251217 USING gin (source_metadata);


--
-- Name: fixes_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_received_at_idx ON public.fixes_old USING btree (received_at);


--
-- Name: fixes_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_source_idx ON public.fixes_old USING btree (source);


--
-- Name: fixes_timestamp_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_timestamp_idx ON public.fixes_old USING btree ("timestamp" DESC);


--
-- Name: flights_aircraft_takeoff_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX flights_aircraft_takeoff_idx ON public.flights USING btree (device_address, takeoff_time DESC);


--
-- Name: flights_club_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX flights_club_id_idx ON public.flights USING btree (club_id);


--
-- Name: flights_landing_time_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX flights_landing_time_idx ON public.flights USING btree (landing_time DESC);


--
-- Name: flights_takeoff_time_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX flights_takeoff_time_idx ON public.flights USING btree (takeoff_time DESC);


--
-- Name: idx_aircraft_aircraft_model; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_aircraft_model ON public.aircraft USING btree (aircraft_model);


--
-- Name: idx_aircraft_analytics_flight_count_30d; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_analytics_flight_count_30d ON public.aircraft_analytics USING btree (flight_count_30d DESC);


--
-- Name: idx_aircraft_analytics_last_flight; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_analytics_last_flight ON public.aircraft_analytics USING btree (last_flight_at DESC);


--
-- Name: idx_aircraft_analytics_z_score; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_analytics_z_score ON public.aircraft_analytics USING btree (z_score_30d DESC);


--
-- Name: idx_aircraft_approved_operations_registration_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_approved_operations_registration_id ON public.aircraft_approved_operations USING btree (aircraft_registration_id);


--
-- Name: idx_aircraft_country_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_country_code ON public.aircraft USING btree (country_code);


--
-- Name: idx_aircraft_from_ddb; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_from_ddb ON public.aircraft USING btree (from_ddb);


--
-- Name: idx_aircraft_icao_model_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_icao_model_code ON public.aircraft USING btree (icao_model_code) WHERE (icao_model_code IS NOT NULL);


--
-- Name: idx_aircraft_identified; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_identified ON public.aircraft USING btree (identified);


--
-- Name: idx_aircraft_last_fix_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_last_fix_at ON public.aircraft USING btree (last_fix_at) WHERE (last_fix_at IS NOT NULL);


--
-- Name: idx_aircraft_model_aircraft_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_model_aircraft_type ON public.aircraft_models USING btree (aircraft_type);


--
-- Name: idx_aircraft_model_engine_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_model_engine_type ON public.aircraft_models USING btree (engine_type);


--
-- Name: idx_aircraft_model_manufacturer_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_model_manufacturer_name ON public.aircraft_models USING btree (manufacturer_name);


--
-- Name: idx_aircraft_model_model_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_model_model_name ON public.aircraft_models USING btree (model_name);


--
-- Name: idx_aircraft_registration; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_registration ON public.aircraft USING btree (registration);


--
-- Name: idx_aircraft_registrations_aircraft_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_registrations_aircraft_id ON public.aircraft_registrations USING btree (aircraft_id);


--
-- Name: idx_aircraft_registrations_club_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_registrations_club_id ON public.aircraft_registrations USING btree (club_id) WHERE (club_id IS NOT NULL);


--
-- Name: idx_aircraft_tracked; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_tracked ON public.aircraft USING btree (tracked);


--
-- Name: idx_aircraft_tracker_device_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_tracker_device_type ON public.aircraft USING btree (tracker_device_type) WHERE (tracker_device_type IS NOT NULL);


--
-- Name: idx_airport_analytics_daily_date_arr; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airport_analytics_daily_date_arr ON public.airport_analytics_daily USING btree (date DESC, arrival_count DESC);


--
-- Name: idx_airport_analytics_daily_date_dep; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airport_analytics_daily_date_dep ON public.airport_analytics_daily USING btree (date DESC, departure_count DESC);


--
-- Name: idx_airports_gps_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_gps_code ON public.airports USING btree (gps_code) WHERE (gps_code IS NOT NULL);


--
-- Name: idx_airports_iata_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_iata_code ON public.airports USING btree (iata_code) WHERE (iata_code IS NOT NULL);


--
-- Name: idx_airports_icao_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_icao_code ON public.airports USING btree (icao_code) WHERE (icao_code IS NOT NULL);


--
-- Name: idx_airports_iso_country; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_iso_country ON public.airports USING btree (iso_country);


--
-- Name: idx_airports_iso_region; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_iso_region ON public.airports USING btree (iso_region);


--
-- Name: idx_airports_location_gist; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_location_gist ON public.airports USING gist (location) WHERE (location IS NOT NULL);


--
-- Name: idx_airports_municipality; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_municipality ON public.airports USING btree (municipality);


--
-- Name: idx_airports_scheduled_service; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_scheduled_service ON public.airports USING btree (scheduled_service) WHERE (scheduled_service = true);


--
-- Name: idx_airports_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_type ON public.airports USING btree (type);


--
-- Name: idx_airspace_sync_log_completed; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airspace_sync_log_completed ON public.airspace_sync_log USING btree (completed_at DESC);


--
-- Name: idx_airspace_sync_log_success; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airspace_sync_log_success ON public.airspace_sync_log USING btree (success) WHERE (success = true);


--
-- Name: idx_airspaces_class; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airspaces_class ON public.airspaces USING btree (airspace_class);


--
-- Name: idx_airspaces_country; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airspaces_country ON public.airspaces USING btree (country_code);


--
-- Name: idx_airspaces_geometry; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airspaces_geometry ON public.airspaces USING gist (geometry);


--
-- Name: idx_airspaces_geometry_geom; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airspaces_geometry_geom ON public.airspaces USING gist (geometry_geom);


--
-- Name: idx_airspaces_openaip_updated; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airspaces_openaip_updated ON public.airspaces USING btree (openaip_updated_at);


--
-- Name: idx_airspaces_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airspaces_type ON public.airspaces USING btree (airspace_type);


--
-- Name: idx_club_analytics_daily_club_date; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_club_analytics_daily_club_date ON public.club_analytics_daily USING btree (club_id, date DESC);


--
-- Name: idx_club_analytics_daily_date; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_club_analytics_daily_date ON public.club_analytics_daily USING btree (date DESC);


--
-- Name: idx_club_pilots_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_club_pilots_name ON public.pilots USING btree (last_name, first_name);


--
-- Name: idx_data_quality_metrics_daily_date_desc; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_data_quality_metrics_daily_date_desc ON public.data_quality_metrics_daily USING btree (metric_date DESC);


--
-- Name: idx_fixes_altitude_agl_feet; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_altitude_agl_feet ON public.fixes_old USING btree (altitude_agl_feet);


--
-- Name: idx_fixes_altitude_agl_valid; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_altitude_agl_valid ON public.fixes_old USING btree (altitude_agl_valid) WHERE (altitude_agl_valid = false);


--
-- Name: idx_fixes_backfill_optimized; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_backfill_optimized ON public.fixes_old USING btree ("timestamp") WHERE ((altitude_agl_valid = false) AND (altitude_msl_feet IS NOT NULL) AND (is_active = true));


--
-- Name: idx_fixes_device_id_timestamp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_device_id_timestamp ON public.fixes_old USING btree (aircraft_id, "timestamp");


--
-- Name: idx_fixes_flight_id_timestamp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_flight_id_timestamp ON public.fixes_old USING btree (flight_id, "timestamp");


--
-- Name: idx_fixes_ground_speed_knots; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_ground_speed_knots ON public.fixes_old USING btree (ground_speed_knots);


--
-- Name: idx_fixes_is_active; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_is_active ON public.fixes_old USING btree (is_active);


--
-- Name: idx_fixes_raw_message_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_raw_message_id ON public.fixes_old USING btree (aprs_message_id);


--
-- Name: idx_fixes_receiver_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_receiver_id ON public.fixes_old USING btree (receiver_id);


--
-- Name: idx_fixes_time_gap_seconds; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_time_gap_seconds ON public.fixes_old USING btree (time_gap_seconds) WHERE (time_gap_seconds IS NOT NULL);


--
-- Name: idx_fixes_timestamp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_timestamp ON public.fixes_old USING btree ("timestamp");


--
-- Name: idx_flight_analytics_daily_date_desc; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flight_analytics_daily_date_desc ON public.flight_analytics_daily USING btree (date DESC);


--
-- Name: idx_flight_analytics_hourly_hour_desc; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flight_analytics_hourly_hour_desc ON public.flight_analytics_hourly USING btree (hour DESC);


--
-- Name: idx_flight_pilots_pilot_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flight_pilots_pilot_id ON public.flight_pilots USING btree (pilot_id);


--
-- Name: idx_flights_aircraft_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_aircraft_id ON public.flights USING btree (aircraft_id);


--
-- Name: idx_flights_bounding_box; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_bounding_box ON public.flights USING btree (min_latitude, max_latitude, min_longitude, max_longitude) WHERE (min_latitude IS NOT NULL);


--
-- Name: idx_flights_callsign; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_callsign ON public.flights USING btree (callsign);


--
-- Name: idx_flights_landing_location_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_landing_location_id ON public.flights USING btree (landing_location_id);


--
-- Name: idx_flights_last_fix_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_last_fix_at ON public.flights USING btree (last_fix_at);


--
-- Name: idx_flights_takeoff_location_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_takeoff_location_id ON public.flights USING btree (takeoff_location_id);


--
-- Name: idx_flights_towed_by_aircraft; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_towed_by_aircraft ON public.flights USING btree (towed_by_aircraft_id) WHERE (towed_by_aircraft_id IS NOT NULL);


--
-- Name: idx_flights_towed_by_flight; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_towed_by_flight ON public.flights USING btree (towed_by_flight_id) WHERE (towed_by_flight_id IS NOT NULL);


--
-- Name: idx_pilots_club_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_pilots_club_id ON public.pilots USING btree (club_id);


--
-- Name: idx_pilots_deleted_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_pilots_deleted_at ON public.pilots USING btree (deleted_at) WHERE (deleted_at IS NULL);


--
-- Name: idx_receiver_statuses_raw_message_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receiver_statuses_raw_message_id ON public.receiver_statuses USING btree (raw_message_id);


--
-- Name: idx_receiver_statuses_received_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receiver_statuses_received_at ON public.receiver_statuses USING btree (received_at);


--
-- Name: idx_receiver_statuses_receiver_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receiver_statuses_receiver_id ON public.receiver_statuses USING btree (receiver_id);


--
-- Name: idx_receivers_city; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receivers_city ON public.receivers USING btree (city);


--
-- Name: idx_receivers_country; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receivers_country ON public.receivers USING btree (country);


--
-- Name: idx_receivers_geocoded; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receivers_geocoded ON public.receivers USING btree (geocoded) WHERE (geocoded = false);


--
-- Name: idx_receivers_lat_lng; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receivers_lat_lng ON public.receivers USING btree (latitude, longitude) WHERE ((latitude IS NOT NULL) AND (longitude IS NOT NULL));


--
-- Name: idx_receivers_location; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receivers_location ON public.receivers USING gist (location);


--
-- Name: idx_receivers_ogn_db_country; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receivers_ogn_db_country ON public.receivers USING btree (ogn_db_country);


--
-- Name: idx_receivers_region; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receivers_region ON public.receivers USING btree (region);


--
-- Name: idx_runways_airport_ident; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_airport_ident ON public.runways USING btree (airport_ident);


--
-- Name: idx_runways_airport_ref; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_airport_ref ON public.runways USING btree (airport_ref);


--
-- Name: idx_runways_closed; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_closed ON public.runways USING btree (closed);


--
-- Name: idx_runways_he_location_gist; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_he_location_gist ON public.runways USING gist (he_location) WHERE (he_location IS NOT NULL);


--
-- Name: idx_runways_le_location_gist; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_le_location_gist ON public.runways USING gist (le_location) WHERE (le_location IS NOT NULL);


--
-- Name: idx_runways_length; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_length ON public.runways USING btree (length_ft) WHERE (length_ft IS NOT NULL);


--
-- Name: idx_runways_lighted; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_lighted ON public.runways USING btree (lighted) WHERE (lighted = true);


--
-- Name: idx_runways_surface; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_surface ON public.runways USING btree (surface);


--
-- Name: idx_server_messages_received_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_server_messages_received_at ON public.server_messages USING btree (received_at);


--
-- Name: idx_server_messages_server_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_server_messages_server_name ON public.server_messages USING btree (server_name);


--
-- Name: idx_server_messages_server_timestamp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_server_messages_server_timestamp ON public.server_messages USING btree (server_timestamp);


--
-- Name: idx_user_fixes_location_geog; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_fixes_location_geog ON public.user_fixes USING gist (location_geog);


--
-- Name: idx_user_fixes_location_geom; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_fixes_location_geom ON public.user_fixes USING gist (location_geom);


--
-- Name: idx_user_fixes_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_fixes_user_id ON public.user_fixes USING btree (user_id);


--
-- Name: idx_user_fixes_user_timestamp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_fixes_user_timestamp ON public.user_fixes USING btree (user_id, "timestamp" DESC);


--
-- Name: locations_address_unique_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX locations_address_unique_idx ON public.locations USING btree (street1, street2, city, state, zip_code, country_mail_code);


--
-- Name: locations_geolocation_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX locations_geolocation_idx ON public.locations USING gist (geolocation);


--
-- Name: users_club_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_club_id_idx ON public.users USING btree (club_id);


--
-- Name: users_email_verification_token_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_email_verification_token_idx ON public.users USING btree (email_verification_token);


--
-- Name: users_password_reset_token_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_password_reset_token_idx ON public.users USING btree (password_reset_token);


--
-- Name: aprs_messages_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.raw_messages_pkey ATTACH PARTITION public.aprs_messages_default_pkey;


--
-- Name: aprs_messages_p20251211_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.raw_messages_pkey ATTACH PARTITION public.aprs_messages_p20251211_pkey;


--
-- Name: aprs_messages_p20251212_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.raw_messages_pkey ATTACH PARTITION public.aprs_messages_p20251212_pkey;


--
-- Name: aprs_messages_p20251213_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.raw_messages_pkey ATTACH PARTITION public.aprs_messages_p20251213_pkey;


--
-- Name: aprs_messages_p20251214_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.raw_messages_pkey ATTACH PARTITION public.aprs_messages_p20251214_pkey;


--
-- Name: aprs_messages_p20251215_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.raw_messages_pkey ATTACH PARTITION public.aprs_messages_p20251215_pkey;


--
-- Name: aprs_messages_p20251216_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.raw_messages_pkey ATTACH PARTITION public.aprs_messages_p20251216_pkey;


--
-- Name: aprs_messages_p20251217_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.raw_messages_pkey ATTACH PARTITION public.aprs_messages_p20251217_pkey;


--
-- Name: fixes_default_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_aircraft_received_at ATTACH PARTITION public.fixes_default_device_id_received_at_idx;


--
-- Name: fixes_default_expr_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_protocol ATTACH PARTITION public.fixes_default_expr_idx;


--
-- Name: fixes_default_flight_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_flight_id_idx ATTACH PARTITION public.fixes_default_flight_id_idx;


--
-- Name: fixes_default_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_default_location_geom_idx;


--
-- Name: fixes_default_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_default_location_idx;


--
-- Name: fixes_default_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_default_pkey;


--
-- Name: fixes_default_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_default_source_idx;


--
-- Name: fixes_default_source_metadata_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source_metadata ATTACH PARTITION public.fixes_default_source_metadata_idx;


--
-- Name: fixes_p20251211_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_aircraft_received_at ATTACH PARTITION public.fixes_p20251211_device_id_received_at_idx;


--
-- Name: fixes_p20251211_expr_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_protocol ATTACH PARTITION public.fixes_p20251211_expr_idx;


--
-- Name: fixes_p20251211_flight_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_flight_id_idx ATTACH PARTITION public.fixes_p20251211_flight_id_idx;


--
-- Name: fixes_p20251211_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251211_location_geom_idx;


--
-- Name: fixes_p20251211_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251211_location_idx;


--
-- Name: fixes_p20251211_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251211_pkey;


--
-- Name: fixes_p20251211_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251211_source_idx;


--
-- Name: fixes_p20251211_source_metadata_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source_metadata ATTACH PARTITION public.fixes_p20251211_source_metadata_idx;


--
-- Name: fixes_p20251212_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_aircraft_received_at ATTACH PARTITION public.fixes_p20251212_device_id_received_at_idx;


--
-- Name: fixes_p20251212_expr_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_protocol ATTACH PARTITION public.fixes_p20251212_expr_idx;


--
-- Name: fixes_p20251212_flight_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_flight_id_idx ATTACH PARTITION public.fixes_p20251212_flight_id_idx;


--
-- Name: fixes_p20251212_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251212_location_geom_idx;


--
-- Name: fixes_p20251212_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251212_location_idx;


--
-- Name: fixes_p20251212_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251212_pkey;


--
-- Name: fixes_p20251212_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251212_source_idx;


--
-- Name: fixes_p20251212_source_metadata_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source_metadata ATTACH PARTITION public.fixes_p20251212_source_metadata_idx;


--
-- Name: fixes_p20251213_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_aircraft_received_at ATTACH PARTITION public.fixes_p20251213_device_id_received_at_idx;


--
-- Name: fixes_p20251213_expr_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_protocol ATTACH PARTITION public.fixes_p20251213_expr_idx;


--
-- Name: fixes_p20251213_flight_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_flight_id_idx ATTACH PARTITION public.fixes_p20251213_flight_id_idx;


--
-- Name: fixes_p20251213_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251213_location_geom_idx;


--
-- Name: fixes_p20251213_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251213_location_idx;


--
-- Name: fixes_p20251213_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251213_pkey;


--
-- Name: fixes_p20251213_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251213_source_idx;


--
-- Name: fixes_p20251213_source_metadata_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source_metadata ATTACH PARTITION public.fixes_p20251213_source_metadata_idx;


--
-- Name: fixes_p20251214_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_aircraft_received_at ATTACH PARTITION public.fixes_p20251214_device_id_received_at_idx;


--
-- Name: fixes_p20251214_expr_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_protocol ATTACH PARTITION public.fixes_p20251214_expr_idx;


--
-- Name: fixes_p20251214_flight_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_flight_id_idx ATTACH PARTITION public.fixes_p20251214_flight_id_idx;


--
-- Name: fixes_p20251214_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251214_location_geom_idx;


--
-- Name: fixes_p20251214_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251214_location_idx;


--
-- Name: fixes_p20251214_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251214_pkey;


--
-- Name: fixes_p20251214_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251214_source_idx;


--
-- Name: fixes_p20251214_source_metadata_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source_metadata ATTACH PARTITION public.fixes_p20251214_source_metadata_idx;


--
-- Name: fixes_p20251215_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_aircraft_received_at ATTACH PARTITION public.fixes_p20251215_device_id_received_at_idx;


--
-- Name: fixes_p20251215_expr_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_protocol ATTACH PARTITION public.fixes_p20251215_expr_idx;


--
-- Name: fixes_p20251215_flight_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_flight_id_idx ATTACH PARTITION public.fixes_p20251215_flight_id_idx;


--
-- Name: fixes_p20251215_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251215_location_geom_idx;


--
-- Name: fixes_p20251215_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251215_location_idx;


--
-- Name: fixes_p20251215_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251215_pkey;


--
-- Name: fixes_p20251215_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251215_source_idx;


--
-- Name: fixes_p20251215_source_metadata_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source_metadata ATTACH PARTITION public.fixes_p20251215_source_metadata_idx;


--
-- Name: fixes_p20251216_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_aircraft_received_at ATTACH PARTITION public.fixes_p20251216_device_id_received_at_idx;


--
-- Name: fixes_p20251216_expr_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_protocol ATTACH PARTITION public.fixes_p20251216_expr_idx;


--
-- Name: fixes_p20251216_flight_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_flight_id_idx ATTACH PARTITION public.fixes_p20251216_flight_id_idx;


--
-- Name: fixes_p20251216_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251216_location_geom_idx;


--
-- Name: fixes_p20251216_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251216_location_idx;


--
-- Name: fixes_p20251216_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251216_pkey;


--
-- Name: fixes_p20251216_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251216_source_idx;


--
-- Name: fixes_p20251216_source_metadata_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source_metadata ATTACH PARTITION public.fixes_p20251216_source_metadata_idx;


--
-- Name: fixes_p20251217_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_aircraft_received_at ATTACH PARTITION public.fixes_p20251217_device_id_received_at_idx;


--
-- Name: fixes_p20251217_expr_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_protocol ATTACH PARTITION public.fixes_p20251217_expr_idx;


--
-- Name: fixes_p20251217_flight_id_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_flight_id_idx ATTACH PARTITION public.fixes_p20251217_flight_id_idx;


--
-- Name: fixes_p20251217_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251217_location_geom_idx;


--
-- Name: fixes_p20251217_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251217_location_idx;


--
-- Name: fixes_p20251217_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251217_pkey;


--
-- Name: fixes_p20251217_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251217_source_idx;


--
-- Name: fixes_p20251217_source_metadata_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source_metadata ATTACH PARTITION public.fixes_p20251217_source_metadata_idx;


--
-- Name: pilots set_club_pilots_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_club_pilots_updated_at BEFORE UPDATE ON public.pilots FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: flights trigger_airport_analytics_daily; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_airport_analytics_daily AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION public.update_airport_analytics_daily();


--
-- Name: flights trigger_club_analytics_daily; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_club_analytics_daily AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION public.update_club_analytics_daily();


--
-- Name: flights trigger_device_analytics; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_device_analytics AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION public.update_device_analytics();


--
-- Name: flights trigger_flight_analytics_daily; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_flight_analytics_daily AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION public.update_flight_analytics_daily();


--
-- Name: flights trigger_flight_analytics_hourly; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_flight_analytics_hourly AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION public.update_flight_analytics_hourly();


--
-- Name: flights trigger_flight_duration_buckets; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER trigger_flight_duration_buckets AFTER INSERT OR DELETE OR UPDATE ON public.flights FOR EACH ROW EXECUTE FUNCTION public.update_flight_duration_buckets();


--
-- Name: aircraft_models update_aircraft_model_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_aircraft_model_updated_at BEFORE UPDATE ON public.aircraft_models FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: aircraft update_aircraft_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_aircraft_updated_at BEFORE UPDATE ON public.aircraft FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: airports update_airport_location_trigger; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_airport_location_trigger BEFORE INSERT OR UPDATE OF latitude_deg, longitude_deg ON public.airports FOR EACH ROW EXECUTE FUNCTION public.update_airport_location();


--
-- Name: airports update_airports_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_airports_updated_at BEFORE UPDATE ON public.airports FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: airspaces update_airspaces_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_airspaces_updated_at BEFORE UPDATE ON public.airspaces FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: receivers update_receivers_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_receivers_updated_at BEFORE UPDATE ON public.receivers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: runways update_runway_locations_trigger; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_runway_locations_trigger BEFORE INSERT OR UPDATE OF le_latitude_deg, le_longitude_deg, he_latitude_deg, he_longitude_deg ON public.runways FOR EACH ROW EXECUTE FUNCTION public.update_runway_locations();


--
-- Name: runways update_runways_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_runways_updated_at BEFORE UPDATE ON public.runways FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: users update_users_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: aircraft aircraft_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft
    ADD CONSTRAINT aircraft_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);


--
-- Name: aircraft_other_names aircraft_other_names_registration_number_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_other_names
    ADD CONSTRAINT aircraft_other_names_registration_number_fkey FOREIGN KEY (registration_number) REFERENCES public.aircraft_registrations(registration_number) ON DELETE CASCADE;


--
-- Name: aircraft_registrations aircraft_registrations_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_aircraft_id_fkey FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) ON DELETE SET NULL;


--
-- Name: aircraft_registrations aircraft_registrations_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id) ON DELETE SET NULL;


--
-- Name: aircraft_registrations aircraft_registrations_home_base_airport_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_home_base_airport_id_fkey FOREIGN KEY (home_base_airport_id) REFERENCES public.airports(id) ON DELETE SET NULL;


--
-- Name: aircraft_registrations aircraft_registrations_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id);


--
-- Name: aircraft_registrations aircraft_registrations_status_code_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_status_code_fkey FOREIGN KEY (status_code) REFERENCES public.status_codes(code);


--
-- Name: aircraft_registrations aircraft_registrations_type_engine_code_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_type_engine_code_fkey FOREIGN KEY (type_engine_code) REFERENCES public.type_engines(code);


--
-- Name: clubs clubs_home_base_airport_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_home_base_airport_id_fkey FOREIGN KEY (home_base_airport_id) REFERENCES public.airports(id) ON DELETE SET NULL;


--
-- Name: clubs clubs_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id);


--
-- Name: fixes fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.fixes
    ADD CONSTRAINT fixes_aircraft_id_fkey FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) ON DELETE SET NULL;


--
-- Name: fixes_old fixes_device_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_device_id_fkey FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) ON DELETE SET NULL;


--
-- Name: fixes fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.fixes
    ADD CONSTRAINT fixes_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: fixes_old fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE SET NULL;


--
-- Name: fixes fixes_raw_message_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.fixes
    ADD CONSTRAINT fixes_raw_message_id_fkey FOREIGN KEY (raw_message_id, received_at) REFERENCES public.raw_messages(id, received_at);


--
-- Name: fixes fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.fixes
    ADD CONSTRAINT fixes_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: fixes_old fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: aircraft_approved_operations fk_aircraft_registration; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_approved_operations
    ADD CONSTRAINT fk_aircraft_registration FOREIGN KEY (aircraft_registration_id) REFERENCES public.aircraft_registrations(registration_number) ON DELETE CASCADE;


--
-- Name: runways fk_runway_airport_ident; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.runways
    ADD CONSTRAINT fk_runway_airport_ident FOREIGN KEY (airport_ident) REFERENCES public.airports(ident);


--
-- Name: runways fk_runway_airport_ref; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.runways
    ADD CONSTRAINT fk_runway_airport_ref FOREIGN KEY (airport_ref) REFERENCES public.airports(id);


--
-- Name: flight_pilots flight_pilots_flight_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_pilots
    ADD CONSTRAINT flight_pilots_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE CASCADE;


--
-- Name: flight_pilots flight_pilots_pilot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_pilots
    ADD CONSTRAINT flight_pilots_pilot_id_fkey FOREIGN KEY (pilot_id) REFERENCES public.pilots(id) ON DELETE CASCADE;


--
-- Name: flights flights_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_aircraft_id_fkey FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) ON DELETE SET NULL;


--
-- Name: flights flights_arrival_airport_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_arrival_airport_id_fkey FOREIGN KEY (arrival_airport_id) REFERENCES public.airports(id);


--
-- Name: flights flights_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id) ON DELETE SET NULL;


--
-- Name: flights flights_departure_airport_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_departure_airport_id_fkey FOREIGN KEY (departure_airport_id) REFERENCES public.airports(id);


--
-- Name: flights flights_landing_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_landing_location_id_fkey FOREIGN KEY (landing_location_id) REFERENCES public.locations(id);


--
-- Name: flights flights_takeoff_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_takeoff_location_id_fkey FOREIGN KEY (takeoff_location_id) REFERENCES public.locations(id);


--
-- Name: flights flights_towed_by_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_towed_by_aircraft_id_fkey FOREIGN KEY (towed_by_aircraft_id) REFERENCES public.aircraft(id);


--
-- Name: flights flights_towed_by_flight_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_towed_by_flight_id_fkey FOREIGN KEY (towed_by_flight_id) REFERENCES public.flights(id);


--
-- Name: pilots pilots_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.pilots
    ADD CONSTRAINT pilots_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);


--
-- Name: pilots pilots_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.pilots
    ADD CONSTRAINT pilots_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: raw_messages raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.raw_messages
    ADD CONSTRAINT raw_messages_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: receiver_statuses receiver_statuses_raw_message_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_statuses
    ADD CONSTRAINT receiver_statuses_raw_message_id_fkey FOREIGN KEY (raw_message_id, received_at) REFERENCES public.raw_messages(id, received_at);


--
-- Name: receiver_statuses receiver_statuses_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_statuses
    ADD CONSTRAINT receiver_statuses_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: receivers_links receivers_links_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers_links
    ADD CONSTRAINT receivers_links_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: receivers_photos receivers_photos_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers_photos
    ADD CONSTRAINT receivers_photos_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: user_fixes user_fixes_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_fixes
    ADD CONSTRAINT user_fixes_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: users users_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id) ON DELETE SET NULL;


--
-- PostgreSQL database dump complete
--

\unrestrict SOAR
