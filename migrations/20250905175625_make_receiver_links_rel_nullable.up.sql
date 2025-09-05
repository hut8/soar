-- Make the rel column in receivers_links table nullable
ALTER TABLE receivers_links ALTER COLUMN rel DROP NOT NULL;