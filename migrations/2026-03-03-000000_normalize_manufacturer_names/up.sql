-- Normalize manufacturer names in aircraft.aircraft_model
-- Replace verbose corporate legal names with clean canonical forms.
-- Each UPDATE targets a specific prefix pattern and replaces it.

-- Boeing variants
UPDATE aircraft SET aircraft_model = 'Boeing' || substr(aircraft_model, length('THE BOEING COMPANY') + 1)
WHERE upper(aircraft_model) LIKE 'THE BOEING COMPANY %';

UPDATE aircraft SET aircraft_model = 'Boeing' || substr(aircraft_model, length('BOEING CO THE') + 1)
WHERE upper(aircraft_model) LIKE 'BOEING CO THE %';

UPDATE aircraft SET aircraft_model = 'Boeing' || substr(aircraft_model, length('BOEING COMPANY') + 1)
WHERE upper(aircraft_model) LIKE 'BOEING COMPANY %';

UPDATE aircraft SET aircraft_model = 'Boeing' || substr(aircraft_model, length('BOEING CO') + 1)
WHERE upper(aircraft_model) LIKE 'BOEING CO %';

UPDATE aircraft SET aircraft_model = 'Boeing' || substr(aircraft_model, length('BOEING') + 1)
WHERE upper(aircraft_model) LIKE 'BOEING %'
  AND upper(aircraft_model) NOT LIKE 'BOEING CO%'
  AND upper(aircraft_model) NOT LIKE 'BOEING COMPANY%';

-- Cessna variants
UPDATE aircraft SET aircraft_model = 'Cessna' || substr(aircraft_model, length('CESSNA AIRCRAFT INC') + 1)
WHERE upper(aircraft_model) LIKE 'CESSNA AIRCRAFT INC %';

UPDATE aircraft SET aircraft_model = 'Cessna' || substr(aircraft_model, length('CESSNA AIRCRAFT CO') + 1)
WHERE upper(aircraft_model) LIKE 'CESSNA AIRCRAFT CO %';

UPDATE aircraft SET aircraft_model = 'Cessna' || substr(aircraft_model, length('CESSNA AIRCRAFT') + 1)
WHERE upper(aircraft_model) LIKE 'CESSNA AIRCRAFT %'
  AND upper(aircraft_model) NOT LIKE 'CESSNA AIRCRAFT INC%'
  AND upper(aircraft_model) NOT LIKE 'CESSNA AIRCRAFT CO%';

UPDATE aircraft SET aircraft_model = 'Cessna' || substr(aircraft_model, length('CESSNA') + 1)
WHERE upper(aircraft_model) LIKE 'CESSNA %'
  AND upper(aircraft_model) NOT LIKE 'CESSNA AIRCRAFT%';

-- Piper variants
UPDATE aircraft SET aircraft_model = 'Piper' || substr(aircraft_model, length('NEW PIPER AIRCRAFT INC') + 1)
WHERE upper(aircraft_model) LIKE 'NEW PIPER AIRCRAFT INC %';

UPDATE aircraft SET aircraft_model = 'Piper' || substr(aircraft_model, length('NEW PIPER AIRCRAFT') + 1)
WHERE upper(aircraft_model) LIKE 'NEW PIPER AIRCRAFT %'
  AND upper(aircraft_model) NOT LIKE 'NEW PIPER AIRCRAFT INC%';

UPDATE aircraft SET aircraft_model = 'Piper' || substr(aircraft_model, length('PIPER AIRCRAFT CORP') + 1)
WHERE upper(aircraft_model) LIKE 'PIPER AIRCRAFT CORP %';

UPDATE aircraft SET aircraft_model = 'Piper' || substr(aircraft_model, length('PIPER AIRCRAFT INC') + 1)
WHERE upper(aircraft_model) LIKE 'PIPER AIRCRAFT INC %';

UPDATE aircraft SET aircraft_model = 'Piper' || substr(aircraft_model, length('PIPER AIRCRAFT') + 1)
WHERE upper(aircraft_model) LIKE 'PIPER AIRCRAFT %'
  AND upper(aircraft_model) NOT LIKE 'PIPER AIRCRAFT CORP%'
  AND upper(aircraft_model) NOT LIKE 'PIPER AIRCRAFT INC%';

UPDATE aircraft SET aircraft_model = 'Piper' || substr(aircraft_model, length('NEW PIPER') + 1)
WHERE upper(aircraft_model) LIKE 'NEW PIPER %'
  AND upper(aircraft_model) NOT LIKE 'NEW PIPER AIRCRAFT%';

UPDATE aircraft SET aircraft_model = 'Piper' || substr(aircraft_model, length('PIPER') + 1)
WHERE upper(aircraft_model) LIKE 'PIPER %'
  AND upper(aircraft_model) NOT LIKE 'PIPER AIRCRAFT%';

-- Beechcraft variants
UPDATE aircraft SET aircraft_model = 'Beechcraft' || substr(aircraft_model, length('HAWKER BEECHCRAFT CORP') + 1)
WHERE upper(aircraft_model) LIKE 'HAWKER BEECHCRAFT CORP %';

UPDATE aircraft SET aircraft_model = 'Beechcraft' || substr(aircraft_model, length('BEECHCRAFT AIRCRAFT CORP') + 1)
WHERE upper(aircraft_model) LIKE 'BEECHCRAFT AIRCRAFT CORP %';

UPDATE aircraft SET aircraft_model = 'Beechcraft' || substr(aircraft_model, length('BEECHCRAFT CORP') + 1)
WHERE upper(aircraft_model) LIKE 'BEECHCRAFT CORP %';

UPDATE aircraft SET aircraft_model = 'Beechcraft' || substr(aircraft_model, length('BEECH AIRCRAFT CORP') + 1)
WHERE upper(aircraft_model) LIKE 'BEECH AIRCRAFT CORP %';

UPDATE aircraft SET aircraft_model = 'Beechcraft' || substr(aircraft_model, length('BEECHCRAFT') + 1)
WHERE upper(aircraft_model) LIKE 'BEECHCRAFT %'
  AND upper(aircraft_model) NOT LIKE 'BEECHCRAFT CORP%'
  AND upper(aircraft_model) NOT LIKE 'BEECHCRAFT AIRCRAFT%';

UPDATE aircraft SET aircraft_model = 'Beechcraft' || substr(aircraft_model, length('BEECH') + 1)
WHERE upper(aircraft_model) LIKE 'BEECH %'
  AND upper(aircraft_model) NOT LIKE 'BEECH AIRCRAFT%'
  AND upper(aircraft_model) NOT LIKE 'BEECHCRAFT%';

-- Airbus variants
UPDATE aircraft SET aircraft_model = 'Airbus Helicopters' || substr(aircraft_model, length('AIRBUS HELICOPTERS') + 1)
WHERE upper(aircraft_model) LIKE 'AIRBUS HELICOPTERS %';

UPDATE aircraft SET aircraft_model = 'Airbus' || substr(aircraft_model, length('AIRBUS INDUSTRIE') + 1)
WHERE upper(aircraft_model) LIKE 'AIRBUS INDUSTRIE %';

UPDATE aircraft SET aircraft_model = 'Airbus' || substr(aircraft_model, length('AIRBUS S A S') + 1)
WHERE upper(aircraft_model) LIKE 'AIRBUS S A S %';

UPDATE aircraft SET aircraft_model = 'Airbus' || substr(aircraft_model, length('AIRBUS SAS') + 1)
WHERE upper(aircraft_model) LIKE 'AIRBUS SAS %';

UPDATE aircraft SET aircraft_model = 'Airbus' || substr(aircraft_model, length('AIRBUS') + 1)
WHERE upper(aircraft_model) LIKE 'AIRBUS %'
  AND upper(aircraft_model) NOT LIKE 'AIRBUS HELICOPTERS%'
  AND upper(aircraft_model) NOT LIKE 'AIRBUS INDUSTRIE%'
  AND upper(aircraft_model) NOT LIKE 'AIRBUS S A S%'
  AND upper(aircraft_model) NOT LIKE 'AIRBUS SAS%';

-- Cirrus variants
UPDATE aircraft SET aircraft_model = 'Cirrus' || substr(aircraft_model, length('CIRRUS DESIGN CORPORATION') + 1)
WHERE upper(aircraft_model) LIKE 'CIRRUS DESIGN CORPORATION %';

UPDATE aircraft SET aircraft_model = 'Cirrus' || substr(aircraft_model, length('CIRRUS DESIGN CORP') + 1)
WHERE upper(aircraft_model) LIKE 'CIRRUS DESIGN CORP %'
  AND upper(aircraft_model) NOT LIKE 'CIRRUS DESIGN CORPORATION%';

UPDATE aircraft SET aircraft_model = 'Cirrus' || substr(aircraft_model, length('CIRRUS') + 1)
WHERE upper(aircraft_model) LIKE 'CIRRUS %'
  AND upper(aircraft_model) NOT LIKE 'CIRRUS DESIGN%';

-- Bell variants
UPDATE aircraft SET aircraft_model = 'Bell' || substr(aircraft_model, length('BELL HELICOPTER TEXTRON') + 1)
WHERE upper(aircraft_model) LIKE 'BELL HELICOPTER TEXTRON %';

UPDATE aircraft SET aircraft_model = 'Bell' || substr(aircraft_model, length('BELL TEXTRON INC') + 1)
WHERE upper(aircraft_model) LIKE 'BELL TEXTRON INC %';

-- Robinson variants
UPDATE aircraft SET aircraft_model = 'Robinson' || substr(aircraft_model, length('ROBINSON HELICOPTER COMPANY') + 1)
WHERE upper(aircraft_model) LIKE 'ROBINSON HELICOPTER COMPANY %';

UPDATE aircraft SET aircraft_model = 'Robinson' || substr(aircraft_model, length('ROBINSON HELICOPTER CO') + 1)
WHERE upper(aircraft_model) LIKE 'ROBINSON HELICOPTER CO %'
  AND upper(aircraft_model) NOT LIKE 'ROBINSON HELICOPTER COMPANY%';

UPDATE aircraft SET aircraft_model = 'Robinson' || substr(aircraft_model, length('ROBINSON HELICOPTER') + 1)
WHERE upper(aircraft_model) LIKE 'ROBINSON HELICOPTER %'
  AND upper(aircraft_model) NOT LIKE 'ROBINSON HELICOPTER CO%';

-- Mooney variants
UPDATE aircraft SET aircraft_model = 'Mooney' || substr(aircraft_model, length('MOONEY INTERNATIONAL CORP') + 1)
WHERE upper(aircraft_model) LIKE 'MOONEY INTERNATIONAL CORP %';

UPDATE aircraft SET aircraft_model = 'Mooney' || substr(aircraft_model, length('MOONEY AIRCRAFT CORP') + 1)
WHERE upper(aircraft_model) LIKE 'MOONEY AIRCRAFT CORP %';

-- Textron Aviation
UPDATE aircraft SET aircraft_model = 'Textron Aviation' || substr(aircraft_model, length('TEXTRON AVIATION INC') + 1)
WHERE upper(aircraft_model) LIKE 'TEXTRON AVIATION INC %';

UPDATE aircraft SET aircraft_model = 'Textron Aviation' || substr(aircraft_model, length('TEXTRON AVIATION') + 1)
WHERE upper(aircraft_model) LIKE 'TEXTRON AVIATION %'
  AND upper(aircraft_model) NOT LIKE 'TEXTRON AVIATION INC%';

-- Bombardier variants
UPDATE aircraft SET aircraft_model = 'Bombardier' || substr(aircraft_model, length('BOMBARDIER INC CANADAIR') + 1)
WHERE upper(aircraft_model) LIKE 'BOMBARDIER INC CANADAIR %';

UPDATE aircraft SET aircraft_model = 'Bombardier' || substr(aircraft_model, length('BOMBARDIER AEROSPACE') + 1)
WHERE upper(aircraft_model) LIKE 'BOMBARDIER AEROSPACE %';

UPDATE aircraft SET aircraft_model = 'Bombardier' || substr(aircraft_model, length('BOMBARDIER INC') + 1)
WHERE upper(aircraft_model) LIKE 'BOMBARDIER INC %'
  AND upper(aircraft_model) NOT LIKE 'BOMBARDIER INC CANADAIR%';

UPDATE aircraft SET aircraft_model = 'Bombardier' || substr(aircraft_model, length('BOMBARDIER') + 1)
WHERE upper(aircraft_model) LIKE 'BOMBARDIER %'
  AND upper(aircraft_model) NOT LIKE 'BOMBARDIER INC%'
  AND upper(aircraft_model) NOT LIKE 'BOMBARDIER AEROSPACE%';

-- Gulfstream variants
UPDATE aircraft SET aircraft_model = 'Gulfstream' || substr(aircraft_model, length('GULFSTREAM AEROSPACE CORP') + 1)
WHERE upper(aircraft_model) LIKE 'GULFSTREAM AEROSPACE CORP %';

UPDATE aircraft SET aircraft_model = 'Gulfstream' || substr(aircraft_model, length('GULFSTREAM AEROSPACE') + 1)
WHERE upper(aircraft_model) LIKE 'GULFSTREAM AEROSPACE %'
  AND upper(aircraft_model) NOT LIKE 'GULFSTREAM AEROSPACE CORP%';

UPDATE aircraft SET aircraft_model = 'Gulfstream' || substr(aircraft_model, length('GULFSTREAM') + 1)
WHERE upper(aircraft_model) LIKE 'GULFSTREAM %'
  AND upper(aircraft_model) NOT LIKE 'GULFSTREAM AEROSPACE%';

-- Grumman / Northrop Grumman
UPDATE aircraft SET aircraft_model = 'Northrop Grumman' || substr(aircraft_model, length('NORTHROP GRUMMAN') + 1)
WHERE upper(aircraft_model) LIKE 'NORTHROP GRUMMAN %';

UPDATE aircraft SET aircraft_model = 'Grumman' || substr(aircraft_model, length('GRUMMAN AMERICAN AVN CORP') + 1)
WHERE upper(aircraft_model) LIKE 'GRUMMAN AMERICAN AVN CORP %';

UPDATE aircraft SET aircraft_model = 'Grumman' || substr(aircraft_model, length('GRUMMAN AIRCRAFT ENG CORP') + 1)
WHERE upper(aircraft_model) LIKE 'GRUMMAN AIRCRAFT ENG CORP %';

UPDATE aircraft SET aircraft_model = 'Grumman' || substr(aircraft_model, length('GRUMMAN') + 1)
WHERE upper(aircraft_model) LIKE 'GRUMMAN %'
  AND upper(aircraft_model) NOT LIKE 'GRUMMAN AMERICAN%'
  AND upper(aircraft_model) NOT LIKE 'GRUMMAN AIRCRAFT%';

-- Embraer variants
UPDATE aircraft SET aircraft_model = 'Embraer' || substr(aircraft_model, length('EMBRAER EMPRESA BRASILEIRA DE') + 1)
WHERE upper(aircraft_model) LIKE 'EMBRAER EMPRESA BRASILEIRA DE %';

UPDATE aircraft SET aircraft_model = 'Embraer' || substr(aircraft_model, length('EMBRAER S A') + 1)
WHERE upper(aircraft_model) LIKE 'EMBRAER S A %';

UPDATE aircraft SET aircraft_model = 'Embraer' || substr(aircraft_model, length('EMBRAER SA') + 1)
WHERE upper(aircraft_model) LIKE 'EMBRAER SA %';

UPDATE aircraft SET aircraft_model = 'Embraer' || substr(aircraft_model, length('EMBRAER') + 1)
WHERE upper(aircraft_model) LIKE 'EMBRAER %'
  AND upper(aircraft_model) NOT LIKE 'EMBRAER EMPRESA%'
  AND upper(aircraft_model) NOT LIKE 'EMBRAER S A%'
  AND upper(aircraft_model) NOT LIKE 'EMBRAER SA%';

-- Diamond variants
UPDATE aircraft SET aircraft_model = 'Diamond' || substr(aircraft_model, length('DIAMOND AIRCRAFT INDUSTRIES') + 1)
WHERE upper(aircraft_model) LIKE 'DIAMOND AIRCRAFT INDUSTRIES %';

UPDATE aircraft SET aircraft_model = 'Diamond' || substr(aircraft_model, length('DIAMOND AIRCRAFT IND') + 1)
WHERE upper(aircraft_model) LIKE 'DIAMOND AIRCRAFT IND %'
  AND upper(aircraft_model) NOT LIKE 'DIAMOND AIRCRAFT INDUSTRIES%';

UPDATE aircraft SET aircraft_model = 'Diamond' || substr(aircraft_model, length('DIAMOND AIRCRAFT') + 1)
WHERE upper(aircraft_model) LIKE 'DIAMOND AIRCRAFT %'
  AND upper(aircraft_model) NOT LIKE 'DIAMOND AIRCRAFT IND%';

-- Maule variants
UPDATE aircraft SET aircraft_model = 'Maule' || substr(aircraft_model, length('MAULE AEROSPACE TECHNOLOGY INC') + 1)
WHERE upper(aircraft_model) LIKE 'MAULE AEROSPACE TECHNOLOGY INC %';

UPDATE aircraft SET aircraft_model = 'Maule' || substr(aircraft_model, length('MAULE AEROSPACE TECHNOLOGY') + 1)
WHERE upper(aircraft_model) LIKE 'MAULE AEROSPACE TECHNOLOGY %'
  AND upper(aircraft_model) NOT LIKE 'MAULE AEROSPACE TECHNOLOGY INC%';

UPDATE aircraft SET aircraft_model = 'Maule' || substr(aircraft_model, length('MAULE AIR INC') + 1)
WHERE upper(aircraft_model) LIKE 'MAULE AIR INC %';

-- Socata variants
UPDATE aircraft SET aircraft_model = 'Socata' || substr(aircraft_model, length('SOCATA GROUP AEROSPATIALE') + 1)
WHERE upper(aircraft_model) LIKE 'SOCATA GROUP AEROSPATIALE %';

UPDATE aircraft SET aircraft_model = 'Socata' || substr(aircraft_model, length('EADS SOCATA') + 1)
WHERE upper(aircraft_model) LIKE 'EADS SOCATA %';

-- Pilatus variants
UPDATE aircraft SET aircraft_model = 'Pilatus' || substr(aircraft_model, length('PILATUS FLUGZEUGWERKE AG') + 1)
WHERE upper(aircraft_model) LIKE 'PILATUS FLUGZEUGWERKE AG %';

UPDATE aircraft SET aircraft_model = 'Pilatus' || substr(aircraft_model, length('PILATUS AIRCRAFT LTD') + 1)
WHERE upper(aircraft_model) LIKE 'PILATUS AIRCRAFT LTD %';

-- Dassault variants
UPDATE aircraft SET aircraft_model = 'Dassault' || substr(aircraft_model, length('AVIONS MARCEL DASSAULT') + 1)
WHERE upper(aircraft_model) LIKE 'AVIONS MARCEL DASSAULT %';

UPDATE aircraft SET aircraft_model = 'Dassault' || substr(aircraft_model, length('DASSAULT BREGUET') + 1)
WHERE upper(aircraft_model) LIKE 'DASSAULT BREGUET %';

UPDATE aircraft SET aircraft_model = 'Dassault' || substr(aircraft_model, length('DASSAULT FALCON') + 1)
WHERE upper(aircraft_model) LIKE 'DASSAULT FALCON %';

UPDATE aircraft SET aircraft_model = 'Dassault' || substr(aircraft_model, length('DASSAULT AVIATION') + 1)
WHERE upper(aircraft_model) LIKE 'DASSAULT AVIATION %';

-- Sikorsky variants
UPDATE aircraft SET aircraft_model = 'Sikorsky' || substr(aircraft_model, length('SIKORSKY AIRCRAFT CORP') + 1)
WHERE upper(aircraft_model) LIKE 'SIKORSKY AIRCRAFT CORP %';

UPDATE aircraft SET aircraft_model = 'Sikorsky' || substr(aircraft_model, length('SIKORSKY AIRCRAFT') + 1)
WHERE upper(aircraft_model) LIKE 'SIKORSKY AIRCRAFT %'
  AND upper(aircraft_model) NOT LIKE 'SIKORSKY AIRCRAFT CORP%';

-- Raytheon variants
UPDATE aircraft SET aircraft_model = 'Raytheon' || substr(aircraft_model, length('RAYTHEON AIRCRAFT COMPANY') + 1)
WHERE upper(aircraft_model) LIKE 'RAYTHEON AIRCRAFT COMPANY %';

UPDATE aircraft SET aircraft_model = 'Raytheon' || substr(aircraft_model, length('RAYTHEON AIRCRAFT CO') + 1)
WHERE upper(aircraft_model) LIKE 'RAYTHEON AIRCRAFT CO %'
  AND upper(aircraft_model) NOT LIKE 'RAYTHEON AIRCRAFT COMPANY%';

-- McDonnell Douglas
UPDATE aircraft SET aircraft_model = 'McDonnell Douglas' || substr(aircraft_model, length('MCDONNELL DOUGLAS HELICOPTER') + 1)
WHERE upper(aircraft_model) LIKE 'MCDONNELL DOUGLAS HELICOPTER %';

UPDATE aircraft SET aircraft_model = 'McDonnell Douglas' || substr(aircraft_model, length('MCDONNELL DOUGLAS CORP') + 1)
WHERE upper(aircraft_model) LIKE 'MCDONNELL DOUGLAS CORP %';

UPDATE aircraft SET aircraft_model = 'McDonnell Douglas' || substr(aircraft_model, length('MCDONNELL DOUGLAS') + 1)
WHERE upper(aircraft_model) LIKE 'MCDONNELL DOUGLAS %'
  AND upper(aircraft_model) NOT LIKE 'MCDONNELL DOUGLAS HELICOPTER%'
  AND upper(aircraft_model) NOT LIKE 'MCDONNELL DOUGLAS CORP%';

-- Lockheed Martin
UPDATE aircraft SET aircraft_model = 'Lockheed Martin' || substr(aircraft_model, length('LOCKHEED MARTIN CORP') + 1)
WHERE upper(aircraft_model) LIKE 'LOCKHEED MARTIN CORP %';

UPDATE aircraft SET aircraft_model = 'Lockheed Martin' || substr(aircraft_model, length('LOCKHEED MARTIN') + 1)
WHERE upper(aircraft_model) LIKE 'LOCKHEED MARTIN %'
  AND upper(aircraft_model) NOT LIKE 'LOCKHEED MARTIN CORP%';

-- De Havilland
UPDATE aircraft SET aircraft_model = 'De Havilland Canada' || substr(aircraft_model, length('DE HAVILLAND AIRCRAFT OF CANADA') + 1)
WHERE upper(aircraft_model) LIKE 'DE HAVILLAND AIRCRAFT OF CANADA %';

UPDATE aircraft SET aircraft_model = 'De Havilland Canada' || substr(aircraft_model, length('DE HAVILLAND CANADA') + 1)
WHERE upper(aircraft_model) LIKE 'DE HAVILLAND CANADA %';

UPDATE aircraft SET aircraft_model = 'De Havilland' || substr(aircraft_model, length('DE HAVILLAND') + 1)
WHERE upper(aircraft_model) LIKE 'DE HAVILLAND %'
  AND upper(aircraft_model) NOT LIKE 'DE HAVILLAND AIRCRAFT OF CANADA%'
  AND upper(aircraft_model) NOT LIKE 'DE HAVILLAND CANADA%';

-- Eurocopter
UPDATE aircraft SET aircraft_model = 'Eurocopter' || substr(aircraft_model, length('EUROCOPTER FRANCE') + 1)
WHERE upper(aircraft_model) LIKE 'EUROCOPTER FRANCE %';

UPDATE aircraft SET aircraft_model = 'Eurocopter' || substr(aircraft_model, length('EUROCOPTER DEUTSCHLAND') + 1)
WHERE upper(aircraft_model) LIKE 'EUROCOPTER DEUTSCHLAND %';

UPDATE aircraft SET aircraft_model = 'Eurocopter' || substr(aircraft_model, length('EUROCOPTER') + 1)
WHERE upper(aircraft_model) LIKE 'EUROCOPTER %'
  AND upper(aircraft_model) NOT LIKE 'EUROCOPTER FRANCE%'
  AND upper(aircraft_model) NOT LIKE 'EUROCOPTER DEUTSCHLAND%';

-- Learjet
UPDATE aircraft SET aircraft_model = 'Learjet' || substr(aircraft_model, length('LEARJET INC') + 1)
WHERE upper(aircraft_model) LIKE 'LEARJET INC %';

UPDATE aircraft SET aircraft_model = 'Learjet' || substr(aircraft_model, length('LEARJET') + 1)
WHERE upper(aircraft_model) LIKE 'LEARJET %'
  AND upper(aircraft_model) NOT LIKE 'LEARJET INC%';

-- Fairchild
UPDATE aircraft SET aircraft_model = 'Fairchild' || substr(aircraft_model, length('FAIRCHILD INDUSTRIES INC') + 1)
WHERE upper(aircraft_model) LIKE 'FAIRCHILD INDUSTRIES INC %';

UPDATE aircraft SET aircraft_model = 'Fairchild' || substr(aircraft_model, length('FAIRCHILD AIRCRAFT INC') + 1)
WHERE upper(aircraft_model) LIKE 'FAIRCHILD AIRCRAFT INC %';

UPDATE aircraft SET aircraft_model = 'Fairchild' || substr(aircraft_model, length('FAIRCHILD') + 1)
WHERE upper(aircraft_model) LIKE 'FAIRCHILD %'
  AND upper(aircraft_model) NOT LIKE 'FAIRCHILD INDUSTRIES%'
  AND upper(aircraft_model) NOT LIKE 'FAIRCHILD AIRCRAFT%';


-- Normalize aircraft_models.manufacturer_name (FAA registry table)
-- These are standalone manufacturer names, so we do exact replacements.

UPDATE aircraft_models SET manufacturer_name = 'Boeing' WHERE upper(manufacturer_name) IN ('THE BOEING COMPANY', 'BOEING CO THE', 'BOEING COMPANY', 'BOEING CO', 'BOEING');
UPDATE aircraft_models SET manufacturer_name = 'Cessna' WHERE upper(manufacturer_name) IN ('CESSNA AIRCRAFT INC', 'CESSNA AIRCRAFT CO', 'CESSNA AIRCRAFT', 'CESSNA');
UPDATE aircraft_models SET manufacturer_name = 'Piper' WHERE upper(manufacturer_name) IN ('NEW PIPER AIRCRAFT INC', 'NEW PIPER AIRCRAFT', 'PIPER AIRCRAFT CORP', 'PIPER AIRCRAFT INC', 'PIPER AIRCRAFT', 'NEW PIPER', 'PIPER');
UPDATE aircraft_models SET manufacturer_name = 'Beechcraft' WHERE upper(manufacturer_name) IN ('HAWKER BEECHCRAFT CORP', 'BEECHCRAFT AIRCRAFT CORP', 'BEECHCRAFT CORP', 'BEECH AIRCRAFT CORP', 'BEECHCRAFT', 'BEECH');
UPDATE aircraft_models SET manufacturer_name = 'Airbus' WHERE upper(manufacturer_name) IN ('AIRBUS INDUSTRIE', 'AIRBUS S A S', 'AIRBUS SAS', 'AIRBUS');
UPDATE aircraft_models SET manufacturer_name = 'Airbus Helicopters' WHERE upper(manufacturer_name) = 'AIRBUS HELICOPTERS';
UPDATE aircraft_models SET manufacturer_name = 'Cirrus' WHERE upper(manufacturer_name) IN ('CIRRUS DESIGN CORPORATION', 'CIRRUS DESIGN CORP', 'CIRRUS');
UPDATE aircraft_models SET manufacturer_name = 'Bell' WHERE upper(manufacturer_name) IN ('BELL HELICOPTER TEXTRON', 'BELL TEXTRON INC', 'BELL');
UPDATE aircraft_models SET manufacturer_name = 'Robinson' WHERE upper(manufacturer_name) IN ('ROBINSON HELICOPTER COMPANY', 'ROBINSON HELICOPTER CO', 'ROBINSON HELICOPTER', 'ROBINSON');
UPDATE aircraft_models SET manufacturer_name = 'Mooney' WHERE upper(manufacturer_name) IN ('MOONEY INTERNATIONAL CORP', 'MOONEY AIRCRAFT CORP', 'MOONEY');
UPDATE aircraft_models SET manufacturer_name = 'Textron Aviation' WHERE upper(manufacturer_name) IN ('TEXTRON AVIATION INC', 'TEXTRON AVIATION');
UPDATE aircraft_models SET manufacturer_name = 'Bombardier' WHERE upper(manufacturer_name) IN ('BOMBARDIER INC CANADAIR', 'BOMBARDIER AEROSPACE', 'BOMBARDIER INC', 'BOMBARDIER');
UPDATE aircraft_models SET manufacturer_name = 'Gulfstream' WHERE upper(manufacturer_name) IN ('GULFSTREAM AEROSPACE CORP', 'GULFSTREAM AEROSPACE', 'GULFSTREAM');
UPDATE aircraft_models SET manufacturer_name = 'Northrop Grumman' WHERE upper(manufacturer_name) = 'NORTHROP GRUMMAN';
UPDATE aircraft_models SET manufacturer_name = 'Grumman' WHERE upper(manufacturer_name) IN ('GRUMMAN AMERICAN AVN CORP', 'GRUMMAN AIRCRAFT ENG CORP', 'GRUMMAN');
UPDATE aircraft_models SET manufacturer_name = 'Embraer' WHERE upper(manufacturer_name) IN ('EMBRAER EMPRESA BRASILEIRA DE', 'EMBRAER S A', 'EMBRAER SA', 'EMBRAER');
UPDATE aircraft_models SET manufacturer_name = 'Diamond' WHERE upper(manufacturer_name) IN ('DIAMOND AIRCRAFT INDUSTRIES', 'DIAMOND AIRCRAFT IND', 'DIAMOND AIRCRAFT', 'DIAMOND');
UPDATE aircraft_models SET manufacturer_name = 'Maule' WHERE upper(manufacturer_name) IN ('MAULE AEROSPACE TECHNOLOGY INC', 'MAULE AEROSPACE TECHNOLOGY', 'MAULE AIR INC', 'MAULE');
UPDATE aircraft_models SET manufacturer_name = 'Socata' WHERE upper(manufacturer_name) IN ('SOCATA GROUP AEROSPATIALE', 'EADS SOCATA', 'SOCATA');
UPDATE aircraft_models SET manufacturer_name = 'Pilatus' WHERE upper(manufacturer_name) IN ('PILATUS FLUGZEUGWERKE AG', 'PILATUS AIRCRAFT LTD', 'PILATUS');
UPDATE aircraft_models SET manufacturer_name = 'Dassault' WHERE upper(manufacturer_name) IN ('AVIONS MARCEL DASSAULT', 'DASSAULT BREGUET', 'DASSAULT FALCON', 'DASSAULT AVIATION', 'DASSAULT');
UPDATE aircraft_models SET manufacturer_name = 'Sikorsky' WHERE upper(manufacturer_name) IN ('SIKORSKY AIRCRAFT CORP', 'SIKORSKY AIRCRAFT', 'SIKORSKY');
UPDATE aircraft_models SET manufacturer_name = 'Raytheon' WHERE upper(manufacturer_name) IN ('RAYTHEON AIRCRAFT COMPANY', 'RAYTHEON AIRCRAFT CO', 'RAYTHEON');
UPDATE aircraft_models SET manufacturer_name = 'McDonnell Douglas' WHERE upper(manufacturer_name) IN ('MCDONNELL DOUGLAS HELICOPTER', 'MCDONNELL DOUGLAS CORP', 'MCDONNELL DOUGLAS');
UPDATE aircraft_models SET manufacturer_name = 'Lockheed Martin' WHERE upper(manufacturer_name) IN ('LOCKHEED MARTIN CORP', 'LOCKHEED MARTIN');
UPDATE aircraft_models SET manufacturer_name = 'Lockheed' WHERE upper(manufacturer_name) = 'LOCKHEED';
UPDATE aircraft_models SET manufacturer_name = 'De Havilland Canada' WHERE upper(manufacturer_name) IN ('DE HAVILLAND AIRCRAFT OF CANADA', 'DE HAVILLAND CANADA');
UPDATE aircraft_models SET manufacturer_name = 'De Havilland' WHERE upper(manufacturer_name) = 'DE HAVILLAND';
UPDATE aircraft_models SET manufacturer_name = 'Eurocopter' WHERE upper(manufacturer_name) IN ('EUROCOPTER FRANCE', 'EUROCOPTER DEUTSCHLAND', 'EUROCOPTER');
UPDATE aircraft_models SET manufacturer_name = 'Aerospatiale' WHERE upper(manufacturer_name) = 'AEROSPATIALE';
UPDATE aircraft_models SET manufacturer_name = 'Learjet' WHERE upper(manufacturer_name) IN ('LEARJET INC', 'LEARJET');
UPDATE aircraft_models SET manufacturer_name = 'Fairchild' WHERE upper(manufacturer_name) IN ('FAIRCHILD INDUSTRIES INC', 'FAIRCHILD AIRCRAFT INC', 'FAIRCHILD');
UPDATE aircraft_models SET manufacturer_name = 'American Champion' WHERE upper(manufacturer_name) IN ('AMERICAN CHAMPION AIRCRAFT CORP', 'AMERICAN CHAMPION AIRCRAFT', 'AMERICAN CHAMPION');
UPDATE aircraft_models SET manufacturer_name = 'Schempp-Hirth' WHERE upper(manufacturer_name) IN ('SCHEMPP-HIRTH FLUGZEUGBAU GMBH', 'SCHEMPP-HIRTH');
UPDATE aircraft_models SET manufacturer_name = 'Schleicher' WHERE upper(manufacturer_name) IN ('ALEXANDER SCHLEICHER GMBH', 'ALEXANDER SCHLEICHER');
UPDATE aircraft_models SET manufacturer_name = 'DG Flugzeugbau' WHERE upper(manufacturer_name) IN ('DG FLUGZEUGBAU GMBH', 'DG FLUGZEUGBAU');
UPDATE aircraft_models SET manufacturer_name = 'Rolladen-Schneider' WHERE upper(manufacturer_name) IN ('ROLLADEN-SCHNEIDER FLUGZEUGBAU', 'ROLLADEN-SCHNEIDER');
