-- Add runways_inferred column to flights table to track whether
-- runways were looked up from the database or inferred from aircraft heading

ALTER TABLE flights
ADD COLUMN IF NOT EXISTS runways_inferred BOOLEAN;

COMMENT ON COLUMN flights.runways_inferred IS 'Whether runways were inferred (true) or looked up in database (false). NULL if no runways determined.';
