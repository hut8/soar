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
-- Name: timescaledb; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS timescaledb WITH SCHEMA public;


--
-- Name: h3; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS h3 WITH SCHEMA public;


--
-- Name: postgis; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS postgis WITH SCHEMA public;


--
-- Name: postgis_raster; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS postgis_raster WITH SCHEMA public;


--
-- Name: h3_postgis; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS h3_postgis WITH SCHEMA public;


--
-- Name: pg_stat_statements; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_stat_statements WITH SCHEMA public;


--
-- Name: pg_trgm; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public;


--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


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
-- Name: aircraft_category; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.aircraft_category AS ENUM (
    'landplane',
    'helicopter',
    'balloon',
    'amphibian',
    'gyroplane',
    'drone',
    'powered_parachute',
    'rotorcraft',
    'seaplane',
    'tiltrotor',
    'vtol',
    'electric',
    'unknown',
    'glider',
    'tow_tug',
    'paraglider',
    'hang_glider',
    'airship',
    'skydiver_parachute',
    'static_obstacle'
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
-- Name: engine_type; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.engine_type AS ENUM (
    'piston',
    'jet',
    'turbine',
    'electric',
    'rocket',
    'special',
    'none',
    'unknown'
);


--
-- Name: icao_aircraft_category; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.icao_aircraft_category AS ENUM (
    'airplane',
    'helicopter'
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
    'aprs',
    'beast',
    'sbs'
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
-- Name: timeout_phase; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.timeout_phase AS ENUM (
    'climbing',
    'cruising',
    'descending',
    'unknown'
);


--
-- Name: wing_type; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.wing_type AS ENUM (
    'fixed_wing',
    'rotary_wing'
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

        INSERT INTO aircraft_analytics (aircraft_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
        SELECT
            NEW.aircraft_id,
            a.registration,
            a.aircraft_model,
            1,
            NEW.takeoff_time,
            COALESCE(NEW.total_distance_meters, 0)
        FROM aircraft a
        WHERE a.id = NEW.aircraft_id
        ON CONFLICT (aircraft_id) DO UPDATE SET
            flight_count_total = aircraft_analytics.flight_count_total + 1,
            last_flight_at = GREATEST(aircraft_analytics.last_flight_at, NEW.takeoff_time),
            total_distance_meters = aircraft_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
            avg_flight_duration_seconds = CASE WHEN aircraft_analytics.flight_count_total + 1 > 0
                THEN ((aircraft_analytics.avg_flight_duration_seconds * aircraft_analytics.flight_count_total) + flight_duration) / (aircraft_analytics.flight_count_total + 1)
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
            UPDATE aircraft_analytics SET
                flight_count_total = GREATEST(0, flight_count_total - 1),
                total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
                updated_at = NOW()
            WHERE aircraft_id = old_device;

            -- Add to new device
            INSERT INTO aircraft_analytics (aircraft_id, registration, aircraft_model, flight_count_total, last_flight_at, total_distance_meters)
            SELECT
                NEW.aircraft_id,
                a.registration,
                a.aircraft_model,
                1,
                NEW.takeoff_time,
                COALESCE(NEW.total_distance_meters, 0)
            FROM aircraft a
            WHERE a.id = NEW.aircraft_id
            ON CONFLICT (aircraft_id) DO UPDATE SET
                flight_count_total = aircraft_analytics.flight_count_total + 1,
                last_flight_at = GREATEST(aircraft_analytics.last_flight_at, NEW.takeoff_time),
                total_distance_meters = aircraft_analytics.total_distance_meters + COALESCE(NEW.total_distance_meters, 0),
                avg_flight_duration_seconds = CASE WHEN aircraft_analytics.flight_count_total + 1 > 0
                    THEN ((aircraft_analytics.avg_flight_duration_seconds * aircraft_analytics.flight_count_total) + flight_duration) / (aircraft_analytics.flight_count_total + 1)
                    ELSE 0 END,
                updated_at = NOW();
        ELSE
            -- Same device, just update distance if changed
            IF OLD.total_distance_meters IS DISTINCT FROM NEW.total_distance_meters THEN
                UPDATE aircraft_analytics SET
                    total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0) + COALESCE(NEW.total_distance_meters, 0)),
                    updated_at = NOW()
                WHERE aircraft_id = new_device;
            END IF;
        END IF;

    -- Handle DELETE
    ELSIF TG_OP = 'DELETE' THEN
        -- Skip flights without takeoff_time
        IF OLD.takeoff_time IS NULL THEN
            RETURN OLD;
        END IF;

        old_device := OLD.aircraft_id;

        UPDATE aircraft_analytics SET
            flight_count_total = GREATEST(0, flight_count_total - 1),
            total_distance_meters = GREATEST(0, total_distance_meters - COALESCE(OLD.total_distance_meters, 0)),
            updated_at = NOW()
        WHERE aircraft_id = old_device;
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
        bucket := get_duration_bucket(flight_duration);

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
        old_bucket := get_duration_bucket(old_duration);
        bucket := get_duration_bucket(flight_duration);

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
        old_bucket := get_duration_bucket(old_duration);

        UPDATE flight_duration_buckets SET
            flight_count = GREATEST(0, flight_count - 1),
            updated_at = NOW()
        WHERE bucket_name = old_bucket;
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
-- Name: _compressed_hypertable_3; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._compressed_hypertable_3 (
);


--
-- Name: _compressed_hypertable_4; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._compressed_hypertable_4 (
);


--
-- Name: raw_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.raw_messages (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid,
    unparsed text,
    raw_message_hash bytea NOT NULL,
    raw_message bytea NOT NULL,
    source public.message_source DEFAULT 'aprs'::public.message_source NOT NULL
);


--
-- Name: _hyper_1_111_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_111_chunk (
    CONSTRAINT constraint_35 CHECK (((received_at >= '2025-12-29 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-30 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_135_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_135_chunk (
    CONSTRAINT constraint_37 CHECK (((received_at >= '2025-12-30 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-31 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_15_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_15_chunk (
    CONSTRAINT constraint_15 CHECK (((received_at >= '2025-12-26 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-27 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_160_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_160_chunk (
    CONSTRAINT constraint_39 CHECK (((received_at >= '2025-12-31 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-01 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_16_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_16_chunk (
    CONSTRAINT constraint_16 CHECK (((received_at >= '2025-12-27 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-28 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_187_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_187_chunk (
    CONSTRAINT constraint_41 CHECK (((received_at >= '2026-01-01 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-02 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_203_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_203_chunk (
    CONSTRAINT constraint_43 CHECK (((received_at >= '2026-01-02 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-03 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_265_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_265_chunk (
    CONSTRAINT constraint_45 CHECK (((received_at >= '2026-01-03 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-04 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_287_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_287_chunk (
    CONSTRAINT constraint_47 CHECK (((received_at >= '2026-01-04 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-05 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_318_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_318_chunk (
    CONSTRAINT constraint_49 CHECK (((received_at >= '2026-01-05 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-06 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_338_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_338_chunk (
    CONSTRAINT constraint_51 CHECK (((received_at >= '2026-01-06 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-07 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_398_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_398_chunk (
    CONSTRAINT constraint_53 CHECK (((received_at >= '2026-01-07 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-08 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_407_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_407_chunk (
    CONSTRAINT constraint_55 CHECK (((received_at >= '2026-01-08 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-09 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_443_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_443_chunk (
    CONSTRAINT constraint_57 CHECK (((received_at >= '2026-01-09 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-10 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_447_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_447_chunk (
    CONSTRAINT constraint_59 CHECK (((received_at >= '2026-01-10 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-11 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_472_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_472_chunk (
    CONSTRAINT constraint_61 CHECK (((received_at >= '2026-01-11 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-12 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_523_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_523_chunk (
    CONSTRAINT constraint_63 CHECK (((received_at >= '2026-01-12 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-13 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_539_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_539_chunk (
    CONSTRAINT constraint_65 CHECK (((received_at >= '2026-01-13 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-14 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_552_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_552_chunk (
    CONSTRAINT constraint_67 CHECK (((received_at >= '2026-01-14 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-15 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_556_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_556_chunk (
    CONSTRAINT constraint_69 CHECK (((received_at >= '2026-01-15 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-16 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_561_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_561_chunk (
    CONSTRAINT constraint_71 CHECK (((received_at >= '2026-01-16 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-17 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_564_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_564_chunk (
    CONSTRAINT constraint_73 CHECK (((received_at >= '2026-01-17 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-18 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_568_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_568_chunk (
    CONSTRAINT constraint_75 CHECK (((received_at >= '2026-01-18 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-19 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_572_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_572_chunk (
    CONSTRAINT constraint_77 CHECK (((received_at >= '2026-01-19 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-20 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_576_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_576_chunk (
    CONSTRAINT constraint_79 CHECK (((received_at >= '2026-01-20 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-21 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_580_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_580_chunk (
    CONSTRAINT constraint_81 CHECK (((received_at >= '2026-01-21 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-22 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_584_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_584_chunk (
    CONSTRAINT constraint_83 CHECK (((received_at >= '2026-01-22 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-23 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_588_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_588_chunk (
    CONSTRAINT constraint_85 CHECK (((received_at >= '2026-01-23 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-24 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_592_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_592_chunk (
    CONSTRAINT constraint_87 CHECK (((received_at >= '2026-01-24 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-25 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_596_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_596_chunk (
    CONSTRAINT constraint_89 CHECK (((received_at >= '2026-01-25 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-26 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: _hyper_1_73_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_1_73_chunk (
    CONSTRAINT constraint_33 CHECK (((received_at >= '2025-12-28 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-29 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.raw_messages);


--
-- Name: fixes; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
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
    receiver_id uuid,
    raw_message_id uuid NOT NULL,
    altitude_agl_valid boolean DEFAULT false NOT NULL,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)) STORED,
    time_gap_seconds integer,
    source_metadata jsonb,
    CONSTRAINT fixes_track_degrees_check2 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: _hyper_2_112_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_112_chunk (
    CONSTRAINT constraint_36 CHECK (((received_at >= '2025-12-29 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-30 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_136_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_136_chunk (
    CONSTRAINT constraint_38 CHECK (((received_at >= '2025-12-30 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-31 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_161_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_161_chunk (
    CONSTRAINT constraint_40 CHECK (((received_at >= '2025-12-31 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-01 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_188_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_188_chunk (
    CONSTRAINT constraint_42 CHECK (((received_at >= '2026-01-01 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-02 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_204_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_204_chunk (
    CONSTRAINT constraint_44 CHECK (((received_at >= '2026-01-02 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-03 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_266_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_266_chunk (
    CONSTRAINT constraint_46 CHECK (((received_at >= '2026-01-03 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-04 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_288_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_288_chunk (
    CONSTRAINT constraint_48 CHECK (((received_at >= '2026-01-04 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-05 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_319_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_319_chunk (
    CONSTRAINT constraint_50 CHECK (((received_at >= '2026-01-05 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-06 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_31_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_31_chunk (
    CONSTRAINT constraint_31 CHECK (((received_at >= '2025-12-26 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-27 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_32_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_32_chunk (
    CONSTRAINT constraint_32 CHECK (((received_at >= '2025-12-27 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-28 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_339_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_339_chunk (
    CONSTRAINT constraint_52 CHECK (((received_at >= '2026-01-06 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-07 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_399_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_399_chunk (
    CONSTRAINT constraint_54 CHECK (((received_at >= '2026-01-07 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-08 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_408_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_408_chunk (
    CONSTRAINT constraint_56 CHECK (((received_at >= '2026-01-08 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-09 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_444_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_444_chunk (
    CONSTRAINT constraint_58 CHECK (((received_at >= '2026-01-09 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-10 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_448_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_448_chunk (
    CONSTRAINT constraint_60 CHECK (((received_at >= '2026-01-10 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-11 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_473_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_473_chunk (
    CONSTRAINT constraint_62 CHECK (((received_at >= '2026-01-11 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-12 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_524_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_524_chunk (
    CONSTRAINT constraint_64 CHECK (((received_at >= '2026-01-12 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-13 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_540_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_540_chunk (
    CONSTRAINT constraint_66 CHECK (((received_at >= '2026-01-13 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-14 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_553_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_553_chunk (
    CONSTRAINT constraint_68 CHECK (((received_at >= '2026-01-14 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-15 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_557_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_557_chunk (
    CONSTRAINT constraint_70 CHECK (((received_at >= '2026-01-15 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-16 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_562_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_562_chunk (
    CONSTRAINT constraint_72 CHECK (((received_at >= '2026-01-16 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-17 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_565_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_565_chunk (
    CONSTRAINT constraint_74 CHECK (((received_at >= '2026-01-17 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-18 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_569_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_569_chunk (
    CONSTRAINT constraint_76 CHECK (((received_at >= '2026-01-18 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-19 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_573_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_573_chunk (
    CONSTRAINT constraint_78 CHECK (((received_at >= '2026-01-19 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-20 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_577_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_577_chunk (
    CONSTRAINT constraint_80 CHECK (((received_at >= '2026-01-20 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-21 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_581_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_581_chunk (
    CONSTRAINT constraint_82 CHECK (((received_at >= '2026-01-21 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-22 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_585_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_585_chunk (
    CONSTRAINT constraint_84 CHECK (((received_at >= '2026-01-22 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-23 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_589_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_589_chunk (
    CONSTRAINT constraint_86 CHECK (((received_at >= '2026-01-23 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-24 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_593_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_593_chunk (
    CONSTRAINT constraint_88 CHECK (((received_at >= '2026-01-24 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-25 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_597_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_597_chunk (
    CONSTRAINT constraint_90 CHECK (((received_at >= '2026-01-25 00:00:00+00'::timestamp with time zone) AND (received_at < '2026-01-26 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: _hyper_2_74_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal._hyper_2_74_chunk (
    CONSTRAINT constraint_34 CHECK (((received_at >= '2025-12-28 00:00:00+00'::timestamp with time zone) AND (received_at < '2025-12-29 00:00:00+00'::timestamp with time zone)))
)
INHERITS (public.fixes);


--
-- Name: compress_hyper_3_538_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_538_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_538_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_541_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_541_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_541_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_542_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_542_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_542_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_543_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_543_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_543_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_544_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_544_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_544_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_545_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_545_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_545_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_546_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_546_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_546_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_547_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_547_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_547_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_548_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_548_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_548_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_549_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_549_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_549_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_551_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_551_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_551_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_554_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_554_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_554_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_558_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_558_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_558_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_560_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_560_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_560_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_566_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_566_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_566_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_570_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_570_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_570_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_574_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_574_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_574_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_578_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_578_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_578_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_582_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_582_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_582_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_586_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_586_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_586_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_590_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_590_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_590_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_594_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_594_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_594_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_598_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_598_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_598_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_3_630_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_3_630_chunk (
    _ts_meta_count integer,
    receiver_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    unparsed _timescaledb_internal.compressed_data,
    raw_message_hash _timescaledb_internal.compressed_data,
    raw_message _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN receiver_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN unparsed SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN unparsed SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN raw_message_hash SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN raw_message_hash SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN raw_message SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN raw_message SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_3_630_chunk ALTER COLUMN source SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_615_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_615_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_615_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_616_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_616_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_616_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_617_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_617_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_617_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_618_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_618_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_618_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_619_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_619_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_619_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_620_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_620_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_620_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_621_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_621_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_621_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_622_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_622_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_622_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_623_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_623_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_623_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_624_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_624_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_624_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_625_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_625_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_625_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_626_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_626_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_626_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_627_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_627_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_627_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_628_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_628_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_628_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_629_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_629_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_629_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


--
-- Name: compress_hyper_4_631_chunk; Type: TABLE; Schema: _timescaledb_internal; Owner: -
--

CREATE TABLE _timescaledb_internal.compress_hyper_4_631_chunk (
    _ts_meta_count integer,
    aircraft_id uuid,
    _ts_meta_v2_bloomh_id _timescaledb_internal.bloom1,
    id _timescaledb_internal.compressed_data,
    source _timescaledb_internal.compressed_data,
    "timestamp" _timescaledb_internal.compressed_data,
    latitude _timescaledb_internal.compressed_data,
    longitude _timescaledb_internal.compressed_data,
    location _timescaledb_internal.compressed_data,
    altitude_msl_feet _timescaledb_internal.compressed_data,
    flight_number _timescaledb_internal.compressed_data,
    squawk _timescaledb_internal.compressed_data,
    ground_speed_knots _timescaledb_internal.compressed_data,
    track_degrees _timescaledb_internal.compressed_data,
    climb_fpm _timescaledb_internal.compressed_data,
    turn_rate_rot _timescaledb_internal.compressed_data,
    _ts_meta_v2_bloomh_flight_id _timescaledb_internal.bloom1,
    flight_id _timescaledb_internal.compressed_data,
    _ts_meta_min_1 timestamp with time zone,
    _ts_meta_max_1 timestamp with time zone,
    received_at _timescaledb_internal.compressed_data,
    is_active _timescaledb_internal.compressed_data,
    altitude_agl_feet _timescaledb_internal.compressed_data,
    receiver_id _timescaledb_internal.compressed_data,
    raw_message_id _timescaledb_internal.compressed_data,
    altitude_agl_valid _timescaledb_internal.compressed_data,
    location_geom _timescaledb_internal.compressed_data,
    time_gap_seconds _timescaledb_internal.compressed_data,
    source_metadata _timescaledb_internal.compressed_data
)
WITH (toast_tuple_target='128');
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN _ts_meta_count SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN aircraft_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN _ts_meta_v2_bloomh_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN source SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN source SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN "timestamp" SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN latitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN longitude SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN location SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN location SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN altitude_msl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN flight_number SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN flight_number SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN squawk SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN squawk SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN ground_speed_knots SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN track_degrees SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN climb_fpm SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN turn_rate_rot SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN _ts_meta_v2_bloomh_flight_id SET STORAGE EXTERNAL;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN flight_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN _ts_meta_min_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN _ts_meta_max_1 SET STATISTICS 1000;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN received_at SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN is_active SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN altitude_agl_feet SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN receiver_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN raw_message_id SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN altitude_agl_valid SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN location_geom SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN location_geom SET STORAGE EXTENDED;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN time_gap_seconds SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN source_metadata SET STATISTICS 0;
ALTER TABLE ONLY _timescaledb_internal.compress_hyper_4_631_chunk ALTER COLUMN source_metadata SET STORAGE EXTENDED;


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
    registration text,
    competition_number text NOT NULL,
    tracked boolean NOT NULL,
    identified boolean NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    from_ogn_ddb boolean DEFAULT true NOT NULL,
    frequency_mhz numeric(6,3),
    pilot_name text,
    home_base_airport_ident text,
    last_fix_at timestamp with time zone,
    club_id uuid,
    icao_model_code character varying(4),
    adsb_emitter_category public.adsb_emitter_category,
    tracker_device_type text,
    country_code character(2),
    latitude double precision,
    longitude double precision,
    location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (
CASE
    WHEN ((latitude IS NOT NULL) AND (longitude IS NOT NULL)) THEN public.st_setsrid(public.st_makepoint(longitude, latitude), 4326)
    ELSE NULL::public.geometry
END) STORED,
    location_geog public.geography(Point,4326) GENERATED ALWAYS AS (
CASE
    WHEN ((latitude IS NOT NULL) AND (longitude IS NOT NULL)) THEN (public.st_point(longitude, latitude))::public.geography
    ELSE NULL::public.geography
END) STORED,
    icao_type_code text,
    owner_operator text,
    faa_pia boolean,
    faa_ladd boolean,
    year smallint,
    is_military boolean,
    aircraft_category public.aircraft_category,
    engine_count smallint,
    engine_type public.engine_type,
    from_adsbx_ddb boolean DEFAULT false NOT NULL,
    current_fix jsonb,
    images jsonb,
    CONSTRAINT icao_model_code_length_check CHECK (((icao_model_code IS NULL) OR (length((icao_model_code)::text) = ANY (ARRAY[3, 4]))))
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
-- Name: aircraft_types; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aircraft_types (
    icao_code text NOT NULL,
    iata_code text DEFAULT ''::text NOT NULL,
    description text NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    manufacturer text,
    wing_type public.wing_type,
    aircraft_category public.icao_aircraft_category
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
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    location_id uuid
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
    openaip_id text NOT NULL,
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
-- Name: club_tow_fees; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.club_tow_fees (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    club_id uuid NOT NULL,
    max_altitude integer,
    cost numeric(10,2) NOT NULL,
    modified_by uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT club_tow_fees_cost_check CHECK ((cost >= (0)::numeric))
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
    user_id uuid NOT NULL,
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
    timeout_phase public.timeout_phase,
    start_location_id uuid,
    end_location_id uuid,
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
    country_code text,
    geolocation point,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    geocode_attempted_at timestamp with time zone
);


--
-- Name: receiver_coverage_h3; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.receiver_coverage_h3 (
    h3_index bigint NOT NULL,
    resolution smallint NOT NULL,
    receiver_id uuid NOT NULL,
    date date NOT NULL,
    fix_count bigint DEFAULT 0 NOT NULL,
    first_seen_at timestamp with time zone NOT NULL,
    last_seen_at timestamp with time zone NOT NULL,
    min_altitude_msl_feet integer,
    max_altitude_msl_feet integer,
    avg_altitude_msl_feet integer,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
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
    le_elevation_ft integer,
    le_heading_degt numeric(5,2),
    le_displaced_threshold_ft integer,
    he_ident text,
    he_latitude_deg numeric(10,8),
    he_longitude_deg numeric(11,8),
    he_elevation_ft integer,
    he_heading_degt numeric(5,2),
    he_displaced_threshold_ft integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    le_location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (
CASE
    WHEN ((le_latitude_deg IS NOT NULL) AND (le_longitude_deg IS NOT NULL)) THEN public.st_setsrid(public.st_makepoint((le_longitude_deg)::double precision, (le_latitude_deg)::double precision), 4326)
    ELSE NULL::public.geometry
END) STORED,
    he_location_geom public.geometry(Point,4326) GENERATED ALWAYS AS (
CASE
    WHEN ((he_latitude_deg IS NOT NULL) AND (he_longitude_deg IS NOT NULL)) THEN public.st_setsrid(public.st_makepoint((he_longitude_deg)::double precision, (he_latitude_deg)::double precision), 4326)
    ELSE NULL::public.geometry
END) STORED
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
    email character varying(320),
    password_hash character varying(255),
    is_admin boolean DEFAULT false NOT NULL,
    club_id uuid,
    email_verified boolean DEFAULT false NOT NULL,
    password_reset_token character varying(255),
    password_reset_expires_at timestamp with time zone,
    email_verification_token character varying(255),
    email_verification_expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    settings jsonb DEFAULT '{}'::jsonb NOT NULL,
    is_licensed boolean DEFAULT false NOT NULL,
    is_instructor boolean DEFAULT false NOT NULL,
    is_tow_pilot boolean DEFAULT false NOT NULL,
    is_examiner boolean DEFAULT false NOT NULL,
    deleted_at timestamp with time zone,
    CONSTRAINT users_auth_consistency_check CHECK ((((email IS NULL) AND (password_hash IS NULL)) OR ((email IS NOT NULL) AND (password_hash IS NOT NULL))))
);


--
-- Name: watchlist; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.watchlist (
    user_id uuid NOT NULL,
    aircraft_id uuid NOT NULL,
    send_email boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: _hyper_1_111_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_111_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_111_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_111_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_135_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_135_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_135_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_135_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_15_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_15_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_15_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_15_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_160_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_160_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_160_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_160_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_16_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_16_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_16_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_16_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_187_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_187_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_187_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_187_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_203_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_203_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_203_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_203_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_265_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_265_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_265_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_265_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_287_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_287_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_287_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_287_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_318_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_318_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_318_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_318_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_338_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_338_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_338_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_338_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_398_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_398_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_398_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_398_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_407_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_407_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_407_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_407_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_443_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_443_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_443_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_443_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_447_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_447_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_447_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_447_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_472_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_472_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_472_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_472_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_523_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_523_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_523_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_523_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_539_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_539_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_539_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_539_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_552_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_552_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_552_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_552_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_556_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_556_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_556_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_556_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_561_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_561_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_561_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_561_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_564_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_564_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_564_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_564_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_568_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_568_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_568_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_568_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_572_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_572_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_572_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_572_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_576_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_576_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_576_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_576_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_580_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_580_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_580_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_580_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_584_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_584_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_584_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_584_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_588_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_588_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_588_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_588_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_592_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_592_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_592_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_592_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_596_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_596_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_596_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_596_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_1_73_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_73_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_1_73_chunk source; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_73_chunk ALTER COLUMN source SET DEFAULT 'aprs'::public.message_source;


--
-- Name: _hyper_2_112_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_112_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_112_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_112_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_112_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_112_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_136_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_136_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_136_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_136_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_136_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_136_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_161_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_161_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_161_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_161_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_161_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_161_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_188_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_188_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_188_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_188_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_188_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_188_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_204_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_204_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_204_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_204_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_204_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_204_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_266_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_266_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_266_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_266_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_266_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_266_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_288_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_288_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_288_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_288_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_288_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_288_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_319_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_319_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_319_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_319_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_319_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_319_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_31_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_31_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_31_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_31_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_31_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_31_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_32_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_32_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_32_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_32_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_32_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_32_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_339_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_339_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_339_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_339_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_339_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_339_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_399_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_399_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_399_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_399_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_399_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_399_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_408_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_408_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_408_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_408_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_408_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_408_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_444_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_444_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_444_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_444_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_444_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_444_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_448_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_448_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_448_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_448_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_448_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_448_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_473_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_473_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_473_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_473_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_473_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_473_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_524_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_524_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_524_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_524_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_524_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_524_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_540_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_540_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_540_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_540_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_540_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_540_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_553_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_553_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_553_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_553_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_553_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_553_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_557_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_557_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_557_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_557_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_557_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_557_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_562_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_562_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_562_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_562_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_562_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_562_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_565_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_565_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_565_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_565_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_565_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_565_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_569_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_569_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_569_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_569_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_569_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_569_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_573_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_573_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_573_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_573_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_573_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_573_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_577_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_577_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_577_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_577_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_577_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_577_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_581_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_581_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_581_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_581_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_581_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_581_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_585_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_585_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_585_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_585_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_585_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_585_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_589_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_589_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_589_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_589_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_589_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_589_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_593_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_593_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_593_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_593_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_593_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_593_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_597_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_597_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_597_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_597_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_597_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_597_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: _hyper_2_74_chunk id; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_74_chunk ALTER COLUMN id SET DEFAULT gen_random_uuid();


--
-- Name: _hyper_2_74_chunk is_active; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_74_chunk ALTER COLUMN is_active SET DEFAULT true;


--
-- Name: _hyper_2_74_chunk altitude_agl_valid; Type: DEFAULT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_74_chunk ALTER COLUMN altitude_agl_valid SET DEFAULT false;


--
-- Name: receivers_links id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers_links ALTER COLUMN id SET DEFAULT nextval('public.receivers_links_id_seq'::regclass);


--
-- Name: receivers_photos id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receivers_photos ALTER COLUMN id SET DEFAULT nextval('public.receivers_photos_id_seq'::regclass);


--
-- Name: _hyper_1_111_chunk 111_103_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_111_chunk
    ADD CONSTRAINT "111_103_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_112_chunk 112_107_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_112_chunk
    ADD CONSTRAINT "112_107_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_135_chunk 135_109_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_135_chunk
    ADD CONSTRAINT "135_109_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_136_chunk 136_113_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_136_chunk
    ADD CONSTRAINT "136_113_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_15_chunk 15_15_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_15_chunk
    ADD CONSTRAINT "15_15_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_160_chunk 160_115_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_160_chunk
    ADD CONSTRAINT "160_115_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_161_chunk 161_119_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_161_chunk
    ADD CONSTRAINT "161_119_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_16_chunk 16_16_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_16_chunk
    ADD CONSTRAINT "16_16_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_187_chunk 187_121_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_187_chunk
    ADD CONSTRAINT "187_121_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_188_chunk 188_125_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_188_chunk
    ADD CONSTRAINT "188_125_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_203_chunk 203_127_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_203_chunk
    ADD CONSTRAINT "203_127_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_204_chunk 204_131_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_204_chunk
    ADD CONSTRAINT "204_131_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_265_chunk 265_133_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_265_chunk
    ADD CONSTRAINT "265_133_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_266_chunk 266_137_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_266_chunk
    ADD CONSTRAINT "266_137_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_287_chunk 287_139_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_287_chunk
    ADD CONSTRAINT "287_139_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_288_chunk 288_143_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_288_chunk
    ADD CONSTRAINT "288_143_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_318_chunk 318_145_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_318_chunk
    ADD CONSTRAINT "318_145_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_319_chunk 319_149_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_319_chunk
    ADD CONSTRAINT "319_149_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_31_chunk 31_47_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_31_chunk
    ADD CONSTRAINT "31_47_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_32_chunk 32_48_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_32_chunk
    ADD CONSTRAINT "32_48_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_338_chunk 338_151_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_338_chunk
    ADD CONSTRAINT "338_151_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_339_chunk 339_155_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_339_chunk
    ADD CONSTRAINT "339_155_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_398_chunk 398_183_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_398_chunk
    ADD CONSTRAINT "398_183_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_399_chunk 399_187_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_399_chunk
    ADD CONSTRAINT "399_187_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_407_chunk 407_189_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_407_chunk
    ADD CONSTRAINT "407_189_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_408_chunk 408_193_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_408_chunk
    ADD CONSTRAINT "408_193_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_443_chunk 443_195_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_443_chunk
    ADD CONSTRAINT "443_195_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_444_chunk 444_199_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_444_chunk
    ADD CONSTRAINT "444_199_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_447_chunk 447_201_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_447_chunk
    ADD CONSTRAINT "447_201_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_448_chunk 448_205_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_448_chunk
    ADD CONSTRAINT "448_205_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_472_chunk 472_207_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_472_chunk
    ADD CONSTRAINT "472_207_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_473_chunk 473_211_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_473_chunk
    ADD CONSTRAINT "473_211_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_523_chunk 523_213_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_523_chunk
    ADD CONSTRAINT "523_213_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_524_chunk 524_217_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_524_chunk
    ADD CONSTRAINT "524_217_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_539_chunk 539_219_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_539_chunk
    ADD CONSTRAINT "539_219_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_540_chunk 540_223_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_540_chunk
    ADD CONSTRAINT "540_223_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_552_chunk 552_225_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_552_chunk
    ADD CONSTRAINT "552_225_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_553_chunk 553_229_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_553_chunk
    ADD CONSTRAINT "553_229_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_556_chunk 556_231_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_556_chunk
    ADD CONSTRAINT "556_231_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_557_chunk 557_235_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_557_chunk
    ADD CONSTRAINT "557_235_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_561_chunk 561_237_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_561_chunk
    ADD CONSTRAINT "561_237_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_562_chunk 562_241_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_562_chunk
    ADD CONSTRAINT "562_241_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_564_chunk 564_243_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_564_chunk
    ADD CONSTRAINT "564_243_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_565_chunk 565_247_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_565_chunk
    ADD CONSTRAINT "565_247_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_568_chunk 568_249_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_568_chunk
    ADD CONSTRAINT "568_249_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_569_chunk 569_253_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_569_chunk
    ADD CONSTRAINT "569_253_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_572_chunk 572_255_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_572_chunk
    ADD CONSTRAINT "572_255_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_573_chunk 573_259_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_573_chunk
    ADD CONSTRAINT "573_259_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_576_chunk 576_261_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_576_chunk
    ADD CONSTRAINT "576_261_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_577_chunk 577_265_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_577_chunk
    ADD CONSTRAINT "577_265_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_580_chunk 580_267_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_580_chunk
    ADD CONSTRAINT "580_267_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_581_chunk 581_271_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_581_chunk
    ADD CONSTRAINT "581_271_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_584_chunk 584_273_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_584_chunk
    ADD CONSTRAINT "584_273_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_585_chunk 585_277_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_585_chunk
    ADD CONSTRAINT "585_277_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_588_chunk 588_279_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_588_chunk
    ADD CONSTRAINT "588_279_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_589_chunk 589_283_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_589_chunk
    ADD CONSTRAINT "589_283_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_592_chunk 592_285_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_592_chunk
    ADD CONSTRAINT "592_285_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_593_chunk 593_289_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_593_chunk
    ADD CONSTRAINT "593_289_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_596_chunk 596_291_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_596_chunk
    ADD CONSTRAINT "596_291_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_597_chunk 597_295_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_597_chunk
    ADD CONSTRAINT "597_295_fixes_pkey2" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_1_73_chunk 73_97_raw_messages_pkey1; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_73_chunk
    ADD CONSTRAINT "73_97_raw_messages_pkey1" PRIMARY KEY (id, received_at);


--
-- Name: _hyper_2_74_chunk 74_101_fixes_pkey2; Type: CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_74_chunk
    ADD CONSTRAINT "74_101_fixes_pkey2" PRIMARY KEY (id, received_at);


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
-- Name: aircraft_types aircraft_types_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_types
    ADD CONSTRAINT aircraft_types_pkey PRIMARY KEY (icao_code, iata_code);


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
-- Name: club_analytics_daily club_analytics_daily_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.club_analytics_daily
    ADD CONSTRAINT club_analytics_daily_pkey PRIMARY KEY (club_id, date);


--
-- Name: club_tow_fees club_tow_fees_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.club_tow_fees
    ADD CONSTRAINT club_tow_fees_pkey PRIMARY KEY (id);


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
-- Name: fixes fixes_pkey2; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_pkey2 PRIMARY KEY (id, received_at);


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
    ADD CONSTRAINT flight_pilots_flight_id_pilot_id_key UNIQUE (flight_id, user_id);


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
-- Name: raw_messages raw_messages_pkey1; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages
    ADD CONSTRAINT raw_messages_pkey1 PRIMARY KEY (id, received_at);


--
-- Name: receiver_coverage_h3 receiver_coverage_h3_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_coverage_h3
    ADD CONSTRAINT receiver_coverage_h3_pkey PRIMARY KEY (h3_index, resolution, receiver_id, date);


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
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: watchlist watchlist_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.watchlist
    ADD CONSTRAINT watchlist_pkey PRIMARY KEY (user_id, aircraft_id);


--
-- Name: _hyper_1_111_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_111_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_111_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_111_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_111_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_111_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_135_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_135_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_135_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_135_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_135_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_135_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_15_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_15_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_15_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_15_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_15_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_15_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_160_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_160_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_160_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_160_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_160_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_160_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_16_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_16_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_16_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_16_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_16_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_16_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_187_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_187_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_187_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_187_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_187_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_187_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_203_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_203_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_203_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_203_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_203_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_203_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_265_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_265_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_265_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_265_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_265_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_265_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_287_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_287_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_287_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_287_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_287_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_287_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_318_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_318_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_318_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_318_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_318_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_318_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_338_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_338_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_338_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_338_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_338_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_338_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_398_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_398_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_398_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_398_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_398_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_398_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_407_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_407_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_407_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_407_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_407_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_407_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_443_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_443_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_443_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_443_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_443_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_443_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_447_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_447_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_447_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_447_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_447_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_447_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_472_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_472_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_472_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_472_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_472_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_472_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_523_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_523_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_523_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_523_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_523_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_523_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_539_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_539_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_539_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_539_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_539_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_539_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_552_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_552_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_552_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_552_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_552_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_552_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_556_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_556_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_556_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_556_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_556_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_556_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_561_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_561_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_561_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_561_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_561_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_561_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_564_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_564_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_564_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_564_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_564_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_564_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_568_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_568_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_568_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_568_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_568_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_568_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_572_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_572_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_572_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_572_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_572_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_572_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_576_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_576_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_576_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_576_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_576_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_576_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_580_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_580_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_580_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_580_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_580_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_580_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_584_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_584_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_584_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_584_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_584_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_584_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_588_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_588_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_588_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_588_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_588_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_588_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_592_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_592_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_592_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_592_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_592_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_592_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_596_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_596_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_596_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_596_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_596_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_596_chunk USING btree (received_at DESC);


--
-- Name: _hyper_1_73_chunk_idx_raw_messages_receiver_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_73_chunk_idx_raw_messages_receiver_id ON _timescaledb_internal._hyper_1_73_chunk USING btree (receiver_id);


--
-- Name: _hyper_1_73_chunk_raw_messages_received_at_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_1_73_chunk_raw_messages_received_at_idx ON _timescaledb_internal._hyper_1_73_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_112_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_112_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_112_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_112_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_112_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_112_chunk USING btree (flight_id);


--
-- Name: _hyper_2_136_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_136_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_136_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_136_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_136_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_136_chunk USING btree (flight_id);


--
-- Name: _hyper_2_161_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_161_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_161_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_161_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_161_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_161_chunk USING btree (flight_id);


--
-- Name: _hyper_2_188_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_188_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_188_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_188_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_188_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_188_chunk USING btree (flight_id);


--
-- Name: _hyper_2_204_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_204_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_204_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_204_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_204_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_204_chunk USING btree (flight_id);


--
-- Name: _hyper_2_266_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_266_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_266_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_266_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_266_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_266_chunk USING btree (flight_id);


--
-- Name: _hyper_2_288_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_288_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_288_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_288_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_288_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_288_chunk USING btree (flight_id);


--
-- Name: _hyper_2_319_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_319_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_319_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_319_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_319_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_319_chunk USING btree (flight_id);


--
-- Name: _hyper_2_31_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_31_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_31_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_31_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_31_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_31_chunk USING btree (flight_id);


--
-- Name: _hyper_2_32_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_32_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_32_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_32_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_32_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_32_chunk USING btree (flight_id);


--
-- Name: _hyper_2_339_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_339_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_339_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_339_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_339_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_339_chunk USING btree (flight_id);


--
-- Name: _hyper_2_399_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_399_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_399_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_399_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_399_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_399_chunk USING btree (flight_id);


--
-- Name: _hyper_2_408_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_408_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_408_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_408_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_408_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_408_chunk USING btree (flight_id);


--
-- Name: _hyper_2_444_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_444_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_444_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_444_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_444_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_444_chunk USING btree (flight_id);


--
-- Name: _hyper_2_448_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_448_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_448_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_448_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_448_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_448_chunk USING btree (flight_id);


--
-- Name: _hyper_2_473_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_473_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_473_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_473_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_473_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_473_chunk USING btree (flight_id);


--
-- Name: _hyper_2_524_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_524_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_524_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_524_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_524_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_524_chunk USING btree (flight_id);


--
-- Name: _hyper_2_540_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_540_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_540_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_540_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_540_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_540_chunk USING btree (flight_id);


--
-- Name: _hyper_2_553_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_553_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_553_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_553_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_553_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_553_chunk USING btree (flight_id);


--
-- Name: _hyper_2_557_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_557_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_557_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_557_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_557_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_557_chunk USING btree (flight_id);


--
-- Name: _hyper_2_562_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_562_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_562_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_562_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_562_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_562_chunk USING btree (flight_id);


--
-- Name: _hyper_2_565_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_565_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_565_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_565_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_565_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_565_chunk USING btree (flight_id);


--
-- Name: _hyper_2_569_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_569_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_569_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_569_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_569_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_569_chunk USING btree (flight_id);


--
-- Name: _hyper_2_573_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_573_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_573_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_573_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_573_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_573_chunk USING btree (flight_id);


--
-- Name: _hyper_2_577_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_577_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_577_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_577_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_577_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_577_chunk USING btree (flight_id);


--
-- Name: _hyper_2_581_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_581_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_581_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_581_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_581_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_581_chunk USING btree (flight_id);


--
-- Name: _hyper_2_585_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_585_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_585_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_585_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_585_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_585_chunk USING btree (flight_id);


--
-- Name: _hyper_2_589_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_589_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_589_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_589_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_589_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_589_chunk USING btree (flight_id);


--
-- Name: _hyper_2_593_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_593_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_593_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_593_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_593_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_593_chunk USING btree (flight_id);


--
-- Name: _hyper_2_597_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_597_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_597_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_597_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_597_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_597_chunk USING btree (flight_id);


--
-- Name: _hyper_2_74_chunk_fixes_received_at_idx1; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_74_chunk_fixes_received_at_idx1 ON _timescaledb_internal._hyper_2_74_chunk USING btree (received_at DESC);


--
-- Name: _hyper_2_74_chunk_idx_fixes_flight_id; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX _hyper_2_74_chunk_idx_fixes_flight_id ON _timescaledb_internal._hyper_2_74_chunk USING btree (flight_id);


--
-- Name: compress_hyper_3_538_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_538_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_538_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_541_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_541_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_541_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_542_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_542_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_542_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_543_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_543_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_543_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_544_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_544_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_544_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_545_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_545_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_545_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_546_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_546_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_546_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_547_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_547_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_547_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_548_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_548_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_548_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_549_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_549_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_549_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_551_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_551_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_551_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_554_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_554_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_554_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_558_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_558_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_558_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_560_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_560_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_560_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_566_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_566_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_566_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_570_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_570_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_570_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_574_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_574_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_574_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_578_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_578_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_578_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_582_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_582_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_582_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_586_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_586_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_586_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_590_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_590_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_590_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_594_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_594_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_594_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_598_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_598_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_598_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_3_630_chunk_receiver_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_3_630_chunk_receiver_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_3_630_chunk USING btree (receiver_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_615_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_615_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_615_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_616_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_616_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_616_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_617_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_617_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_617_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_618_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_618_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_618_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_619_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_619_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_619_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_620_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_620_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_620_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_621_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_621_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_621_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_622_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_622_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_622_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_623_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_623_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_623_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_624_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_624_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_624_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_625_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_625_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_625_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_626_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_626_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_626_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_627_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_627_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_627_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_628_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_628_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_628_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_629_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_629_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_629_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


--
-- Name: compress_hyper_4_631_chunk_aircraft_id__ts_meta_min_1__ts_m_idx; Type: INDEX; Schema: _timescaledb_internal; Owner: -
--

CREATE INDEX compress_hyper_4_631_chunk_aircraft_id__ts_meta_min_1__ts_m_idx ON _timescaledb_internal.compress_hyper_4_631_chunk USING btree (aircraft_id, _ts_meta_min_1 DESC, _ts_meta_max_1 DESC);


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
-- Name: fixes_received_at_idx1; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_received_at_idx1 ON public.fixes USING btree (received_at DESC);


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
-- Name: idx_aircraft_category; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_category ON public.aircraft USING btree (aircraft_category) WHERE (aircraft_category IS NOT NULL);


--
-- Name: idx_aircraft_country_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_country_code ON public.aircraft USING btree (country_code);


--
-- Name: idx_aircraft_engine_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_engine_type ON public.aircraft USING btree (engine_type) WHERE (engine_type IS NOT NULL);


--
-- Name: idx_aircraft_faa_ladd; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_faa_ladd ON public.aircraft USING btree (faa_ladd) WHERE (faa_ladd = true);


--
-- Name: idx_aircraft_faa_pia; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_faa_pia ON public.aircraft USING btree (faa_pia) WHERE (faa_pia = true);


--
-- Name: idx_aircraft_from_ddb; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_from_ddb ON public.aircraft USING btree (from_ogn_ddb);


--
-- Name: idx_aircraft_icao_model_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_icao_model_code ON public.aircraft USING btree (icao_model_code) WHERE (icao_model_code IS NOT NULL);


--
-- Name: idx_aircraft_icao_type_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_icao_type_code ON public.aircraft USING btree (icao_type_code) WHERE (icao_type_code IS NOT NULL);


--
-- Name: idx_aircraft_identified; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_identified ON public.aircraft USING btree (identified);


--
-- Name: idx_aircraft_images; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_images ON public.aircraft USING gin (images);


--
-- Name: idx_aircraft_is_military; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_is_military ON public.aircraft USING btree (is_military) WHERE (is_military = true);


--
-- Name: idx_aircraft_last_fix_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_last_fix_at ON public.aircraft USING btree (last_fix_at) WHERE (last_fix_at IS NOT NULL);


--
-- Name: idx_aircraft_lat_lon; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_lat_lon ON public.aircraft USING btree (latitude, longitude) WHERE ((latitude IS NOT NULL) AND (longitude IS NOT NULL));


--
-- Name: idx_aircraft_location_geog; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_location_geog ON public.aircraft USING gist (location_geog) WHERE (location_geog IS NOT NULL);


--
-- Name: idx_aircraft_location_geom; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_location_geom ON public.aircraft USING gist (location_geom) WHERE (location_geom IS NOT NULL);


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
-- Name: idx_aircraft_owner_operator; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_owner_operator ON public.aircraft USING btree (owner_operator) WHERE (owner_operator IS NOT NULL);


--
-- Name: idx_aircraft_registration_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_aircraft_registration_unique ON public.aircraft USING btree (registration) WHERE (registration IS NOT NULL);


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
-- Name: idx_aircraft_types_description; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_types_description ON public.aircraft_types USING btree (description);


--
-- Name: idx_aircraft_types_iata_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_types_iata_code ON public.aircraft_types USING btree (iata_code);


--
-- Name: idx_aircraft_year; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_year ON public.aircraft USING btree (year) WHERE (year IS NOT NULL);


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
-- Name: idx_airports_location_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_airports_location_id ON public.airports USING btree (location_id);


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
-- Name: idx_club_tow_fees_club_altitude; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_club_tow_fees_club_altitude ON public.club_tow_fees USING btree (club_id, max_altitude) WHERE (max_altitude IS NOT NULL);


--
-- Name: idx_club_tow_fees_club_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_club_tow_fees_club_id ON public.club_tow_fees USING btree (club_id);


--
-- Name: idx_club_tow_fees_club_null_altitude; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_club_tow_fees_club_null_altitude ON public.club_tow_fees USING btree (club_id) WHERE (max_altitude IS NULL);


--
-- Name: idx_coverage_h3_receiver_date; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_coverage_h3_receiver_date ON public.receiver_coverage_h3 USING btree (receiver_id, date DESC);


--
-- Name: idx_coverage_h3_resolution_date; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_coverage_h3_resolution_date ON public.receiver_coverage_h3 USING btree (resolution, date DESC);


--
-- Name: idx_fixes_flight_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_flight_id ON public.fixes USING btree (flight_id);


--
-- Name: idx_flight_analytics_daily_date_desc; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flight_analytics_daily_date_desc ON public.flight_analytics_daily USING btree (date DESC);


--
-- Name: idx_flight_analytics_hourly_hour_desc; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flight_analytics_hourly_hour_desc ON public.flight_analytics_hourly USING btree (hour DESC);


--
-- Name: idx_flight_pilots_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flight_pilots_user_id ON public.flight_pilots USING btree (user_id);


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
-- Name: idx_flights_end_location_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_end_location_id ON public.flights USING btree (end_location_id);


--
-- Name: idx_flights_landing_location_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_landing_location_id ON public.flights USING btree (landing_location_id);


--
-- Name: idx_flights_last_fix_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_last_fix_at ON public.flights USING btree (last_fix_at);


--
-- Name: idx_flights_start_location_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_start_location_id ON public.flights USING btree (start_location_id);


--
-- Name: idx_flights_takeoff_location_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_takeoff_location_id ON public.flights USING btree (takeoff_location_id);


--
-- Name: idx_flights_timeout_phase; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_timeout_phase ON public.flights USING btree (timeout_phase) WHERE (timeout_phase IS NOT NULL);


--
-- Name: idx_flights_towed_by_aircraft; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_towed_by_aircraft ON public.flights USING btree (towed_by_aircraft_id) WHERE (towed_by_aircraft_id IS NOT NULL);


--
-- Name: idx_flights_towed_by_flight; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_towed_by_flight ON public.flights USING btree (towed_by_flight_id) WHERE (towed_by_flight_id IS NOT NULL);


--
-- Name: idx_locations_needs_geocoding; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_locations_needs_geocoding ON public.locations USING btree (geocode_attempted_at) WHERE ((geolocation IS NULL) AND (geocode_attempted_at IS NULL));


--
-- Name: idx_raw_messages_receiver_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_raw_messages_receiver_id ON public.raw_messages USING btree (receiver_id);


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
-- Name: idx_runways_he_location_geom; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_he_location_geom ON public.runways USING gist (he_location_geom) WHERE (he_location_geom IS NOT NULL);


--
-- Name: idx_runways_le_location_geom; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_runways_le_location_geom ON public.runways USING gist (le_location_geom) WHERE (le_location_geom IS NOT NULL);


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
-- Name: idx_user_fixes_user_timestamp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_fixes_user_timestamp ON public.user_fixes USING btree (user_id, "timestamp" DESC);


--
-- Name: idx_users_deleted_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_users_deleted_at ON public.users USING btree (deleted_at) WHERE (deleted_at IS NULL);


--
-- Name: idx_watchlist_aircraft_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_watchlist_aircraft_id ON public.watchlist USING btree (aircraft_id);


--
-- Name: idx_watchlist_send_email; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_watchlist_send_email ON public.watchlist USING btree (aircraft_id) WHERE (send_email = true);


--
-- Name: locations_address_unique_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX locations_address_unique_idx ON public.locations USING btree (street1, street2, city, state, zip_code, COALESCE(country_code, 'US'::text));


--
-- Name: locations_geolocation_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX locations_geolocation_idx ON public.locations USING gist (geolocation);


--
-- Name: raw_messages_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX raw_messages_received_at_idx ON public.raw_messages USING btree (received_at DESC);


--
-- Name: users_club_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_club_id_idx ON public.users USING btree (club_id);


--
-- Name: users_email_unique_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX users_email_unique_idx ON public.users USING btree (email) WHERE (email IS NOT NULL);


--
-- Name: users_email_verification_token_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_email_verification_token_idx ON public.users USING btree (email_verification_token);


--
-- Name: users_password_reset_token_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_password_reset_token_idx ON public.users USING btree (password_reset_token);


--
-- Name: watchlist set_watchlist_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_watchlist_updated_at BEFORE UPDATE ON public.watchlist FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


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
-- Name: runways update_runways_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_runways_updated_at BEFORE UPDATE ON public.runways FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: users update_users_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: _hyper_1_111_chunk 111_174_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_111_chunk
    ADD CONSTRAINT "111_174_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_112_chunk 112_106_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_112_chunk
    ADD CONSTRAINT "112_106_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_112_chunk 112_108_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_112_chunk
    ADD CONSTRAINT "112_108_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_112_chunk 112_300_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_112_chunk
    ADD CONSTRAINT "112_300_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_135_chunk 135_175_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_135_chunk
    ADD CONSTRAINT "135_175_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_136_chunk 136_112_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_136_chunk
    ADD CONSTRAINT "136_112_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_136_chunk 136_114_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_136_chunk
    ADD CONSTRAINT "136_114_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_136_chunk 136_301_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_136_chunk
    ADD CONSTRAINT "136_301_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_15_chunk 15_171_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_15_chunk
    ADD CONSTRAINT "15_171_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_1_160_chunk 160_176_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_160_chunk
    ADD CONSTRAINT "160_176_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_161_chunk 161_118_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_161_chunk
    ADD CONSTRAINT "161_118_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_161_chunk 161_120_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_161_chunk
    ADD CONSTRAINT "161_120_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_161_chunk 161_302_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_161_chunk
    ADD CONSTRAINT "161_302_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_16_chunk 16_172_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_16_chunk
    ADD CONSTRAINT "16_172_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_1_187_chunk 187_177_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_187_chunk
    ADD CONSTRAINT "187_177_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_188_chunk 188_124_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_188_chunk
    ADD CONSTRAINT "188_124_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_188_chunk 188_126_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_188_chunk
    ADD CONSTRAINT "188_126_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_188_chunk 188_303_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_188_chunk
    ADD CONSTRAINT "188_303_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_203_chunk 203_178_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_203_chunk
    ADD CONSTRAINT "203_178_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_204_chunk 204_130_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_204_chunk
    ADD CONSTRAINT "204_130_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_204_chunk 204_132_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_204_chunk
    ADD CONSTRAINT "204_132_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_204_chunk 204_304_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_204_chunk
    ADD CONSTRAINT "204_304_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_265_chunk 265_179_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_265_chunk
    ADD CONSTRAINT "265_179_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_266_chunk 266_136_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_266_chunk
    ADD CONSTRAINT "266_136_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_266_chunk 266_138_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_266_chunk
    ADD CONSTRAINT "266_138_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_266_chunk 266_305_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_266_chunk
    ADD CONSTRAINT "266_305_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_287_chunk 287_180_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_287_chunk
    ADD CONSTRAINT "287_180_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_288_chunk 288_142_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_288_chunk
    ADD CONSTRAINT "288_142_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_288_chunk 288_144_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_288_chunk
    ADD CONSTRAINT "288_144_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_288_chunk 288_306_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_288_chunk
    ADD CONSTRAINT "288_306_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_318_chunk 318_181_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_318_chunk
    ADD CONSTRAINT "318_181_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_319_chunk 319_148_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_319_chunk
    ADD CONSTRAINT "319_148_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_319_chunk 319_150_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_319_chunk
    ADD CONSTRAINT "319_150_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_319_chunk 319_307_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_319_chunk
    ADD CONSTRAINT "319_307_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_2_31_chunk 31_297_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_31_chunk
    ADD CONSTRAINT "31_297_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_2_31_chunk 31_79_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_31_chunk
    ADD CONSTRAINT "31_79_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_31_chunk 31_95_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_31_chunk
    ADD CONSTRAINT "31_95_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_32_chunk 32_298_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_32_chunk
    ADD CONSTRAINT "32_298_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_2_32_chunk 32_80_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_32_chunk
    ADD CONSTRAINT "32_80_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_32_chunk 32_96_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_32_chunk
    ADD CONSTRAINT "32_96_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_1_338_chunk 338_182_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_338_chunk
    ADD CONSTRAINT "338_182_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_339_chunk 339_154_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_339_chunk
    ADD CONSTRAINT "339_154_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_339_chunk 339_156_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_339_chunk
    ADD CONSTRAINT "339_156_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_339_chunk 339_308_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_339_chunk
    ADD CONSTRAINT "339_308_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_398_chunk 398_184_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_398_chunk
    ADD CONSTRAINT "398_184_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_399_chunk 399_186_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_399_chunk
    ADD CONSTRAINT "399_186_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_399_chunk 399_188_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_399_chunk
    ADD CONSTRAINT "399_188_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_399_chunk 399_309_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_399_chunk
    ADD CONSTRAINT "399_309_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_407_chunk 407_190_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_407_chunk
    ADD CONSTRAINT "407_190_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_408_chunk 408_192_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_408_chunk
    ADD CONSTRAINT "408_192_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_408_chunk 408_194_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_408_chunk
    ADD CONSTRAINT "408_194_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_408_chunk 408_310_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_408_chunk
    ADD CONSTRAINT "408_310_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_443_chunk 443_196_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_443_chunk
    ADD CONSTRAINT "443_196_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_444_chunk 444_198_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_444_chunk
    ADD CONSTRAINT "444_198_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_444_chunk 444_200_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_444_chunk
    ADD CONSTRAINT "444_200_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_444_chunk 444_311_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_444_chunk
    ADD CONSTRAINT "444_311_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_447_chunk 447_202_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_447_chunk
    ADD CONSTRAINT "447_202_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_448_chunk 448_204_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_448_chunk
    ADD CONSTRAINT "448_204_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_448_chunk 448_206_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_448_chunk
    ADD CONSTRAINT "448_206_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_448_chunk 448_312_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_448_chunk
    ADD CONSTRAINT "448_312_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_472_chunk 472_208_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_472_chunk
    ADD CONSTRAINT "472_208_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_473_chunk 473_210_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_473_chunk
    ADD CONSTRAINT "473_210_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_473_chunk 473_212_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_473_chunk
    ADD CONSTRAINT "473_212_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_473_chunk 473_313_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_473_chunk
    ADD CONSTRAINT "473_313_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_523_chunk 523_214_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_523_chunk
    ADD CONSTRAINT "523_214_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_524_chunk 524_216_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_524_chunk
    ADD CONSTRAINT "524_216_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_524_chunk 524_218_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_524_chunk
    ADD CONSTRAINT "524_218_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_524_chunk 524_314_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_524_chunk
    ADD CONSTRAINT "524_314_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_539_chunk 539_220_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_539_chunk
    ADD CONSTRAINT "539_220_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_540_chunk 540_222_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_540_chunk
    ADD CONSTRAINT "540_222_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_540_chunk 540_224_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_540_chunk
    ADD CONSTRAINT "540_224_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_540_chunk 540_315_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_540_chunk
    ADD CONSTRAINT "540_315_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_552_chunk 552_226_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_552_chunk
    ADD CONSTRAINT "552_226_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_553_chunk 553_228_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_553_chunk
    ADD CONSTRAINT "553_228_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_553_chunk 553_230_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_553_chunk
    ADD CONSTRAINT "553_230_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_553_chunk 553_316_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_553_chunk
    ADD CONSTRAINT "553_316_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_556_chunk 556_232_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_556_chunk
    ADD CONSTRAINT "556_232_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_557_chunk 557_234_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_557_chunk
    ADD CONSTRAINT "557_234_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_557_chunk 557_236_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_557_chunk
    ADD CONSTRAINT "557_236_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_557_chunk 557_317_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_557_chunk
    ADD CONSTRAINT "557_317_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_561_chunk 561_238_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_561_chunk
    ADD CONSTRAINT "561_238_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_562_chunk 562_240_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_562_chunk
    ADD CONSTRAINT "562_240_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_562_chunk 562_242_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_562_chunk
    ADD CONSTRAINT "562_242_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_562_chunk 562_318_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_562_chunk
    ADD CONSTRAINT "562_318_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_564_chunk 564_244_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_564_chunk
    ADD CONSTRAINT "564_244_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_565_chunk 565_246_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_565_chunk
    ADD CONSTRAINT "565_246_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_565_chunk 565_248_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_565_chunk
    ADD CONSTRAINT "565_248_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_565_chunk 565_319_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_565_chunk
    ADD CONSTRAINT "565_319_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_568_chunk 568_250_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_568_chunk
    ADD CONSTRAINT "568_250_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_569_chunk 569_252_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_569_chunk
    ADD CONSTRAINT "569_252_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_569_chunk 569_254_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_569_chunk
    ADD CONSTRAINT "569_254_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_569_chunk 569_320_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_569_chunk
    ADD CONSTRAINT "569_320_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_572_chunk 572_256_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_572_chunk
    ADD CONSTRAINT "572_256_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_573_chunk 573_258_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_573_chunk
    ADD CONSTRAINT "573_258_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_573_chunk 573_260_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_573_chunk
    ADD CONSTRAINT "573_260_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_573_chunk 573_321_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_573_chunk
    ADD CONSTRAINT "573_321_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_576_chunk 576_262_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_576_chunk
    ADD CONSTRAINT "576_262_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_577_chunk 577_264_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_577_chunk
    ADD CONSTRAINT "577_264_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_577_chunk 577_266_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_577_chunk
    ADD CONSTRAINT "577_266_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_577_chunk 577_322_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_577_chunk
    ADD CONSTRAINT "577_322_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_580_chunk 580_268_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_580_chunk
    ADD CONSTRAINT "580_268_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_581_chunk 581_270_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_581_chunk
    ADD CONSTRAINT "581_270_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_581_chunk 581_272_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_581_chunk
    ADD CONSTRAINT "581_272_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_581_chunk 581_323_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_581_chunk
    ADD CONSTRAINT "581_323_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_584_chunk 584_274_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_584_chunk
    ADD CONSTRAINT "584_274_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_585_chunk 585_276_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_585_chunk
    ADD CONSTRAINT "585_276_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_585_chunk 585_278_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_585_chunk
    ADD CONSTRAINT "585_278_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_585_chunk 585_324_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_585_chunk
    ADD CONSTRAINT "585_324_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_588_chunk 588_280_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_588_chunk
    ADD CONSTRAINT "588_280_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_589_chunk 589_282_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_589_chunk
    ADD CONSTRAINT "589_282_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_589_chunk 589_284_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_589_chunk
    ADD CONSTRAINT "589_284_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_589_chunk 589_325_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_589_chunk
    ADD CONSTRAINT "589_325_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_592_chunk 592_286_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_592_chunk
    ADD CONSTRAINT "592_286_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_593_chunk 593_288_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_593_chunk
    ADD CONSTRAINT "593_288_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_593_chunk 593_290_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_593_chunk
    ADD CONSTRAINT "593_290_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_593_chunk 593_326_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_593_chunk
    ADD CONSTRAINT "593_326_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_596_chunk 596_292_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_596_chunk
    ADD CONSTRAINT "596_292_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_597_chunk 597_294_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_597_chunk
    ADD CONSTRAINT "597_294_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_597_chunk 597_296_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_597_chunk
    ADD CONSTRAINT "597_296_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_597_chunk 597_327_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_597_chunk
    ADD CONSTRAINT "597_327_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: _hyper_1_73_chunk 73_173_raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_1_73_chunk
    ADD CONSTRAINT "73_173_raw_messages_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: _hyper_2_74_chunk 74_100_fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_74_chunk
    ADD CONSTRAINT "74_100_fixes_flight_id_fkey" FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: _hyper_2_74_chunk 74_102_fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_74_chunk
    ADD CONSTRAINT "74_102_fixes_receiver_id_fkey" FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: _hyper_2_74_chunk 74_299_fixes_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: _timescaledb_internal; Owner: -
--

ALTER TABLE ONLY _timescaledb_internal._hyper_2_74_chunk
    ADD CONSTRAINT "74_299_fixes_aircraft_id_fkey" FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


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
-- Name: airports airports_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.airports
    ADD CONSTRAINT airports_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id);


--
-- Name: club_tow_fees club_tow_fees_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.club_tow_fees
    ADD CONSTRAINT club_tow_fees_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id) ON DELETE CASCADE;


--
-- Name: club_tow_fees club_tow_fees_modified_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.club_tow_fees
    ADD CONSTRAINT club_tow_fees_modified_by_fkey FOREIGN KEY (modified_by) REFERENCES public.users(id);


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

ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_aircraft_id_fkey FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) NOT VALID;


--
-- Name: fixes fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE RESTRICT;


--
-- Name: fixes fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes
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
-- Name: flight_pilots flight_pilots_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flight_pilots
    ADD CONSTRAINT flight_pilots_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


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
-- Name: flights flights_end_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_end_location_id_fkey FOREIGN KEY (end_location_id) REFERENCES public.locations(id);


--
-- Name: flights flights_landing_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_landing_location_id_fkey FOREIGN KEY (landing_location_id) REFERENCES public.locations(id);


--
-- Name: flights flights_start_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_start_location_id_fkey FOREIGN KEY (start_location_id) REFERENCES public.locations(id);


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
-- Name: raw_messages raw_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.raw_messages
    ADD CONSTRAINT raw_messages_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: receiver_coverage_h3 receiver_coverage_h3_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_coverage_h3
    ADD CONSTRAINT receiver_coverage_h3_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


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
-- Name: watchlist watchlist_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.watchlist
    ADD CONSTRAINT watchlist_aircraft_id_fkey FOREIGN KEY (aircraft_id) REFERENCES public.aircraft(id) ON DELETE CASCADE;


--
-- Name: watchlist watchlist_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.watchlist
    ADD CONSTRAINT watchlist_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict SOAR
