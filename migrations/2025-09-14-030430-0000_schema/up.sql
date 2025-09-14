--
-- PostgreSQL database dump
--

\restrict Lxg388RxdBV3iASHrlT40v8joWqfmki7OlEiaZIPukqijFnsGWNGn5DDxzBzEhl

-- Dumped from database version 17.6 (Ubuntu 17.6-1.pgdg22.04+1)
-- Dumped by pg_dump version 17.6 (Ubuntu 17.6-1.pgdg22.04+1)

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
-- Name: pg_trgm; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public;


--
-- Name: EXTENSION pg_trgm; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pg_trgm IS 'text similarity measurement and index searching based on trigrams';


--
-- Name: postgis; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS postgis WITH SCHEMA public;


--
-- Name: EXTENSION postgis; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION postgis IS 'PostGIS geometry and geography spatial types and functions';


--
-- Name: access_level; Type: TYPE; Schema: public; Owner: liam
--

CREATE TYPE public.access_level AS ENUM (
    'standard',
    'admin'
);


ALTER TYPE public.access_level OWNER TO liam;

--
-- Name: address_type; Type: TYPE; Schema: public; Owner: liam
--

CREATE TYPE public.address_type AS ENUM (
    'Unknown',
    'Icao',
    'Flarm',
    'OgnTracker'
);


ALTER TYPE public.address_type OWNER TO liam;

--
-- Name: adsb_emitter_category; Type: TYPE; Schema: public; Owner: liam
--

CREATE TYPE public.adsb_emitter_category AS ENUM (
    'A0',
    'A1',
    'A2',
    'A3',
    'A4',
    'A5',
    'A6',
    'A7',
    'B0',
    'B1',
    'B2',
    'B3',
    'B4',
    'B6',
    'B7',
    'C0',
    'C1',
    'C2',
    'C3',
    'C4',
    'C5'
);


ALTER TYPE public.adsb_emitter_category OWNER TO liam;

--
-- Name: aircraft_type; Type: TYPE; Schema: public; Owner: liam
--

CREATE TYPE public.aircraft_type AS ENUM (
    'Reserved0',
    'GliderMotorGlider',
    'TowTug',
    'HelicopterGyro',
    'SkydiverParachute',
    'DropPlane',
    'HangGlider',
    'Paraglider',
    'RecipEngine',
    'JetTurboprop',
    'Unknown',
    'Balloon',
    'Airship',
    'Uav',
    'ReservedE',
    'StaticObstacle'
);


ALTER TYPE public.aircraft_type OWNER TO liam;

--
-- Name: airworthiness_class; Type: TYPE; Schema: public; Owner: liam
--

CREATE TYPE public.airworthiness_class AS ENUM (
    'Standard',
    'Limited',
    'Restricted',
    'Experimental',
    'Provisional',
    'Multiple',
    'Primary',
    'Special Flight Permit',
    'Light Sport'
);


ALTER TYPE public.airworthiness_class OWNER TO liam;

--
-- Name: device_type_enum; Type: TYPE; Schema: public; Owner: liam
--

CREATE TYPE public.device_type_enum AS ENUM (
    'flarm',
    'ogn',
    'icao',
    'unknown'
);


ALTER TYPE public.device_type_enum OWNER TO liam;

--
-- Name: update_airport_location(); Type: FUNCTION; Schema: public; Owner: liam
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


ALTER FUNCTION public.update_airport_location() OWNER TO liam;

--
-- Name: update_runway_locations(); Type: FUNCTION; Schema: public; Owner: liam
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


ALTER FUNCTION public.update_runway_locations() OWNER TO liam;

--
-- Name: update_updated_at_column(); Type: FUNCTION; Schema: public; Owner: liam
--

CREATE FUNCTION public.update_updated_at_column() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.update_updated_at_column() OWNER TO liam;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.__diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


ALTER TABLE public.__diesel_schema_migrations OWNER TO liam;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO liam;

--
-- Name: aircraft_model; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.aircraft_model (
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
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);


ALTER TABLE public.aircraft_model OWNER TO liam;

--
-- Name: aircraft_other_names; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.aircraft_other_names (
    registration_number character varying(5) NOT NULL,
    seq smallint NOT NULL,
    other_name character varying(50),
    CONSTRAINT aircraft_other_names_seq_check CHECK (((seq >= 1) AND (seq <= 5)))
);


ALTER TABLE public.aircraft_other_names OWNER TO liam;

--
-- Name: aircraft_registrations; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.aircraft_registrations (
    registration_number character varying(6) NOT NULL,
    serial_number character varying(30) NOT NULL,
    mfr_mdl_code character varying(7),
    eng_mfr_mdl_code character varying(5),
    year_mfr integer,
    type_registration_code character(1),
    registrant_name character varying(50),
    last_action_date date,
    certificate_issue_date date,
    op_restricted_other boolean,
    op_restricted_ag_pest_control boolean,
    op_restricted_aerial_surveying boolean,
    op_restricted_aerial_advertising boolean,
    op_restricted_forest boolean,
    op_restricted_patrolling boolean,
    op_restricted_weather_control boolean,
    op_restricted_carriage_of_cargo boolean,
    op_experimental_show_compliance boolean,
    op_experimental_research_development boolean,
    op_experimental_amateur_built boolean,
    op_experimental_exhibition boolean,
    op_experimental_racing boolean,
    op_experimental_crew_training boolean,
    op_experimental_market_survey boolean,
    op_experimental_operating_kit_built boolean,
    op_experimental_light_sport_reg_prior_2008 boolean,
    op_experimental_light_sport_operating_kit_built boolean,
    op_experimental_light_sport_prev_21_190 boolean,
    op_experimental_uas_research_development boolean,
    op_experimental_uas_market_survey boolean,
    op_experimental_uas_crew_training boolean,
    op_experimental_uas_exhibition boolean,
    op_experimental_uas_compliance_with_cfr boolean,
    op_sfp_ferry_for_repairs_alterations_storage boolean,
    op_sfp_evacuate_impending_danger boolean,
    op_sfp_excess_of_max_certificated boolean,
    op_sfp_delivery_or_export boolean,
    op_sfp_production_flight_testing boolean,
    op_sfp_customer_demo boolean,
    type_aircraft_code character(1),
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
    club_id uuid,
    home_base_airport_id integer,
    is_tow_plane boolean,
    airworthiness_class public.airworthiness_class,
    location_id uuid,
    device_id integer,
    CONSTRAINT aircraft_registrations_year_mfr_check CHECK (((year_mfr >= 1800) AND (year_mfr <= 2999)))
);


ALTER TABLE public.aircraft_registrations OWNER TO liam;

--
-- Name: airports; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.airports (
    id integer NOT NULL,
    ident character varying(7) NOT NULL,
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
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);


ALTER TABLE public.airports OWNER TO liam;

--
-- Name: clubs; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.clubs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    is_soaring boolean DEFAULT false,
    home_base_airport_id integer,
    location_id uuid
);


ALTER TABLE public.clubs OWNER TO liam;

--
-- Name: countries; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.countries (
    code character(2) NOT NULL,
    name text NOT NULL
);


ALTER TABLE public.countries OWNER TO liam;

--
-- Name: devices; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.devices (
    device_id integer NOT NULL,
    aircraft_model text NOT NULL,
    registration text NOT NULL,
    competition_number text NOT NULL,
    tracked boolean NOT NULL,
    identified boolean NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    device_type public.device_type_enum NOT NULL
);


ALTER TABLE public.devices OWNER TO liam;

--
-- Name: fixes; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.fixes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source character varying(9) NOT NULL,
    destination character varying(9) NOT NULL,
    via text[],
    raw_packet text NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    location public.geography(Point,4326) GENERATED ALWAYS AS ((public.st_point(longitude, latitude))::public.geography) STORED,
    altitude_feet integer,
    aircraft_id character varying(10),
    device_type public.address_type,
    aircraft_type public.aircraft_type,
    flight_number character varying(20),
    emitter_category public.adsb_emitter_category,
    registration character varying(10),
    model character varying(50),
    squawk character varying(4),
    ground_speed_knots real,
    track_degrees real,
    climb_fpm integer,
    turn_rate_rot real,
    snr_db real,
    bit_errors_corrected integer,
    freq_offset_khz real,
    club_id uuid,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    flight_id uuid,
    device_id integer,
    CONSTRAINT fixes_track_degrees_check CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);


ALTER TABLE public.fixes OWNER TO liam;

--
-- Name: flights; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.flights (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    aircraft_id character varying(10) NOT NULL,
    takeoff_time timestamp with time zone NOT NULL,
    landing_time timestamp with time zone,
    departure_airport character varying(10),
    arrival_airport character varying(10),
    tow_aircraft_id character varying(5),
    tow_release_height_msl integer,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    club_id uuid
);


ALTER TABLE public.flights OWNER TO liam;

--
-- Name: locations; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.locations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    street1 text,
    street2 text,
    city text,
    state text,
    zip_code text,
    region_code text,
    county_mail_code text,
    country_mail_code text,
    geolocation point,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.locations OWNER TO liam;

--
-- Name: receivers; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.receivers (
    id integer NOT NULL,
    callsign text NOT NULL,
    description text,
    contact text,
    email text,
    country text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);


ALTER TABLE public.receivers OWNER TO liam;

--
-- Name: receivers_id_seq; Type: SEQUENCE; Schema: public; Owner: liam
--

CREATE SEQUENCE public.receivers_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.receivers_id_seq OWNER TO liam;

--
-- Name: receivers_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: liam
--

ALTER SEQUENCE public.receivers_id_seq OWNED BY public.receivers.id;


--
-- Name: receivers_links; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.receivers_links (
    id integer NOT NULL,
    receiver_id integer NOT NULL,
    rel text,
    href text NOT NULL,
    created_at timestamp with time zone DEFAULT now()
);


ALTER TABLE public.receivers_links OWNER TO liam;

--
-- Name: receivers_links_id_seq; Type: SEQUENCE; Schema: public; Owner: liam
--

CREATE SEQUENCE public.receivers_links_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.receivers_links_id_seq OWNER TO liam;

--
-- Name: receivers_links_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: liam
--

ALTER SEQUENCE public.receivers_links_id_seq OWNED BY public.receivers_links.id;


--
-- Name: receivers_photos; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.receivers_photos (
    id integer NOT NULL,
    receiver_id integer NOT NULL,
    photo_url text NOT NULL,
    created_at timestamp with time zone DEFAULT now()
);


ALTER TABLE public.receivers_photos OWNER TO liam;

--
-- Name: receivers_photos_id_seq; Type: SEQUENCE; Schema: public; Owner: liam
--

CREATE SEQUENCE public.receivers_photos_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.receivers_photos_id_seq OWNER TO liam;

--
-- Name: receivers_photos_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: liam
--

ALTER SEQUENCE public.receivers_photos_id_seq OWNED BY public.receivers_photos.id;


--
-- Name: regions; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.regions (
    code character(1) NOT NULL,
    description text NOT NULL
);


ALTER TABLE public.regions OWNER TO liam;

--
-- Name: runways; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.runways (
    id integer NOT NULL,
    airport_ref integer NOT NULL,
    airport_ident character varying(7) NOT NULL,
    length_ft integer,
    width_ft integer,
    surface character varying(10),
    lighted boolean DEFAULT false NOT NULL,
    closed boolean DEFAULT false NOT NULL,
    le_ident character varying(7),
    le_latitude_deg numeric(10,8),
    le_longitude_deg numeric(11,8),
    le_location public.geography(Point,4326),
    le_elevation_ft integer,
    le_heading_degt numeric(5,2),
    le_displaced_threshold_ft integer,
    he_ident character varying(7),
    he_latitude_deg numeric(10,8),
    he_longitude_deg numeric(11,8),
    he_location public.geography(Point,4326),
    he_elevation_ft integer,
    he_heading_degt numeric(5,2),
    he_displaced_threshold_ft integer,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);


ALTER TABLE public.runways OWNER TO liam;

--
-- Name: states; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.states (
    code character(2) NOT NULL,
    name text NOT NULL
);


ALTER TABLE public.states OWNER TO liam;

--
-- Name: status_codes; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.status_codes (
    code text NOT NULL,
    description text NOT NULL
);


ALTER TABLE public.status_codes OWNER TO liam;

--
-- Name: type_aircraft; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.type_aircraft (
    code character(1) NOT NULL,
    description text NOT NULL
);


ALTER TABLE public.type_aircraft OWNER TO liam;

--
-- Name: type_engines; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.type_engines (
    code smallint NOT NULL,
    description text NOT NULL
);


ALTER TABLE public.type_engines OWNER TO liam;

--
-- Name: type_registrations; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.type_registrations (
    code character(1) NOT NULL,
    description text NOT NULL
);


ALTER TABLE public.type_registrations OWNER TO liam;

--
-- Name: users; Type: TABLE; Schema: public; Owner: liam
--

CREATE TABLE public.users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    first_name character varying(255) NOT NULL,
    last_name character varying(255) NOT NULL,
    email character varying(320) NOT NULL,
    password_hash character varying(255) NOT NULL,
    access_level public.access_level DEFAULT 'standard'::public.access_level NOT NULL,
    club_id uuid,
    email_verified boolean DEFAULT false,
    password_reset_token character varying(255),
    password_reset_expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    email_verification_token character varying(255),
    email_verification_expires_at timestamp with time zone
);


ALTER TABLE public.users OWNER TO liam;

--
-- Name: receivers id; Type: DEFAULT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers ALTER COLUMN id SET DEFAULT nextval('public.receivers_id_seq'::regclass);


--
-- Name: receivers_links id; Type: DEFAULT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers_links ALTER COLUMN id SET DEFAULT nextval('public.receivers_links_id_seq'::regclass);


--
-- Name: receivers_photos id; Type: DEFAULT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers_photos ALTER COLUMN id SET DEFAULT nextval('public.receivers_photos_id_seq'::regclass);


--
-- Name: __diesel_schema_migrations __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.__diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: aircraft_model aircraft_model_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_model
    ADD CONSTRAINT aircraft_model_pkey PRIMARY KEY (manufacturer_code, model_code, series_code);


--
-- Name: aircraft_other_names aircraft_other_names_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_other_names
    ADD CONSTRAINT aircraft_other_names_pkey PRIMARY KEY (registration_number, seq);


--
-- Name: aircraft_registrations aircraft_registrations_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_pkey PRIMARY KEY (registration_number);


--
-- Name: airports airports_ident_key; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.airports
    ADD CONSTRAINT airports_ident_key UNIQUE (ident);


--
-- Name: airports airports_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.airports
    ADD CONSTRAINT airports_pkey PRIMARY KEY (id);


--
-- Name: clubs clubs_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_pkey PRIMARY KEY (id);


--
-- Name: countries countries_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.countries
    ADD CONSTRAINT countries_pkey PRIMARY KEY (code);


--
-- Name: devices devices_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_pkey PRIMARY KEY (device_id);


--
-- Name: fixes fixes_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_pkey PRIMARY KEY (id);


--
-- Name: flights flights_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_pkey PRIMARY KEY (id);


--
-- Name: locations locations_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.locations
    ADD CONSTRAINT locations_pkey PRIMARY KEY (id);


--
-- Name: receivers receivers_callsign_key; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers
    ADD CONSTRAINT receivers_callsign_key UNIQUE (callsign);


--
-- Name: receivers_links receivers_links_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers_links
    ADD CONSTRAINT receivers_links_pkey PRIMARY KEY (id);


--
-- Name: receivers_photos receivers_photos_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers_photos
    ADD CONSTRAINT receivers_photos_pkey PRIMARY KEY (id);


--
-- Name: receivers receivers_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers
    ADD CONSTRAINT receivers_pkey PRIMARY KEY (id);


--
-- Name: regions regions_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.regions
    ADD CONSTRAINT regions_pkey PRIMARY KEY (code);


--
-- Name: runways runways_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.runways
    ADD CONSTRAINT runways_pkey PRIMARY KEY (id);


--
-- Name: states states_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.states
    ADD CONSTRAINT states_pkey PRIMARY KEY (code);


--
-- Name: status_codes status_codes_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.status_codes
    ADD CONSTRAINT status_codes_pkey PRIMARY KEY (code);


--
-- Name: type_aircraft type_aircraft_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.type_aircraft
    ADD CONSTRAINT type_aircraft_pkey PRIMARY KEY (code);


--
-- Name: type_engines type_engines_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.type_engines
    ADD CONSTRAINT type_engines_pkey PRIMARY KEY (code);


--
-- Name: type_registrations type_registrations_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.type_registrations
    ADD CONSTRAINT type_registrations_pkey PRIMARY KEY (code);


--
-- Name: users users_email_key; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: aircraft_registrations_aw_class_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_aw_class_idx ON public.aircraft_registrations USING btree (airworthiness_class, type_aircraft_code);


--
-- Name: aircraft_registrations_club_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_club_id_idx ON public.aircraft_registrations USING btree (club_id);


--
-- Name: aircraft_registrations_device_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_device_id_idx ON public.aircraft_registrations USING btree (device_id);


--
-- Name: aircraft_registrations_eng_mfr_mdl_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_eng_mfr_mdl_idx ON public.aircraft_registrations USING btree (eng_mfr_mdl_code);


--
-- Name: aircraft_registrations_home_base_airport_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_home_base_airport_id_idx ON public.aircraft_registrations USING btree (home_base_airport_id);


--
-- Name: aircraft_registrations_location_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_location_id_idx ON public.aircraft_registrations USING btree (location_id);


--
-- Name: aircraft_registrations_mfr_mdl_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_mfr_mdl_idx ON public.aircraft_registrations USING btree (mfr_mdl_code);


--
-- Name: aircraft_registrations_serial_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_serial_idx ON public.aircraft_registrations USING btree (serial_number);


--
-- Name: aircraft_registrations_status_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_status_idx ON public.aircraft_registrations USING btree (status_code);


--
-- Name: aircraft_registrations_transponder_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX aircraft_registrations_transponder_idx ON public.aircraft_registrations USING btree (transponder_code);


--
-- Name: airports_iata_trgm_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX airports_iata_trgm_idx ON public.airports USING gin (iata_code public.gin_trgm_ops) WHERE (iata_code IS NOT NULL);


--
-- Name: airports_icao_trgm_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX airports_icao_trgm_idx ON public.airports USING gin (icao_code public.gin_trgm_ops) WHERE (icao_code IS NOT NULL);


--
-- Name: airports_ident_trgm_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX airports_ident_trgm_idx ON public.airports USING gin (ident public.gin_trgm_ops);


--
-- Name: airports_name_trgm_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX airports_name_trgm_idx ON public.airports USING gin (name public.gin_trgm_ops);


--
-- Name: clubs_home_base_airport_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX clubs_home_base_airport_id_idx ON public.clubs USING btree (home_base_airport_id);


--
-- Name: clubs_is_soaring_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX clubs_is_soaring_idx ON public.clubs USING btree (is_soaring);


--
-- Name: clubs_location_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX clubs_location_id_idx ON public.clubs USING btree (location_id);


--
-- Name: clubs_name_trgm_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX clubs_name_trgm_idx ON public.clubs USING gin (name public.gin_trgm_ops);


--
-- Name: fixes_aircraft_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_aircraft_id_idx ON public.fixes USING btree (aircraft_id);


--
-- Name: fixes_aircraft_timestamp_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_aircraft_timestamp_idx ON public.fixes USING btree (aircraft_id, "timestamp" DESC);


--
-- Name: fixes_club_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_club_id_idx ON public.fixes USING btree (club_id);


--
-- Name: fixes_flight_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_flight_id_idx ON public.fixes USING btree (flight_id);


--
-- Name: fixes_location_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_location_idx ON public.fixes USING gist (location);


--
-- Name: fixes_registration_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_registration_idx ON public.fixes USING btree (registration);


--
-- Name: fixes_registration_timestamp_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_registration_timestamp_idx ON public.fixes USING btree (registration, "timestamp" DESC);


--
-- Name: fixes_source_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_source_idx ON public.fixes USING btree (source);


--
-- Name: fixes_timestamp_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX fixes_timestamp_idx ON public.fixes USING btree ("timestamp" DESC);


--
-- Name: flights_aircraft_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX flights_aircraft_id_idx ON public.flights USING btree (aircraft_id);


--
-- Name: flights_aircraft_takeoff_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX flights_aircraft_takeoff_idx ON public.flights USING btree (aircraft_id, takeoff_time DESC);


--
-- Name: flights_club_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX flights_club_id_idx ON public.flights USING btree (club_id);


--
-- Name: flights_landing_time_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX flights_landing_time_idx ON public.flights USING btree (landing_time DESC);


--
-- Name: flights_takeoff_time_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX flights_takeoff_time_idx ON public.flights USING btree (takeoff_time DESC);


--
-- Name: flights_tow_aircraft_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX flights_tow_aircraft_idx ON public.flights USING btree (tow_aircraft_id);


--
-- Name: idx_aircraft_model_aircraft_type; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_aircraft_model_aircraft_type ON public.aircraft_model USING btree (aircraft_type);


--
-- Name: idx_aircraft_model_engine_type; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_aircraft_model_engine_type ON public.aircraft_model USING btree (engine_type);


--
-- Name: idx_aircraft_model_manufacturer_code; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_aircraft_model_manufacturer_code ON public.aircraft_model USING btree (manufacturer_code);


--
-- Name: idx_aircraft_model_manufacturer_name; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_aircraft_model_manufacturer_name ON public.aircraft_model USING btree (manufacturer_name);


--
-- Name: idx_aircraft_model_model_name; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_aircraft_model_model_name ON public.aircraft_model USING btree (model_name);


--
-- Name: idx_airports_gps_code; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_gps_code ON public.airports USING btree (gps_code) WHERE (gps_code IS NOT NULL);


--
-- Name: idx_airports_iata_code; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_iata_code ON public.airports USING btree (iata_code) WHERE (iata_code IS NOT NULL);


--
-- Name: idx_airports_icao_code; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_icao_code ON public.airports USING btree (icao_code) WHERE (icao_code IS NOT NULL);


--
-- Name: idx_airports_ident; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_ident ON public.airports USING btree (ident);


--
-- Name: idx_airports_iso_country; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_iso_country ON public.airports USING btree (iso_country);


--
-- Name: idx_airports_iso_region; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_iso_region ON public.airports USING btree (iso_region);


--
-- Name: idx_airports_location_gist; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_location_gist ON public.airports USING gist (location) WHERE (location IS NOT NULL);


--
-- Name: idx_airports_municipality; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_municipality ON public.airports USING btree (municipality);


--
-- Name: idx_airports_scheduled_service; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_scheduled_service ON public.airports USING btree (scheduled_service) WHERE (scheduled_service = true);


--
-- Name: idx_airports_type; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_airports_type ON public.airports USING btree (type);


--
-- Name: idx_devices_aircraft_model; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_devices_aircraft_model ON public.devices USING btree (aircraft_model);


--
-- Name: idx_devices_device_type; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_devices_device_type ON public.devices USING btree (device_type);


--
-- Name: idx_devices_identified; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_devices_identified ON public.devices USING btree (identified);


--
-- Name: idx_devices_registration; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_devices_registration ON public.devices USING btree (registration);


--
-- Name: idx_devices_tracked; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_devices_tracked ON public.devices USING btree (tracked);


--
-- Name: idx_receivers_callsign; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_receivers_callsign ON public.receivers USING btree (callsign);


--
-- Name: idx_receivers_country; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_receivers_country ON public.receivers USING btree (country);


--
-- Name: idx_receivers_links_receiver_id; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_receivers_links_receiver_id ON public.receivers_links USING btree (receiver_id);


--
-- Name: idx_receivers_photos_receiver_id; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_receivers_photos_receiver_id ON public.receivers_photos USING btree (receiver_id);


--
-- Name: idx_runways_airport_ident; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_runways_airport_ident ON public.runways USING btree (airport_ident);


--
-- Name: idx_runways_airport_ref; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_runways_airport_ref ON public.runways USING btree (airport_ref);


--
-- Name: idx_runways_closed; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_runways_closed ON public.runways USING btree (closed);


--
-- Name: idx_runways_he_location_gist; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_runways_he_location_gist ON public.runways USING gist (he_location) WHERE (he_location IS NOT NULL);


--
-- Name: idx_runways_le_location_gist; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_runways_le_location_gist ON public.runways USING gist (le_location) WHERE (le_location IS NOT NULL);


--
-- Name: idx_runways_length; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_runways_length ON public.runways USING btree (length_ft) WHERE (length_ft IS NOT NULL);


--
-- Name: idx_runways_lighted; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_runways_lighted ON public.runways USING btree (lighted) WHERE (lighted = true);


--
-- Name: idx_runways_surface; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX idx_runways_surface ON public.runways USING btree (surface);


--
-- Name: locations_address_unique_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE UNIQUE INDEX locations_address_unique_idx ON public.locations USING btree (COALESCE(street1, ''::text), COALESCE(street2, ''::text), COALESCE(city, ''::text), COALESCE(state, ''::text), COALESCE(zip_code, ''::text), COALESCE(country_mail_code, 'US'::text));


--
-- Name: locations_geolocation_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX locations_geolocation_idx ON public.locations USING gist (geolocation);


--
-- Name: users_access_level_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX users_access_level_idx ON public.users USING btree (access_level);


--
-- Name: users_club_id_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX users_club_id_idx ON public.users USING btree (club_id);


--
-- Name: users_email_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX users_email_idx ON public.users USING btree (email);


--
-- Name: users_email_verification_token_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX users_email_verification_token_idx ON public.users USING btree (email_verification_token);


--
-- Name: users_password_reset_token_idx; Type: INDEX; Schema: public; Owner: liam
--

CREATE INDEX users_password_reset_token_idx ON public.users USING btree (password_reset_token);


--
-- Name: aircraft_model update_aircraft_model_updated_at; Type: TRIGGER; Schema: public; Owner: liam
--

CREATE TRIGGER update_aircraft_model_updated_at BEFORE UPDATE ON public.aircraft_model FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: airports update_airport_location_trigger; Type: TRIGGER; Schema: public; Owner: liam
--

CREATE TRIGGER update_airport_location_trigger BEFORE INSERT OR UPDATE OF latitude_deg, longitude_deg ON public.airports FOR EACH ROW EXECUTE FUNCTION public.update_airport_location();


--
-- Name: airports update_airports_updated_at; Type: TRIGGER; Schema: public; Owner: liam
--

CREATE TRIGGER update_airports_updated_at BEFORE UPDATE ON public.airports FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: devices update_devices_updated_at; Type: TRIGGER; Schema: public; Owner: liam
--

CREATE TRIGGER update_devices_updated_at BEFORE UPDATE ON public.devices FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: receivers update_receivers_updated_at; Type: TRIGGER; Schema: public; Owner: liam
--

CREATE TRIGGER update_receivers_updated_at BEFORE UPDATE ON public.receivers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: runways update_runway_locations_trigger; Type: TRIGGER; Schema: public; Owner: liam
--

CREATE TRIGGER update_runway_locations_trigger BEFORE INSERT OR UPDATE OF le_latitude_deg, le_longitude_deg, he_latitude_deg, he_longitude_deg ON public.runways FOR EACH ROW EXECUTE FUNCTION public.update_runway_locations();


--
-- Name: runways update_runways_updated_at; Type: TRIGGER; Schema: public; Owner: liam
--

CREATE TRIGGER update_runways_updated_at BEFORE UPDATE ON public.runways FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: users update_users_updated_at; Type: TRIGGER; Schema: public; Owner: liam
--

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();


--
-- Name: aircraft_other_names aircraft_other_names_registration_number_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_other_names
    ADD CONSTRAINT aircraft_other_names_registration_number_fkey FOREIGN KEY (registration_number) REFERENCES public.aircraft_registrations(registration_number) ON DELETE CASCADE;


--
-- Name: aircraft_registrations aircraft_registrations_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);


--
-- Name: aircraft_registrations aircraft_registrations_device_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.devices(device_id);


--
-- Name: aircraft_registrations aircraft_registrations_home_base_airport_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_home_base_airport_id_fkey FOREIGN KEY (home_base_airport_id) REFERENCES public.airports(id) ON DELETE SET NULL;


--
-- Name: aircraft_registrations aircraft_registrations_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id);


--
-- Name: aircraft_registrations aircraft_registrations_status_code_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_status_code_fkey FOREIGN KEY (status_code) REFERENCES public.status_codes(code);


--
-- Name: aircraft_registrations aircraft_registrations_type_aircraft_code_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_type_aircraft_code_fkey FOREIGN KEY (type_aircraft_code) REFERENCES public.type_aircraft(code);


--
-- Name: aircraft_registrations aircraft_registrations_type_engine_code_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_type_engine_code_fkey FOREIGN KEY (type_engine_code) REFERENCES public.type_engines(code);


--
-- Name: aircraft_registrations aircraft_registrations_type_registration_code_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_type_registration_code_fkey FOREIGN KEY (type_registration_code) REFERENCES public.type_registrations(code);


--
-- Name: clubs clubs_home_base_airport_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_home_base_airport_id_fkey FOREIGN KEY (home_base_airport_id) REFERENCES public.airports(id) ON DELETE SET NULL;


--
-- Name: clubs clubs_location_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id);


--
-- Name: fixes fixes_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);


--
-- Name: fixes fixes_flight_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES public.flights(id);


--
-- Name: runways fk_runway_airport_ident; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.runways
    ADD CONSTRAINT fk_runway_airport_ident FOREIGN KEY (airport_ident) REFERENCES public.airports(ident);


--
-- Name: runways fk_runway_airport_ref; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.runways
    ADD CONSTRAINT fk_runway_airport_ref FOREIGN KEY (airport_ref) REFERENCES public.airports(id);


--
-- Name: flights flights_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);


--
-- Name: flights flights_tow_aircraft_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_tow_aircraft_id_fkey FOREIGN KEY (tow_aircraft_id) REFERENCES public.aircraft_registrations(registration_number);


--
-- Name: receivers_links receivers_links_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers_links
    ADD CONSTRAINT receivers_links_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: receivers_photos receivers_photos_receiver_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.receivers_photos
    ADD CONSTRAINT receivers_photos_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;


--
-- Name: users users_club_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: liam
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id) ON DELETE SET NULL;


--
-- PostgreSQL database dump complete
--

\unrestrict Lxg388RxdBV3iASHrlT40v8joWqfmki7OlEiaZIPukqijFnsGWNGn5DDxzBzEhl

