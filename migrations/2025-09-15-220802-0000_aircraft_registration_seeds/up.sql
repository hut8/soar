-- Seed data for aircraft registration related tables

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
  (0,'None'),
  (1,'Reciprocating'),
  (2,'Turbo-prop'),
  (3,'Turbo-shaft'),
  (4,'Turbo-jet'),
  (5,'Turbo-fan'),
  (6,'Ramjet'),
  (7,'2-Cycle'),
  (8,'4-Cycle'),
  (9,'Unknown'),
  (10,'Electric'),
  (11,'Rotary');

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
