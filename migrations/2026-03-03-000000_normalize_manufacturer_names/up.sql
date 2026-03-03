-- Normalize manufacturer names in aircraft.aircraft_model and aircraft_models.manufacturer_name.
-- Replace verbose corporate legal names with clean canonical forms.
--
-- Uses a temporary function to apply longest-match-first prefix replacement,
-- handling both "MANUFACTURER MODEL" and standalone "MANUFACTURER" values.

CREATE OR REPLACE FUNCTION _normalize_aircraft_model(input TEXT) RETURNS TEXT AS $$
DECLARE
    upper_input TEXT := upper(trim(input));
    trimmed TEXT := trim(input);
    -- Each entry: pattern (UPPERCASE), canonical replacement
    -- Ordered longest-first so the first match wins.
    patterns TEXT[][] := ARRAY[
        -- Boeing
        ARRAY['THE BOEING COMPANY', 'Boeing'],
        ARRAY['BOEING CO THE', 'Boeing'],
        ARRAY['BOEING COMPANY', 'Boeing'],
        ARRAY['BOEING CO', 'Boeing'],
        ARRAY['BOEING', 'Boeing'],
        -- Cessna
        ARRAY['CESSNA AIRCRAFT INC', 'Cessna'],
        ARRAY['CESSNA AIRCRAFT CO', 'Cessna'],
        ARRAY['CESSNA AIRCRAFT', 'Cessna'],
        ARRAY['CESSNA', 'Cessna'],
        -- Piper
        ARRAY['NEW PIPER AIRCRAFT INC', 'Piper'],
        ARRAY['NEW PIPER AIRCRAFT', 'Piper'],
        ARRAY['PIPER AIRCRAFT CORP', 'Piper'],
        ARRAY['PIPER AIRCRAFT INC', 'Piper'],
        ARRAY['PIPER AIRCRAFT', 'Piper'],
        ARRAY['NEW PIPER', 'Piper'],
        ARRAY['PIPER', 'Piper'],
        -- Beechcraft
        ARRAY['HAWKER BEECHCRAFT CORP', 'Beechcraft'],
        ARRAY['BEECHCRAFT AIRCRAFT CORP', 'Beechcraft'],
        ARRAY['BEECHCRAFT CORP', 'Beechcraft'],
        ARRAY['BEECH AIRCRAFT CORP', 'Beechcraft'],
        ARRAY['BEECHCRAFT', 'Beechcraft'],
        ARRAY['BEECH', 'Beechcraft'],
        -- Airbus
        ARRAY['AIRBUS HELICOPTERS', 'Airbus Helicopters'],
        ARRAY['AIRBUS INDUSTRIE', 'Airbus'],
        ARRAY['AIRBUS S A S', 'Airbus'],
        ARRAY['AIRBUS SAS', 'Airbus'],
        ARRAY['AIRBUS', 'Airbus'],
        -- Cirrus
        ARRAY['CIRRUS DESIGN CORPORATION', 'Cirrus'],
        ARRAY['CIRRUS DESIGN CORP', 'Cirrus'],
        ARRAY['CIRRUS', 'Cirrus'],
        -- Bell
        ARRAY['BELL HELICOPTER TEXTRON', 'Bell'],
        ARRAY['BELL TEXTRON INC', 'Bell'],
        ARRAY['BELL', 'Bell'],
        -- Robinson
        ARRAY['ROBINSON HELICOPTER COMPANY', 'Robinson'],
        ARRAY['ROBINSON HELICOPTER CO', 'Robinson'],
        ARRAY['ROBINSON HELICOPTER', 'Robinson'],
        ARRAY['ROBINSON', 'Robinson'],
        -- Mooney
        ARRAY['MOONEY INTERNATIONAL CORP', 'Mooney'],
        ARRAY['MOONEY AIRCRAFT CORP', 'Mooney'],
        ARRAY['MOONEY', 'Mooney'],
        -- Textron Aviation
        ARRAY['TEXTRON AVIATION INC', 'Textron Aviation'],
        ARRAY['TEXTRON AVIATION', 'Textron Aviation'],
        -- Bombardier
        ARRAY['BOMBARDIER INC CANADAIR', 'Bombardier'],
        ARRAY['BOMBARDIER AEROSPACE', 'Bombardier'],
        ARRAY['BOMBARDIER INC', 'Bombardier'],
        ARRAY['BOMBARDIER', 'Bombardier'],
        -- Gulfstream
        ARRAY['GULFSTREAM AEROSPACE CORP', 'Gulfstream'],
        ARRAY['GULFSTREAM AEROSPACE', 'Gulfstream'],
        ARRAY['GULFSTREAM', 'Gulfstream'],
        -- Grumman / Northrop Grumman
        ARRAY['NORTHROP GRUMMAN', 'Northrop Grumman'],
        ARRAY['GRUMMAN AMERICAN AVN CORP', 'Grumman'],
        ARRAY['GRUMMAN AIRCRAFT ENG CORP', 'Grumman'],
        ARRAY['GRUMMAN', 'Grumman'],
        -- Embraer
        ARRAY['EMBRAER EMPRESA BRASILEIRA DE', 'Embraer'],
        ARRAY['EMBRAER S A', 'Embraer'],
        ARRAY['EMBRAER SA', 'Embraer'],
        ARRAY['EMBRAER', 'Embraer'],
        -- Diamond
        ARRAY['DIAMOND AIRCRAFT INDUSTRIES', 'Diamond'],
        ARRAY['DIAMOND AIRCRAFT IND', 'Diamond'],
        ARRAY['DIAMOND AIRCRAFT', 'Diamond'],
        ARRAY['DIAMOND', 'Diamond'],
        -- Maule
        ARRAY['MAULE AEROSPACE TECHNOLOGY INC', 'Maule'],
        ARRAY['MAULE AEROSPACE TECHNOLOGY', 'Maule'],
        ARRAY['MAULE AIR INC', 'Maule'],
        ARRAY['MAULE', 'Maule'],
        -- Socata
        ARRAY['SOCATA GROUP AEROSPATIALE', 'Socata'],
        ARRAY['EADS SOCATA', 'Socata'],
        ARRAY['SOCATA', 'Socata'],
        -- Pilatus
        ARRAY['PILATUS FLUGZEUGWERKE AG', 'Pilatus'],
        ARRAY['PILATUS AIRCRAFT LTD', 'Pilatus'],
        ARRAY['PILATUS', 'Pilatus'],
        -- Dassault
        ARRAY['AVIONS MARCEL DASSAULT', 'Dassault'],
        ARRAY['DASSAULT BREGUET', 'Dassault'],
        ARRAY['DASSAULT FALCON', 'Dassault'],
        ARRAY['DASSAULT AVIATION', 'Dassault'],
        ARRAY['DASSAULT', 'Dassault'],
        -- Sikorsky
        ARRAY['SIKORSKY AIRCRAFT CORP', 'Sikorsky'],
        ARRAY['SIKORSKY AIRCRAFT', 'Sikorsky'],
        ARRAY['SIKORSKY', 'Sikorsky'],
        -- Raytheon
        ARRAY['RAYTHEON AIRCRAFT COMPANY', 'Raytheon'],
        ARRAY['RAYTHEON AIRCRAFT CO', 'Raytheon'],
        ARRAY['RAYTHEON', 'Raytheon'],
        -- McDonnell Douglas
        ARRAY['MCDONNELL DOUGLAS HELICOPTER', 'McDonnell Douglas'],
        ARRAY['MCDONNELL DOUGLAS CORP', 'McDonnell Douglas'],
        ARRAY['MCDONNELL DOUGLAS', 'McDonnell Douglas'],
        -- Lockheed
        ARRAY['LOCKHEED MARTIN CORP', 'Lockheed Martin'],
        ARRAY['LOCKHEED MARTIN', 'Lockheed Martin'],
        ARRAY['LOCKHEED', 'Lockheed'],
        -- De Havilland
        ARRAY['DE HAVILLAND AIRCRAFT OF CANADA', 'De Havilland Canada'],
        ARRAY['DE HAVILLAND CANADA', 'De Havilland Canada'],
        ARRAY['DE HAVILLAND', 'De Havilland'],
        -- Eurocopter / Aerospatiale
        ARRAY['EUROCOPTER FRANCE', 'Eurocopter'],
        ARRAY['EUROCOPTER DEUTSCHLAND', 'Eurocopter'],
        ARRAY['EUROCOPTER', 'Eurocopter'],
        ARRAY['AEROSPATIALE', 'Aerospatiale'],
        -- Learjet
        ARRAY['LEARJET INC', 'Learjet'],
        ARRAY['LEARJET', 'Learjet'],
        -- Fairchild
        ARRAY['FAIRCHILD INDUSTRIES INC', 'Fairchild'],
        ARRAY['FAIRCHILD AIRCRAFT INC', 'Fairchild'],
        ARRAY['FAIRCHILD', 'Fairchild'],
        -- American Champion
        ARRAY['AMERICAN CHAMPION AIRCRAFT CORP', 'American Champion'],
        ARRAY['AMERICAN CHAMPION AIRCRAFT', 'American Champion'],
        ARRAY['AMERICAN CHAMPION', 'American Champion'],
        -- Glider manufacturers
        ARRAY['SCHEMPP-HIRTH FLUGZEUGBAU GMBH', 'Schempp-Hirth'],
        ARRAY['SCHEMPP-HIRTH', 'Schempp-Hirth'],
        ARRAY['ALEXANDER SCHLEICHER GMBH', 'Schleicher'],
        ARRAY['ALEXANDER SCHLEICHER', 'Schleicher'],
        ARRAY['DG FLUGZEUGBAU GMBH', 'DG Flugzeugbau'],
        ARRAY['DG FLUGZEUGBAU', 'DG Flugzeugbau'],
        ARRAY['ROLLADEN-SCHNEIDER FLUGZEUGBAU', 'Rolladen-Schneider'],
        ARRAY['ROLLADEN-SCHNEIDER', 'Rolladen-Schneider']
    ];
    pat TEXT;
    canonical TEXT;
    rest TEXT;
    after_char TEXT;
BEGIN
    IF trimmed IS NULL OR trimmed = '' THEN
        RETURN input;
    END IF;

    FOR i IN 1..array_length(patterns, 1) LOOP
        pat := patterns[i][1];
        canonical := patterns[i][2];

        IF upper_input LIKE pat || ' %' THEN
            -- Prefix match with model after it
            rest := ltrim(substr(trimmed, length(pat) + 1));
            RETURN canonical || ' ' || rest;
        ELSIF upper_input = pat THEN
            -- Exact match (manufacturer-only value)
            RETURN canonical;
        END IF;
    END LOOP;

    -- No match, return unchanged
    RETURN trimmed;
END;
$$ LANGUAGE plpgsql;

-- Apply to aircraft.aircraft_model
UPDATE aircraft
SET aircraft_model = _normalize_aircraft_model(aircraft_model)
WHERE _normalize_aircraft_model(aircraft_model) != aircraft_model;

-- Apply to aircraft_models.manufacturer_name (standalone manufacturer names)
-- For these, exact match is the norm, but we use the same function for consistency.
UPDATE aircraft_models
SET manufacturer_name = _normalize_aircraft_model(manufacturer_name)
WHERE _normalize_aircraft_model(manufacturer_name) != manufacturer_name;

-- Clean up the temporary function
DROP FUNCTION _normalize_aircraft_model(TEXT);
