-- Revert: Make the rel column in receivers_links table NOT NULL again
-- Note: This will fail if there are any NULL values in the rel column
ALTER TABLE receivers_links ALTER COLUMN rel SET NOT NULL;