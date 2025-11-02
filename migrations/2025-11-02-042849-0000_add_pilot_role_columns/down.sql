-- Remove role columns from pilots table
ALTER TABLE pilots DROP COLUMN is_examiner;
ALTER TABLE pilots DROP COLUMN is_tow_pilot;
ALTER TABLE pilots DROP COLUMN is_instructor;
