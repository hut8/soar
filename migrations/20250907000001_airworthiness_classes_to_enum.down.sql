-- Add down migration script here
-- =========================================================
-- Restore airworthiness_classes table from enum type
-- =========================================================

-- 1. Recreate the airworthiness_classes table
CREATE TABLE airworthiness_classes (
  code CHAR(1) PRIMARY KEY,
  description TEXT NOT NULL
);

-- 2. Insert the original values
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

-- 3. Add the old column back
ALTER TABLE aircraft_registrations 
ADD COLUMN airworthiness_class_code CHAR(1) REFERENCES airworthiness_classes(code);

-- 4. Update the old column based on enum values
UPDATE aircraft_registrations 
SET airworthiness_class_code = CASE 
    WHEN airworthiness_class = 'Standard' THEN '1'
    WHEN airworthiness_class = 'Limited' THEN '2'
    WHEN airworthiness_class = 'Restricted' THEN '3'
    WHEN airworthiness_class = 'Experimental' THEN '4'
    WHEN airworthiness_class = 'Provisional' THEN '5'
    WHEN airworthiness_class = 'Multiple' THEN '6'
    WHEN airworthiness_class = 'Primary' THEN '7'
    WHEN airworthiness_class = 'Special Flight Permit' THEN '8'
    WHEN airworthiness_class = 'Light Sport' THEN '9'
END;

-- 5. Drop the enum column
ALTER TABLE aircraft_registrations 
DROP COLUMN airworthiness_class;

-- 6. Drop the enum type
DROP TYPE airworthiness_class;

-- 7. Restore the original index
DROP INDEX aircraft_registrations_aw_class_idx;
CREATE INDEX aircraft_registrations_aw_class_idx ON aircraft_registrations (airworthiness_class_code, type_aircraft_code);