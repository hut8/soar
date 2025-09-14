CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public;
COMMENT ON EXTENSION pg_trgm IS 'text similarity measurement and index searching based on trigrams';
CREATE EXTENSION IF NOT EXISTS postgis WITH SCHEMA public;
COMMENT ON EXTENSION postgis IS 'PostGIS geometry and geography spatial types and functions';
CREATE TYPE public.access_level AS ENUM (
    'standard',
    'admin'
);
CREATE TYPE public.address_type AS ENUM (
    'Unknown',
    'Icao',
    'Flarm',
    'OgnTracker'
);
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
CREATE TYPE public.device_type_enum AS ENUM (
    'flarm',
    'ogn',
    'icao',
    'unknown'
);
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
CREATE FUNCTION public.update_updated_at_column() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;
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
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
CREATE TABLE public.aircraft_other_names (
    registration_number character varying(5) NOT NULL,
    seq smallint NOT NULL,
    other_name character varying(50) NOT NULL,
    CONSTRAINT aircraft_other_names_seq_check CHECK (((seq >= 1) AND (seq <= 5)))
);
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
    op_restricted_other boolean NOT NULL DEFAULT false,
    op_restricted_ag_pest_control boolean NOT NULL DEFAULT false,
    op_restricted_aerial_surveying boolean NOT NULL DEFAULT false,
    op_restricted_aerial_advertising boolean NOT NULL DEFAULT false,
    op_restricted_forest boolean NOT NULL DEFAULT false,
    op_restricted_patrolling boolean NOT NULL DEFAULT false,
    op_restricted_weather_control boolean NOT NULL DEFAULT false,
    op_restricted_carriage_of_cargo boolean NOT NULL DEFAULT false,
    op_experimental_show_compliance boolean NOT NULL DEFAULT false,
    op_experimental_research_development boolean NOT NULL DEFAULT false,
    op_experimental_amateur_built boolean NOT NULL DEFAULT false,
    op_experimental_exhibition boolean NOT NULL DEFAULT false,
    op_experimental_racing boolean NOT NULL DEFAULT false,
    op_experimental_crew_training boolean NOT NULL DEFAULT false,
    op_experimental_market_survey boolean NOT NULL DEFAULT false,
    op_experimental_operating_kit_built boolean NOT NULL DEFAULT false,
    op_experimental_light_sport_reg_prior_2008 boolean NOT NULL DEFAULT false,
    op_experimental_light_sport_operating_kit_built boolean NOT NULL DEFAULT false,
    op_experimental_light_sport_prev_21_190 boolean NOT NULL DEFAULT false,
    op_experimental_uas_research_development boolean NOT NULL DEFAULT false,
    op_experimental_uas_market_survey boolean NOT NULL DEFAULT false,
    op_experimental_uas_crew_training boolean NOT NULL DEFAULT false,
    op_experimental_uas_exhibition boolean NOT NULL DEFAULT false,
    op_experimental_uas_compliance_with_cfr boolean NOT NULL DEFAULT false,
    op_sfp_ferry_for_repairs_alterations_storage boolean NOT NULL DEFAULT false,
    op_sfp_evacuate_impending_danger boolean NOT NULL DEFAULT false,
    op_sfp_excess_of_max_certificated boolean NOT NULL DEFAULT false,
    op_sfp_delivery_or_export boolean NOT NULL DEFAULT false,
    op_sfp_production_flight_testing boolean NOT NULL DEFAULT false,
    op_sfp_customer_demo boolean NOT NULL DEFAULT false,
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
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
CREATE TABLE public.clubs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name character varying(255) NOT NULL,
    is_soaring boolean DEFAULT false,
    home_base_airport_id integer,
    location_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
CREATE TABLE public.countries (
    code character(2) NOT NULL,
    name text NOT NULL
);
CREATE TABLE public.devices (
    device_id integer NOT NULL,
    device_type public.device_type_enum NOT NULL,
    aircraft_model text NOT NULL,
    registration text NOT NULL,
    competition_number text NOT NULL,
    tracked boolean NOT NULL,
    identified boolean NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
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
    flight_id uuid,
    device_id integer,
    CONSTRAINT fixes_track_degrees_check CHECK (((track_degrees >= (0)::double precision) AND (track_degrees < (360)::double precision)))
);
CREATE TABLE public.flights (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    aircraft_id character varying(10) NOT NULL,
    takeoff_time timestamp with time zone NOT NULL,
    landing_time timestamp with time zone,
    departure_airport character varying(10),
    arrival_airport character varying(10),
    tow_aircraft_id character varying(5),
    tow_release_height_msl integer,
    club_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
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
CREATE TABLE public.receivers (
    id integer NOT NULL,
    callsign text NOT NULL,
    description text,
    contact text,
    email text,
    country text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
CREATE SEQUENCE public.receivers_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.receivers_id_seq OWNED BY public.receivers.id;

CREATE TABLE public.receivers_links (
    id integer NOT NULL,
    receiver_id integer NOT NULL,
    rel text,
    href text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE SEQUENCE public.receivers_links_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE public.receivers_links_id_seq OWNED BY public.receivers_links.id;

CREATE TABLE public.receivers_photos (
    id integer NOT NULL,
    receiver_id integer NOT NULL,
    photo_url text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE SEQUENCE public.receivers_photos_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;
ALTER SEQUENCE public.receivers_photos_id_seq OWNED BY public.receivers_photos.id;
CREATE TABLE public.regions (
    code character(1) NOT NULL,
    description text NOT NULL
);
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
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
CREATE TABLE public.states (
    code character(2) NOT NULL,
    name text NOT NULL
);
CREATE TABLE public.status_codes (
    code text NOT NULL,
    description text NOT NULL
);
CREATE TABLE public.type_aircraft (
    code character(1) NOT NULL,
    description text NOT NULL
);
CREATE TABLE public.type_engines (
    code smallint NOT NULL,
    description text NOT NULL
);
CREATE TABLE public.type_registrations (
    code character(1) NOT NULL,
    description text NOT NULL
);
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
    email_verification_token character varying(255),
    email_verification_expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.receivers ALTER COLUMN id SET DEFAULT nextval('public.receivers_id_seq'::regclass);
ALTER TABLE ONLY public.receivers_links ALTER COLUMN id SET DEFAULT nextval('public.receivers_links_id_seq'::regclass);
ALTER TABLE ONLY public.receivers_photos ALTER COLUMN id SET DEFAULT nextval('public.receivers_photos_id_seq'::regclass);
ALTER TABLE ONLY public.aircraft_model
    ADD CONSTRAINT aircraft_model_pkey PRIMARY KEY (manufacturer_code, model_code, series_code);
ALTER TABLE ONLY public.aircraft_other_names
    ADD CONSTRAINT aircraft_other_names_pkey PRIMARY KEY (registration_number, seq);
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_pkey PRIMARY KEY (registration_number);
ALTER TABLE ONLY public.airports
    ADD CONSTRAINT airports_ident_key UNIQUE (ident);
ALTER TABLE ONLY public.airports
    ADD CONSTRAINT airports_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.countries
    ADD CONSTRAINT countries_pkey PRIMARY KEY (code);
ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_pkey PRIMARY KEY (device_id);
ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.locations
    ADD CONSTRAINT locations_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.receivers
    ADD CONSTRAINT receivers_callsign_key UNIQUE (callsign);
ALTER TABLE ONLY public.receivers_links
    ADD CONSTRAINT receivers_links_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.receivers_photos
    ADD CONSTRAINT receivers_photos_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.receivers
    ADD CONSTRAINT receivers_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.regions
    ADD CONSTRAINT regions_pkey PRIMARY KEY (code);
ALTER TABLE ONLY public.runways
    ADD CONSTRAINT runways_pkey PRIMARY KEY (id);
ALTER TABLE ONLY public.states
    ADD CONSTRAINT states_pkey PRIMARY KEY (code);
ALTER TABLE ONLY public.status_codes
    ADD CONSTRAINT status_codes_pkey PRIMARY KEY (code);
ALTER TABLE ONLY public.type_aircraft
    ADD CONSTRAINT type_aircraft_pkey PRIMARY KEY (code);
ALTER TABLE ONLY public.type_engines
    ADD CONSTRAINT type_engines_pkey PRIMARY KEY (code);
ALTER TABLE ONLY public.type_registrations
    ADD CONSTRAINT type_registrations_pkey PRIMARY KEY (code);
ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);
ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);
CREATE INDEX aircraft_registrations_aw_class_idx ON public.aircraft_registrations USING btree (airworthiness_class, type_aircraft_code);
CREATE INDEX aircraft_registrations_club_id_idx ON public.aircraft_registrations USING btree (club_id);
CREATE INDEX aircraft_registrations_device_id_idx ON public.aircraft_registrations USING btree (device_id);
CREATE INDEX aircraft_registrations_eng_mfr_mdl_idx ON public.aircraft_registrations USING btree (eng_mfr_mdl_code);
CREATE INDEX aircraft_registrations_home_base_airport_id_idx ON public.aircraft_registrations USING btree (home_base_airport_id);
CREATE INDEX aircraft_registrations_location_id_idx ON public.aircraft_registrations USING btree (location_id);
CREATE INDEX aircraft_registrations_mfr_mdl_idx ON public.aircraft_registrations USING btree (mfr_mdl_code);
CREATE INDEX aircraft_registrations_serial_idx ON public.aircraft_registrations USING btree (serial_number);
CREATE INDEX aircraft_registrations_status_idx ON public.aircraft_registrations USING btree (status_code);
CREATE INDEX aircraft_registrations_transponder_idx ON public.aircraft_registrations USING btree (transponder_code);
CREATE INDEX airports_iata_trgm_idx ON public.airports USING gin (iata_code public.gin_trgm_ops) WHERE (iata_code IS NOT NULL);
CREATE INDEX airports_icao_trgm_idx ON public.airports USING gin (icao_code public.gin_trgm_ops) WHERE (icao_code IS NOT NULL);
CREATE INDEX airports_ident_trgm_idx ON public.airports USING gin (ident public.gin_trgm_ops);
CREATE INDEX airports_name_trgm_idx ON public.airports USING gin (name public.gin_trgm_ops);
CREATE INDEX clubs_home_base_airport_id_idx ON public.clubs USING btree (home_base_airport_id);
CREATE INDEX clubs_is_soaring_idx ON public.clubs USING btree (is_soaring);
CREATE INDEX clubs_location_id_idx ON public.clubs USING btree (location_id);
CREATE INDEX clubs_name_trgm_idx ON public.clubs USING gin (name public.gin_trgm_ops);
CREATE INDEX fixes_aircraft_id_idx ON public.fixes USING btree (aircraft_id);
CREATE INDEX fixes_aircraft_timestamp_idx ON public.fixes USING btree (aircraft_id, "timestamp" DESC);
CREATE INDEX fixes_club_id_idx ON public.fixes USING btree (club_id);
CREATE INDEX fixes_flight_id_idx ON public.fixes USING btree (flight_id);
CREATE INDEX fixes_location_idx ON public.fixes USING gist (location);
CREATE INDEX fixes_registration_idx ON public.fixes USING btree (registration);
CREATE INDEX fixes_registration_timestamp_idx ON public.fixes USING btree (registration, "timestamp" DESC);
CREATE INDEX fixes_source_idx ON public.fixes USING btree (source);
CREATE INDEX fixes_timestamp_idx ON public.fixes USING btree ("timestamp" DESC);
CREATE INDEX flights_aircraft_id_idx ON public.flights USING btree (aircraft_id);
CREATE INDEX flights_aircraft_takeoff_idx ON public.flights USING btree (aircraft_id, takeoff_time DESC);
CREATE INDEX flights_club_id_idx ON public.flights USING btree (club_id);
CREATE INDEX flights_landing_time_idx ON public.flights USING btree (landing_time DESC);
CREATE INDEX flights_takeoff_time_idx ON public.flights USING btree (takeoff_time DESC);
CREATE INDEX flights_tow_aircraft_idx ON public.flights USING btree (tow_aircraft_id);
CREATE INDEX idx_aircraft_model_aircraft_type ON public.aircraft_model USING btree (aircraft_type);
CREATE INDEX idx_aircraft_model_engine_type ON public.aircraft_model USING btree (engine_type);
CREATE INDEX idx_aircraft_model_manufacturer_code ON public.aircraft_model USING btree (manufacturer_code);
CREATE INDEX idx_aircraft_model_manufacturer_name ON public.aircraft_model USING btree (manufacturer_name);
CREATE INDEX idx_aircraft_model_model_name ON public.aircraft_model USING btree (model_name);
CREATE INDEX idx_airports_gps_code ON public.airports USING btree (gps_code) WHERE (gps_code IS NOT NULL);
CREATE INDEX idx_airports_iata_code ON public.airports USING btree (iata_code) WHERE (iata_code IS NOT NULL);
CREATE INDEX idx_airports_icao_code ON public.airports USING btree (icao_code) WHERE (icao_code IS NOT NULL);
CREATE INDEX idx_airports_ident ON public.airports USING btree (ident);
CREATE INDEX idx_airports_iso_country ON public.airports USING btree (iso_country);
CREATE INDEX idx_airports_iso_region ON public.airports USING btree (iso_region);
CREATE INDEX idx_airports_location_gist ON public.airports USING gist (location) WHERE (location IS NOT NULL);
CREATE INDEX idx_airports_municipality ON public.airports USING btree (municipality);
CREATE INDEX idx_airports_scheduled_service ON public.airports USING btree (scheduled_service) WHERE (scheduled_service = true);
CREATE INDEX idx_airports_type ON public.airports USING btree (type);
CREATE INDEX idx_devices_aircraft_model ON public.devices USING btree (aircraft_model);
CREATE INDEX idx_devices_device_type ON public.devices USING btree (device_type);
CREATE INDEX idx_devices_identified ON public.devices USING btree (identified);
CREATE INDEX idx_devices_registration ON public.devices USING btree (registration);
CREATE INDEX idx_devices_tracked ON public.devices USING btree (tracked);
CREATE INDEX idx_receivers_callsign ON public.receivers USING btree (callsign);
CREATE INDEX idx_receivers_country ON public.receivers USING btree (country);
CREATE INDEX idx_receivers_links_receiver_id ON public.receivers_links USING btree (receiver_id);
CREATE INDEX idx_receivers_photos_receiver_id ON public.receivers_photos USING btree (receiver_id);
CREATE INDEX idx_runways_airport_ident ON public.runways USING btree (airport_ident);
CREATE INDEX idx_runways_airport_ref ON public.runways USING btree (airport_ref);
CREATE INDEX idx_runways_closed ON public.runways USING btree (closed);
CREATE INDEX idx_runways_he_location_gist ON public.runways USING gist (he_location) WHERE (he_location IS NOT NULL);
CREATE INDEX idx_runways_le_location_gist ON public.runways USING gist (le_location) WHERE (le_location IS NOT NULL);
CREATE INDEX idx_runways_length ON public.runways USING btree (length_ft) WHERE (length_ft IS NOT NULL);
CREATE INDEX idx_runways_lighted ON public.runways USING btree (lighted) WHERE (lighted = true);
CREATE INDEX idx_runways_surface ON public.runways USING btree (surface);
CREATE UNIQUE INDEX locations_address_unique_idx ON public.locations USING btree (COALESCE(street1, ''::text), COALESCE(street2, ''::text), COALESCE(city, ''::text), COALESCE(state, ''::text), COALESCE(zip_code, ''::text), COALESCE(country_mail_code, 'US'::text));
CREATE INDEX locations_geolocation_idx ON public.locations USING gist (geolocation);
CREATE INDEX users_access_level_idx ON public.users USING btree (access_level);
CREATE INDEX users_club_id_idx ON public.users USING btree (club_id);
CREATE INDEX users_email_idx ON public.users USING btree (email);
CREATE INDEX users_email_verification_token_idx ON public.users USING btree (email_verification_token);
CREATE INDEX users_password_reset_token_idx ON public.users USING btree (password_reset_token);
CREATE TRIGGER update_aircraft_model_updated_at BEFORE UPDATE ON public.aircraft_model FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();
CREATE TRIGGER update_airport_location_trigger BEFORE INSERT OR UPDATE OF latitude_deg, longitude_deg ON public.airports FOR EACH ROW EXECUTE FUNCTION public.update_airport_location();
CREATE TRIGGER update_airports_updated_at BEFORE UPDATE ON public.airports FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();
CREATE TRIGGER update_devices_updated_at BEFORE UPDATE ON public.devices FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();
CREATE TRIGGER update_receivers_updated_at BEFORE UPDATE ON public.receivers FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();
CREATE TRIGGER update_runway_locations_trigger BEFORE INSERT OR UPDATE OF le_latitude_deg, le_longitude_deg, he_latitude_deg, he_longitude_deg ON public.runways FOR EACH ROW EXECUTE FUNCTION public.update_runway_locations();
CREATE TRIGGER update_runways_updated_at BEFORE UPDATE ON public.runways FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at_column();
ALTER TABLE ONLY public.aircraft_other_names
    ADD CONSTRAINT aircraft_other_names_registration_number_fkey FOREIGN KEY (registration_number) REFERENCES public.aircraft_registrations(registration_number) ON DELETE CASCADE;
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.devices(device_id);
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_home_base_airport_id_fkey FOREIGN KEY (home_base_airport_id) REFERENCES public.airports(id) ON DELETE SET NULL;
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id);
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_status_code_fkey FOREIGN KEY (status_code) REFERENCES public.status_codes(code);
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_type_aircraft_code_fkey FOREIGN KEY (type_aircraft_code) REFERENCES public.type_aircraft(code);
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_type_engine_code_fkey FOREIGN KEY (type_engine_code) REFERENCES public.type_engines(code);
ALTER TABLE ONLY public.aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_type_registration_code_fkey FOREIGN KEY (type_registration_code) REFERENCES public.type_registrations(code);
ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_home_base_airport_id_fkey FOREIGN KEY (home_base_airport_id) REFERENCES public.airports(id) ON DELETE SET NULL;
ALTER TABLE ONLY public.clubs
    ADD CONSTRAINT clubs_location_id_fkey FOREIGN KEY (location_id) REFERENCES public.locations(id);
ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);
ALTER TABLE ONLY public.fixes
    ADD CONSTRAINT fixes_flight_id_fkey FOREIGN KEY (flight_id) REFERENCES public.flights(id);
ALTER TABLE ONLY public.runways
    ADD CONSTRAINT fk_runway_airport_ident FOREIGN KEY (airport_ident) REFERENCES public.airports(ident);
ALTER TABLE ONLY public.runways
    ADD CONSTRAINT fk_runway_airport_ref FOREIGN KEY (airport_ref) REFERENCES public.airports(id);
ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id);
ALTER TABLE ONLY public.flights
    ADD CONSTRAINT flights_tow_aircraft_id_fkey FOREIGN KEY (tow_aircraft_id) REFERENCES public.aircraft_registrations(registration_number);
ALTER TABLE ONLY public.receivers_links
    ADD CONSTRAINT receivers_links_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;
ALTER TABLE ONLY public.receivers_photos
    ADD CONSTRAINT receivers_photos_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES public.receivers(id) ON DELETE CASCADE;
ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_club_id_fkey FOREIGN KEY (club_id) REFERENCES public.clubs(id) ON DELETE SET NULL;
