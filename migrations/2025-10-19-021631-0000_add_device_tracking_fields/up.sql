-- Add icao_model_code to devices table
-- This is the 8-character ICAO model code sent in ADS-B packets
ALTER TABLE devices ADD COLUMN icao_model_code VARCHAR(8);

-- Add adsb_emitter_category to devices table
-- This tracks the ADS-B emitter category (A0-A7, B0-B7, C0-C5, etc.)
ALTER TABLE devices ADD COLUMN adsb_emitter_category adsb_emitter_category;

-- Create index for lookups by icao_model_code
CREATE INDEX idx_devices_icao_model_code ON devices(icao_model_code) WHERE icao_model_code IS NOT NULL;
