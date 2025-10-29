-- Add callsign column to flights table
-- This will store the flight number/callsign (e.g., KLM33K) from APRS packets
ALTER TABLE flights ADD COLUMN callsign TEXT;

-- Add index for efficient lookups by callsign
CREATE INDEX idx_flights_callsign ON flights (callsign);
