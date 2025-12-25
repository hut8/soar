-- Revert registration NULL values back to empty strings and make NOT NULL again
UPDATE aircraft SET registration = '' WHERE registration IS NULL;

ALTER TABLE aircraft ALTER COLUMN registration SET NOT NULL;
