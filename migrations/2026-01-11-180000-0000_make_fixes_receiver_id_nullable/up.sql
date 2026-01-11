-- Make receiver_id nullable on fixes table
-- ADS-B/Beast and SBS data sources don't have receiver information,
-- so receiver_id should be NULL for those fixes

ALTER TABLE fixes ALTER COLUMN receiver_id DROP NOT NULL;
