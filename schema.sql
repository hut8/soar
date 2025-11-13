--
-- PostgreSQL database dump
--

\restrict mKjdUTYIfcMKob3RN2hcQ8kMctDm9SqdZJ7RsbSWA3px10N5Zt0QwZgzXTDBckV

-- Dumped from database version 17.6 (Ubuntu 17.6-2.pgdg22.04+1)
-- Dumped by pg_dump version 17.6 (Ubuntu 17.6-2.pgdg22.04+1)

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
-- Name: template_public_aprs_messages; Type: TABLE; Schema: partman; Owner: -
--

CREATE TABLE partman.template_public_aprs_messages (
    id uuid NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


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
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.__diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
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
    device_id uuid,
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
-- Name: aprs_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
)
PARTITION BY RANGE (received_at);


--
-- Name: aprs_messages_default; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_default (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_old; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_old (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251108; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251108 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251109; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251109 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251110; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251110 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251111; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251111 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251112; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251112 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251113; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251113 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251114; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251114 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251115; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251115 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
);


--
-- Name: aprs_messages_p20251116; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aprs_messages_p20251116 (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    raw_message text NOT NULL,
    received_at timestamp with time zone NOT NULL,
    receiver_id uuid NOT NULL,
    unparsed text,
    raw_message_hash bytea NOT NULL
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
-- Name: devices; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.devices (
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
    snr_db real,
    bit_errors_corrected integer,
    freq_offset_khz real,
    flight_id uuid,
    device_id uuid NOT NULL,
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
    snr_db real,
    bit_errors_corrected integer,
    freq_offset_khz real,
    flight_id uuid,
    device_id uuid NOT NULL,
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
    device_id uuid NOT NULL,
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
-- Name: fixes_p20251109; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251109 (
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
    device_id uuid NOT NULL,
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
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251110; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251110 (
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
    device_id uuid NOT NULL,
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
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251111; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251111 (
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
    device_id uuid NOT NULL,
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
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251112; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251112 (
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
    device_id uuid NOT NULL,
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
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251113; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251113 (
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
    device_id uuid NOT NULL,
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
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251114; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251114 (
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
    device_id uuid NOT NULL,
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
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251115; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251115 (
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
    device_id uuid NOT NULL,
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
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


--
-- Name: fixes_p20251116; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.fixes_p20251116 (
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
    device_id uuid NOT NULL,
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
    CONSTRAINT fixes_track_degrees_check1 CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
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
    device_id uuid,
    takeoff_altitude_offset_ft integer,
    landing_altitude_offset_ft integer,
    takeoff_runway_ident text,
    landing_runway_ident text,
    total_distance_meters double precision,
    maximum_displacement_meters double precision,
    departure_airport_id integer,
    arrival_airport_id integer,
    towed_by_device_id uuid,
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
    CONSTRAINT check_timed_out_or_landed CHECK (((timed_out_at IS NULL) OR (landing_time IS NULL)))
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
    aprs_message_id uuid
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
-- Name: aprs_messages_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_default DEFAULT;


--
-- Name: aprs_messages_p20251108; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251108 FOR VALUES FROM ('2025-11-08 00:00:00+00') TO ('2025-11-09 00:00:00+00');


--
-- Name: aprs_messages_p20251109; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251109 FOR VALUES FROM ('2025-11-09 00:00:00+00') TO ('2025-11-10 00:00:00+00');


--
-- Name: aprs_messages_p20251110; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251110 FOR VALUES FROM ('2025-11-10 00:00:00+00') TO ('2025-11-11 00:00:00+00');


--
-- Name: aprs_messages_p20251111; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251111 FOR VALUES FROM ('2025-11-11 00:00:00+00') TO ('2025-11-12 00:00:00+00');


--
-- Name: aprs_messages_p20251112; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251112 FOR VALUES FROM ('2025-11-12 00:00:00+00') TO ('2025-11-13 00:00:00+00');


--
-- Name: aprs_messages_p20251113; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251113 FOR VALUES FROM ('2025-11-13 00:00:00+00') TO ('2025-11-14 00:00:00+00');


--
-- Name: aprs_messages_p20251114; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251114 FOR VALUES FROM ('2025-11-14 00:00:00+00') TO ('2025-11-15 00:00:00+00');


--
-- Name: aprs_messages_p20251115; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251115 FOR VALUES FROM ('2025-11-15 00:00:00+00') TO ('2025-11-16 00:00:00+00');


--
-- Name: aprs_messages_p20251116; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages ATTACH PARTITION public.aprs_messages_p20251116 FOR VALUES FROM ('2025-11-16 00:00:00+00') TO ('2025-11-17 00:00:00+00');


--
-- Name: fixes_default; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_default DEFAULT;


--
-- Name: fixes_p20251109; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251109 FOR VALUES FROM ('2025-11-09 00:00:00+00') TO ('2025-11-10 00:00:00+00');


--
-- Name: fixes_p20251110; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251110 FOR VALUES FROM ('2025-11-10 00:00:00+00') TO ('2025-11-11 00:00:00+00');


--
-- Name: fixes_p20251111; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251111 FOR VALUES FROM ('2025-11-11 00:00:00+00') TO ('2025-11-12 00:00:00+00');


--
-- Name: fixes_p20251112; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251112 FOR VALUES FROM ('2025-11-12 00:00:00+00') TO ('2025-11-13 00:00:00+00');


--
-- Name: fixes_p20251113; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251113 FOR VALUES FROM ('2025-11-13 00:00:00+00') TO ('2025-11-14 00:00:00+00');


--
-- Name: fixes_p20251114; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251114 FOR VALUES FROM ('2025-11-14 00:00:00+00') TO ('2025-11-15 00:00:00+00');


--
-- Name: fixes_p20251115; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251115 FOR VALUES FROM ('2025-11-15 00:00:00+00') TO ('2025-11-16 00:00:00+00');


--
-- Name: fixes_p20251116; Type: TABLE ATTACH; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes ATTACH PARTITION public.fixes_p20251116 FOR VALUES FROM ('2025-11-16 00:00:00+00') TO ('2025-11-17 00:00:00+00');


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
-- Name: aircraft_registrations aircraft_registrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_pkey PRIMARY KEY (registration_number);


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
-- Name: aprs_messages aprs_messages_pkey1; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages
    ADD CONSTRAINT aprs_messages_pkey1 PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_default aprs_messages_default_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_default
    ADD CONSTRAINT aprs_messages_default_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251108 aprs_messages_p20251108_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251108
    ADD CONSTRAINT aprs_messages_p20251108_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251109 aprs_messages_p20251109_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251109
    ADD CONSTRAINT aprs_messages_p20251109_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251110 aprs_messages_p20251110_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251110
    ADD CONSTRAINT aprs_messages_p20251110_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251111 aprs_messages_p20251111_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251111
    ADD CONSTRAINT aprs_messages_p20251111_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251112 aprs_messages_p20251112_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251112
    ADD CONSTRAINT aprs_messages_p20251112_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251113 aprs_messages_p20251113_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251113
    ADD CONSTRAINT aprs_messages_p20251113_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251114 aprs_messages_p20251114_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251114
    ADD CONSTRAINT aprs_messages_p20251114_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251115 aprs_messages_p20251115_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251115
    ADD CONSTRAINT aprs_messages_p20251115_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_p20251116 aprs_messages_p20251116_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_p20251116
    ADD CONSTRAINT aprs_messages_p20251116_pkey PRIMARY KEY (id, received_at);


--
-- Name: aprs_messages_old aprs_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_old
    ADD CONSTRAINT aprs_messages_pkey PRIMARY KEY (id);


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
-- Name: devices devices_address_type_address_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_address_type_address_unique UNIQUE (address_type, address);


--
-- Name: devices devices_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_pkey PRIMARY KEY (id);


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
-- Name: fixes_p20251109 fixes_p20251109_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251109
    ADD CONSTRAINT fixes_p20251109_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251110 fixes_p20251110_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251110
    ADD CONSTRAINT fixes_p20251110_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251111 fixes_p20251111_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251111
    ADD CONSTRAINT fixes_p20251111_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251112 fixes_p20251112_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251112
    ADD CONSTRAINT fixes_p20251112_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251113 fixes_p20251113_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251113
    ADD CONSTRAINT fixes_p20251113_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251114 fixes_p20251114_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251114
    ADD CONSTRAINT fixes_p20251114_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251115 fixes_p20251115_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251115
    ADD CONSTRAINT fixes_p20251115_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_p20251116 fixes_p20251116_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_p20251116
    ADD CONSTRAINT fixes_p20251116_pkey PRIMARY KEY (id, received_at);


--
-- Name: fixes_old fixes_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_pkey PRIMARY KEY (id);


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
-- Name: idx_fixes_device_received_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_device_received_at ON ONLY public.fixes USING btree (device_id, received_at DESC);


--
-- Name: fixes_default_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_default_device_id_received_at_idx ON public.fixes_default USING btree (device_id, received_at DESC);


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
-- Name: fixes_device_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_device_received_at_idx ON public.fixes_old USING btree (device_id, received_at DESC);


--
-- Name: fixes_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_location_geom_idx ON public.fixes_old USING gist (location_geom);


--
-- Name: fixes_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_location_idx ON public.fixes_old USING gist (location);


--
-- Name: fixes_p20251109_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251109_device_id_received_at_idx ON public.fixes_p20251109 USING btree (device_id, received_at DESC);


--
-- Name: fixes_p20251109_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251109_location_geom_idx ON public.fixes_p20251109 USING gist (location_geom);


--
-- Name: fixes_p20251109_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251109_location_idx ON public.fixes_p20251109 USING gist (location);


--
-- Name: fixes_p20251109_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251109_source_idx ON public.fixes_p20251109 USING btree (source);


--
-- Name: fixes_p20251110_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251110_device_id_received_at_idx ON public.fixes_p20251110 USING btree (device_id, received_at DESC);


--
-- Name: fixes_p20251110_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251110_location_geom_idx ON public.fixes_p20251110 USING gist (location_geom);


--
-- Name: fixes_p20251110_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251110_location_idx ON public.fixes_p20251110 USING gist (location);


--
-- Name: fixes_p20251110_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251110_source_idx ON public.fixes_p20251110 USING btree (source);


--
-- Name: fixes_p20251111_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251111_device_id_received_at_idx ON public.fixes_p20251111 USING btree (device_id, received_at DESC);


--
-- Name: fixes_p20251111_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251111_location_geom_idx ON public.fixes_p20251111 USING gist (location_geom);


--
-- Name: fixes_p20251111_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251111_location_idx ON public.fixes_p20251111 USING gist (location);


--
-- Name: fixes_p20251111_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251111_source_idx ON public.fixes_p20251111 USING btree (source);


--
-- Name: fixes_p20251112_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251112_device_id_received_at_idx ON public.fixes_p20251112 USING btree (device_id, received_at DESC);


--
-- Name: fixes_p20251112_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251112_location_geom_idx ON public.fixes_p20251112 USING gist (location_geom);


--
-- Name: fixes_p20251112_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251112_location_idx ON public.fixes_p20251112 USING gist (location);


--
-- Name: fixes_p20251112_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251112_source_idx ON public.fixes_p20251112 USING btree (source);


--
-- Name: fixes_p20251113_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251113_device_id_received_at_idx ON public.fixes_p20251113 USING btree (device_id, received_at DESC);


--
-- Name: fixes_p20251113_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251113_location_geom_idx ON public.fixes_p20251113 USING gist (location_geom);


--
-- Name: fixes_p20251113_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251113_location_idx ON public.fixes_p20251113 USING gist (location);


--
-- Name: fixes_p20251113_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251113_source_idx ON public.fixes_p20251113 USING btree (source);


--
-- Name: fixes_p20251114_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251114_device_id_received_at_idx ON public.fixes_p20251114 USING btree (device_id, received_at DESC);


--
-- Name: fixes_p20251114_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251114_location_geom_idx ON public.fixes_p20251114 USING gist (location_geom);


--
-- Name: fixes_p20251114_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251114_location_idx ON public.fixes_p20251114 USING gist (location);


--
-- Name: fixes_p20251114_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251114_source_idx ON public.fixes_p20251114 USING btree (source);


--
-- Name: fixes_p20251115_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251115_device_id_received_at_idx ON public.fixes_p20251115 USING btree (device_id, received_at DESC);


--
-- Name: fixes_p20251115_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251115_location_geom_idx ON public.fixes_p20251115 USING gist (location_geom);


--
-- Name: fixes_p20251115_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251115_location_idx ON public.fixes_p20251115 USING gist (location);


--
-- Name: fixes_p20251115_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251115_source_idx ON public.fixes_p20251115 USING btree (source);


--
-- Name: fixes_p20251116_device_id_received_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251116_device_id_received_at_idx ON public.fixes_p20251116 USING btree (device_id, received_at DESC);


--
-- Name: fixes_p20251116_location_geom_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251116_location_geom_idx ON public.fixes_p20251116 USING gist (location_geom);


--
-- Name: fixes_p20251116_location_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251116_location_idx ON public.fixes_p20251116 USING gist (location);


--
-- Name: fixes_p20251116_source_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fixes_p20251116_source_idx ON public.fixes_p20251116 USING btree (source);


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
-- Name: idx_aircraft_approved_operations_registration_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_approved_operations_registration_id ON public.aircraft_approved_operations USING btree (aircraft_registration_id);


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
-- Name: idx_aircraft_registrations_club_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_registrations_club_id ON public.aircraft_registrations USING btree (club_id) WHERE (club_id IS NOT NULL);


--
-- Name: idx_aircraft_registrations_device_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aircraft_registrations_device_id ON public.aircraft_registrations USING btree (device_id);


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
-- Name: idx_aprs_messages_received_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aprs_messages_received_at ON public.aprs_messages_old USING btree (received_at);


--
-- Name: idx_aprs_messages_receiver_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_aprs_messages_receiver_id ON public.aprs_messages_old USING btree (receiver_id);


--
-- Name: idx_club_pilots_name; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_club_pilots_name ON public.pilots USING btree (last_name, first_name);


--
-- Name: idx_devices_address_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_devices_address_unique ON public.devices USING btree (address);


--
-- Name: idx_devices_aircraft_model; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_aircraft_model ON public.devices USING btree (aircraft_model);


--
-- Name: idx_devices_country_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_country_code ON public.devices USING btree (country_code);


--
-- Name: idx_devices_from_ddb; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_from_ddb ON public.devices USING btree (from_ddb);


--
-- Name: idx_devices_icao_model_code; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_icao_model_code ON public.devices USING btree (icao_model_code) WHERE (icao_model_code IS NOT NULL);


--
-- Name: idx_devices_identified; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_identified ON public.devices USING btree (identified);


--
-- Name: idx_devices_last_fix_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_last_fix_at ON public.devices USING btree (last_fix_at) WHERE (last_fix_at IS NOT NULL);


--
-- Name: idx_devices_registration; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_registration ON public.devices USING btree (registration);


--
-- Name: idx_devices_tracked; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_tracked ON public.devices USING btree (tracked);


--
-- Name: idx_devices_tracker_device_type; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_devices_tracker_device_type ON public.devices USING btree (tracker_device_type) WHERE (tracker_device_type IS NOT NULL);


--
-- Name: idx_fixes_altitude_agl_feet; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_altitude_agl_feet ON public.fixes_old USING btree (altitude_agl_feet);


--
-- Name: idx_fixes_altitude_agl_valid; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_altitude_agl_valid ON public.fixes_old USING btree (altitude_agl_valid) WHERE (altitude_agl_valid = false);


--
-- Name: idx_fixes_aprs_message_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_aprs_message_id ON public.fixes_old USING btree (aprs_message_id);


--
-- Name: idx_fixes_backfill_optimized; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_backfill_optimized ON public.fixes_old USING btree ("timestamp") WHERE ((altitude_agl_valid = false) AND (altitude_msl_feet IS NOT NULL) AND (is_active = true));


--
-- Name: idx_fixes_device_id_timestamp; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_fixes_device_id_timestamp ON public.fixes_old USING btree (device_id, "timestamp");


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
-- Name: idx_flight_pilots_pilot_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flight_pilots_pilot_id ON public.flight_pilots USING btree (pilot_id);


--
-- Name: idx_flights_bounding_box; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_bounding_box ON public.flights USING btree (min_latitude, max_latitude, min_longitude, max_longitude) WHERE (min_latitude IS NOT NULL);


--
-- Name: idx_flights_callsign; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_callsign ON public.flights USING btree (callsign);


--
-- Name: idx_flights_device_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_device_id ON public.flights USING btree (device_id);


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
-- Name: idx_flights_towed_by_device; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_flights_towed_by_device ON public.flights USING btree (towed_by_device_id) WHERE (towed_by_device_id IS NOT NULL);


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
-- Name: idx_receiver_statuses_aprs_message_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_receiver_statuses_aprs_message_id ON public.receiver_statuses USING btree (aprs_message_id);


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

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_default_pkey;


--
-- Name: aprs_messages_p20251108_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251108_pkey;


--
-- Name: aprs_messages_p20251109_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251109_pkey;


--
-- Name: aprs_messages_p20251110_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251110_pkey;


--
-- Name: aprs_messages_p20251111_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251111_pkey;


--
-- Name: aprs_messages_p20251112_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251112_pkey;


--
-- Name: aprs_messages_p20251113_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251113_pkey;


--
-- Name: aprs_messages_p20251114_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251114_pkey;


--
-- Name: aprs_messages_p20251115_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251115_pkey;


--
-- Name: aprs_messages_p20251116_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.aprs_messages_pkey1 ATTACH PARTITION public.aprs_messages_p20251116_pkey;


--
-- Name: fixes_default_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_default_device_id_received_at_idx;


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
-- Name: fixes_p20251109_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_p20251109_device_id_received_at_idx;


--
-- Name: fixes_p20251109_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251109_location_geom_idx;


--
-- Name: fixes_p20251109_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251109_location_idx;


--
-- Name: fixes_p20251109_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251109_pkey;


--
-- Name: fixes_p20251109_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251109_source_idx;


--
-- Name: fixes_p20251110_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_p20251110_device_id_received_at_idx;


--
-- Name: fixes_p20251110_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251110_location_geom_idx;


--
-- Name: fixes_p20251110_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251110_location_idx;


--
-- Name: fixes_p20251110_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251110_pkey;


--
-- Name: fixes_p20251110_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251110_source_idx;


--
-- Name: fixes_p20251111_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_p20251111_device_id_received_at_idx;


--
-- Name: fixes_p20251111_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251111_location_geom_idx;


--
-- Name: fixes_p20251111_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251111_location_idx;


--
-- Name: fixes_p20251111_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251111_pkey;


--
-- Name: fixes_p20251111_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251111_source_idx;


--
-- Name: fixes_p20251112_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_p20251112_device_id_received_at_idx;


--
-- Name: fixes_p20251112_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251112_location_geom_idx;


--
-- Name: fixes_p20251112_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251112_location_idx;


--
-- Name: fixes_p20251112_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251112_pkey;


--
-- Name: fixes_p20251112_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251112_source_idx;


--
-- Name: fixes_p20251113_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_p20251113_device_id_received_at_idx;


--
-- Name: fixes_p20251113_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251113_location_geom_idx;


--
-- Name: fixes_p20251113_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251113_location_idx;


--
-- Name: fixes_p20251113_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251113_pkey;


--
-- Name: fixes_p20251113_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251113_source_idx;


--
-- Name: fixes_p20251114_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_p20251114_device_id_received_at_idx;


--
-- Name: fixes_p20251114_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251114_location_geom_idx;


--
-- Name: fixes_p20251114_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251114_location_idx;


--
-- Name: fixes_p20251114_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251114_pkey;


--
-- Name: fixes_p20251114_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251114_source_idx;


--
-- Name: fixes_p20251115_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_p20251115_device_id_received_at_idx;


--
-- Name: fixes_p20251115_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251115_location_geom_idx;


--
-- Name: fixes_p20251115_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251115_location_idx;


--
-- Name: fixes_p20251115_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251115_pkey;


--
-- Name: fixes_p20251115_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251115_source_idx;


--
-- Name: fixes_p20251116_device_id_received_at_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_device_received_at ATTACH PARTITION public.fixes_p20251116_device_id_received_at_idx;


--
-- Name: fixes_p20251116_location_geom_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location_geom ATTACH PARTITION public.fixes_p20251116_location_geom_idx;


--
-- Name: fixes_p20251116_location_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_location ATTACH PARTITION public.fixes_p20251116_location_idx;


--
-- Name: fixes_p20251116_pkey; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.fixes_pkey1 ATTACH PARTITION public.fixes_p20251116_pkey;


--
-- Name: fixes_p20251116_source_idx; Type: INDEX ATTACH; Schema: public; Owner: -
--

ALTER INDEX public.idx_fixes_source ATTACH PARTITION public.fixes_p20251116_source_idx;


--
-- Name: aprs_messages_old ensure_aprs_message_hash; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER ensure_aprs_message_hash BEFORE INSERT ON public.aprs_messages_old FOR EACH ROW EXECUTE FUNCTION public.compute_aprs_message_hash();


--
-- Name: pilots set_club_pilots_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER set_club_pilots_updated_at BEFORE UPDATE ON public.pilots FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: aircraft_models update_aircraft_model_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_aircraft_model_updated_at BEFORE UPDATE ON public.aircraft_models FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: airports update_airport_location_trigger; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_airport_location_trigger BEFORE INSERT OR UPDATE OF latitude_deg, longitude_deg ON public.airports FOR EACH ROW EXECUTE FUNCTION public.update_airport_location();


--
-- Name: airports update_airports_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_airports_updated_at BEFORE UPDATE ON public.airports FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: devices update_devices_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER update_devices_updated_at BEFORE UPDATE ON public.devices FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


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
-- Name: aircraft_other_names aircraft_other_names_registration_number_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_other_names
    ADD CONSTRAINT aircraft_other_names_registration_number_fkey FOREIGN KEY (registration_number) REFERENCES public.aircraft_registrations(registration_number) ON DELETE CASCADE;


--
-- Name: aircraft_registrations aircraft_registrations_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id) ON DELETE SET NULL;


--
-- Name: aircraft_registrations aircraft_registrations_device_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.devices(id) ON DELETE SET NULL;


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
-- Name: aprs_messages_old aprs_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aprs_messages_old
    ADD CONSTRAINT aprs_messages_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: aprs_messages aprs_messages_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.aprs_messages
    ADD CONSTRAINT aprs_messages_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


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
-- Name: devices devices_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);


--
-- Name: fixes_old fixes_aprs_message_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_aprs_message_id_fkey FOREIGN KEY (aprs_message_id) REFERENCES public.aprs_messages_old(id) ON DELETE SET NULL;


--
-- Name: fixes fixes_aprs_message_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.fixes
    ADD CONSTRAINT fixes_aprs_message_id_fkey FOREIGN KEY (aprs_message_id) REFERENCES public.aprs_messages_old(id) ON DELETE SET NULL;


--
-- Name: fixes_old fixes_device_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.devices(id) ON DELETE SET NULL;


--
-- Name: fixes fixes_device_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.fixes
    ADD CONSTRAINT fixes_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.devices(id) ON DELETE SET NULL;


--
-- Name: fixes_old fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE SET NULL;


--
-- Name: fixes fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.fixes
    ADD CONSTRAINT fixes_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES public.flights(id) ON DELETE SET NULL;


--
-- Name: fixes_old fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.fixes_old
    ADD CONSTRAINT fixes_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE SET NULL;


--
-- Name: fixes fixes_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE public.fixes
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
-- Name: flights flights_device_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.devices(id) ON DELETE SET NULL;


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
-- Name: flights flights_towed_by_device_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_towed_by_device_id_fkey FOREIGN KEY (towed_by_device_id) REFERENCES public.devices(id);


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
-- Name: receiver_statuses receiver_statuses_aprs_message_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.receiver_statuses
    ADD CONSTRAINT receiver_statuses_aprs_message_id_fkey FOREIGN KEY (aprs_message_id) REFERENCES public.aprs_messages_old(id) ON DELETE SET NULL;


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
-- Name: users users_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id) ON DELETE SET NULL;


--
-- PostgreSQL database dump complete
--

\unrestrict mKjdUTYIfcMKob3RN2hcQ8kMctDm9SqdZJ7RsbSWA3px10N5Zt0QwZgzXTDBckV
