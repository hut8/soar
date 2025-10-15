-- Revert the column rename from 'aprs_type' back to 'destination'
ALTER TABLE fixes RENAME COLUMN aprs_type TO destination;
