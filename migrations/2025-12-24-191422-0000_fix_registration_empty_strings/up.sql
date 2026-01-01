-- Make registration nullable, then convert empty strings to NULL
ALTER TABLE aircraft ALTER COLUMN registration DROP NOT NULL;

UPDATE aircraft SET registration = NULL WHERE registration = '';
