-- Remove the 10 character limit on the surface column in runways table
-- Change from VARCHAR(10) to TEXT to allow longer surface descriptions
ALTER TABLE runways ALTER COLUMN surface TYPE TEXT;
