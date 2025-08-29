-- Add up migration script here
-- =========================================================
-- Lookup tables (created first to satisfy foreign key constraints)
-- =========================================================
CREATE TABLE type_registrations (
  code CHAR(1) PRIMARY KEY,
  description TEXT NOT NULL
);

CREATE TABLE airworthiness_classes (
  code CHAR(1) PRIMARY KEY,
  description TEXT NOT NULL
);

CREATE TABLE type_aircraft (
  code CHAR(1) PRIMARY KEY,
  description TEXT NOT NULL
);

CREATE TABLE type_engines (
  code VARCHAR(2) PRIMARY KEY,
  description TEXT NOT NULL
);

CREATE TABLE status_codes (
  code TEXT PRIMARY KEY,
  description TEXT NOT NULL
);

CREATE TABLE regions (
  code CHAR(1) PRIMARY KEY,
  description TEXT NOT NULL
);

-- These two aren't enumerated in the first 8 pages; keep structure for your own seed lists.
CREATE TABLE countries (
  code CHAR(2) PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE TABLE states (
  code CHAR(2) PRIMARY KEY,
  name TEXT NOT NULL
);

-- =========================================================
-- Core table: one row per aircraft (N-number)
-- (First 8 pages only; stops before "AIRCRAFT REFERENCE FILE")
-- =========================================================
CREATE TABLE aircraft_registrations (
  registration_number                 VARCHAR(5)  PRIMARY KEY,
  serial_number            VARCHAR(30) NOT NULL,
  mfr_mdl_code             VARCHAR(7),     -- manufacturer/model code (ref table is after p.8, so keep as free text)
  eng_mfr_mdl_code         VARCHAR(5),     -- engine mfr/model code
  year_mfr                 INTEGER CHECK (year_mfr BETWEEN 1800 AND 2999),

  -- Registrant / address
  type_registration_code   CHAR(1) REFERENCES type_registrations(code),
  registrant_name          VARCHAR(50),
  street1                  VARCHAR(33),
  street2                  VARCHAR(33),
  city                     VARCHAR(18),
  state                    CHAR(2) REFERENCES states(code),
  zip_code                 VARCHAR(10),
  region_code              CHAR(1) REFERENCES regions(code),
  county_mail_code         VARCHAR(3),
  country_mail_code        CHAR(2) REFERENCES countries(code),

  -- Dates
  last_action_date         DATE,
  certificate_issue_date   DATE,

  -- Airworthiness & approved operations
  airworthiness_class_code CHAR(1) REFERENCES airworthiness_classes(code),

  -- Approved Operation booleans (derived from positions 239–247; first 8 pages)
  -- Restricted (0–7)
  op_restricted_other                       BOOLEAN,
  op_restricted_ag_pest_control             BOOLEAN,
  op_restricted_aerial_surveying            BOOLEAN,
  op_restricted_aerial_advertising          BOOLEAN,
  op_restricted_forest                      BOOLEAN,
  op_restricted_patrolling                  BOOLEAN,
  op_restricted_weather_control             BOOLEAN,
  op_restricted_carriage_of_cargo           BOOLEAN,

  -- Experimental (0–9 with subcodes shown in doc)
  op_experimental_show_compliance           BOOLEAN,
  op_experimental_research_development      BOOLEAN,
  op_experimental_amateur_built             BOOLEAN,
  op_experimental_exhibition                BOOLEAN,
  op_experimental_racing                    BOOLEAN,
  op_experimental_crew_training             BOOLEAN,
  op_experimental_market_survey             BOOLEAN,
  op_experimental_operating_kit_built       BOOLEAN,
  op_experimental_light_sport_reg_prior_2008        BOOLEAN,  -- 8A
  op_experimental_light_sport_operating_kit_built   BOOLEAN,  -- 8B
  op_experimental_light_sport_prev_21_190           BOOLEAN,  -- 8C
  op_experimental_uas_research_development          BOOLEAN,  -- 9A
  op_experimental_uas_market_survey                 BOOLEAN,  -- 9B
  op_experimental_uas_crew_training                 BOOLEAN,  -- 9C
  op_experimental_uas_exhibition                    BOOLEAN,  -- 9D
  op_experimental_uas_compliance_with_cfr           BOOLEAN,  -- 9E

  -- Special Flight Permit (1–6)
  op_sfp_ferry_for_repairs_alterations_storage      BOOLEAN,
  op_sfp_evacuate_impending_danger                  BOOLEAN,
  op_sfp_excess_of_max_certificated                 BOOLEAN,
  op_sfp_delivery_or_export                         BOOLEAN,
  op_sfp_production_flight_testing                  BOOLEAN,
  op_sfp_customer_demo                              BOOLEAN,

  -- NOTE: Light Sport uses a letter in pos 239 for A/G/L/P/W (aircraft category),
  -- but that's already represented by type_aircraft_code at pos 249, so no extra flags here.

  type_aircraft_code        CHAR(1) REFERENCES type_aircraft(code),
  type_engine_code          VARCHAR(2) REFERENCES type_engines(code),
  status_code               TEXT REFERENCES status_codes(code), -- letters A..Z and numerics 1..29 appear

  -- Transponder / Mode S (store once, numerically)
  transponder_code          BIGINT,   -- 24-bit Mode S can fit in BIGINT; no need for separate hex/octal

  fractional_owner          BOOLEAN,
  airworthiness_date        DATE,

  -- Registration expiration
  expiration_date           DATE,

  -- FAA unique ID
  unique_id                 CHAR(8),

  -- Amateur/kit
  kit_mfr_name              VARCHAR(30),
  kit_model_name            VARCHAR(20),

  -- Optional: raw ops string for ETL/audit
  approved_operations_raw   VARCHAR(9)
);

-- =========================================================
-- Multi-value "Other Name" slots (normalize to rows 1..5)
-- =========================================================
CREATE TABLE aircraft_other_names (
  registration_number   VARCHAR(5) REFERENCES aircraft_registrations(registration_number) ON DELETE CASCADE,
  seq        SMALLINT CHECK (seq BETWEEN 1 AND 5),
  other_name VARCHAR(50),
  PRIMARY KEY (registration_number, seq)
);

-- =========================================================
-- Helpful indexes
-- =========================================================
CREATE INDEX aircraft_registrations_mfr_mdl_idx     ON aircraft_registrations (mfr_mdl_code);
CREATE INDEX aircraft_registrations_eng_mfr_mdl_idx ON aircraft_registrations (eng_mfr_mdl_code);
CREATE INDEX aircraft_registrations_state_county    ON aircraft_registrations (state, county_mail_code);
CREATE INDEX aircraft_registrations_status_idx      ON aircraft_registrations (status_code);
CREATE INDEX aircraft_registrations_aw_class_idx    ON aircraft_registrations (airworthiness_class_code, type_aircraft_code);
CREATE INDEX aircraft_registrations_transponder_idx ON aircraft_registrations (transponder_code);
CREATE INDEX aircraft_registrations_serial_idx      ON aircraft_registrations (serial_number);

-- =========================================================
-- SEED VALUES (from the first 8 pages only)
-- =========================================================

-- Type of Registration (pos 57)
INSERT INTO type_registrations (code, description) VALUES
  ('1','Individual'),
  ('2','Partnership'),
  ('3','Corporation'),
  ('4','Co-Owned'),
  ('5','Government'),
  ('7','LLC'),
  ('8','Non-Citizen Corporation'),
  ('9','Non-Citizen Co-Owned');

-- Airworthiness Class (pos 238)
INSERT INTO airworthiness_classes (code, description) VALUES
  ('1','Standard'),
  ('2','Limited'),
  ('3','Restricted'),
  ('4','Experimental'),
  ('5','Provisional'),
  ('6','Multiple'),
  ('7','Primary'),
  ('8','Special Flight Permit'),
  ('9','Light Sport');

-- Type Aircraft (pos 249)
INSERT INTO type_aircraft (code, description) VALUES
  ('1','Glider'),
  ('2','Balloon'),
  ('3','Blimp/Dirigible'),
  ('4','Fixed Wing Single-Engine'),
  ('5','Fixed Wing Multi-Engine'),
  ('6','Rotorcraft'),
  ('7','Weight-Shift-Control'),
  ('8','Powered Parachute'),
  ('9','Gyroplane'),
  ('H','Hybrid Lift'),
  ('O','Other');

-- Type Engine (pos 251–252)
INSERT INTO type_engines (code, description) VALUES
  ('00','None'),
  ('01','Reciprocating'),
  ('02','Turbo-prop'),
  ('03','Turbo-shaft'),
  ('04','Turbo-jet'),
  ('05','Turbo-fan'),
  ('06','Ramjet'),
  ('07','2-Cycle'),
  ('08','4-Cycle'),
  ('09','Unknown'),
  ('10','Electric'),
  ('11','Rotary');

-- Regions (pos 211)
INSERT INTO regions (code, description) VALUES
  ('1','Eastern'),
  ('2','Southwestern'),
  ('3','Central'),
  ('4','Western-Pacific'),
  ('5','Alaskan'),
  ('7','Southern'),
  ('8','European'),
  ('C','Great Lakes'),
  ('E','New England'),
  ('S','Northwest Mountain');

-- Status Codes (pos 254–255)
-- The first 8 pages present both letters and numeric codes. Below are the ones
-- that are explicitly spelled out there. You can extend this list later if you
-- need all letter-by-letter wording.
INSERT INTO status_codes (code, description) VALUES
  ('A','Triennial Aircraft Registration form mailed; not returned by Post Office'),
  ('D','Expired Dealer'),
  ('E','Certificate of Aircraft Registration revoked'),
  ('M','Certificate of Registration mailed (context per doc)'),
  ('N','Non-citizen Corporation (status context per doc)'),
  ('R','Registration pending (context per doc)'),
  ('T','Valid Registration from a Trainee'),
  ('V','Valid Registration'),
  ('W','Certificate of Registration deemed Ineffective or Invalid'),
  ('X','Enforcement Letter'),
  ('Z','Permanent Reserved'),
  ('1','Triennial form returned undeliverable'),
  ('2','N-Number assigned but not registered'),
  ('3','N-Number assigned as a Non Type Certificated aircraft; not yet registered'),
  ('4','N-Number assigned as import; not yet registered'),
  ('5','Reserved N-Number'),
  ('6','Administratively canceled'),
  ('7','Sale reported'),
  ('8','Second attempt at mailing Triennial form; no response'),
  ('9','Certificate of Registration revoked'),
  ('10','N-Number assigned, not registered, pending cancellation'),
  ('11','N-Number assigned, not registered, canceled'),
  ('12','Sale reported – pending cancellation'),
  ('13','Sale reported – canceled'),
  ('14','Registration pending'),
  ('15','Second Notice for Re-Registration/Renewal'),
  ('16','Registration Expired – Pending Cancellation'),
  ('17','Sale Reported – Pending Cancellation'),
  ('18','Sale Reported – Canceled'),
  ('19','Registration Pending – Pending Cancellation'),
  ('20','Registration Pending – Canceled'),
  ('21','Revoked – Pending Cancellation'),
  ('22','Revoked – Canceled'),
  ('23','Expired Dealer (Pending Cancellation)'),
  ('24','Third Notice for Re-Registration/Renewal'),
  ('25','First Notice for Registration Renewal'),
  ('26','Second Notice for Registration Renewal'),
  ('27','Registration Expired'),
  ('28','Third Notice for Registration Renewal'),
  ('29','Registration Expired – Pending Cancellation');
