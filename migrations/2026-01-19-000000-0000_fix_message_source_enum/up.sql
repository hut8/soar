-- Fix message_source enum: rename ogn->aprs, adsb->beast, and add sbs
--
-- The previous migration incorrectly renamed the values. This migration:
-- - Renames 'ogn' back to 'aprs' (OGN uses APRS protocol)
-- - Renames 'adsb' to 'beast' (Beast is the binary protocol format)
-- - Adds 'sbs' (SBS-1 BaseStation CSV format)
--
-- Using ALTER TYPE ... RENAME VALUE (PostgreSQL 10+) for instant metadata-only changes.
-- No data migration needed - this is O(1) regardless of table size.

-- Rename existing values (instant - metadata only)
ALTER TYPE message_source RENAME VALUE 'ogn' TO 'aprs';
ALTER TYPE message_source RENAME VALUE 'adsb' TO 'beast';

-- Add new value (instant - metadata only)
ALTER TYPE message_source ADD VALUE 'sbs';

-- Update default for new rows
ALTER TABLE raw_messages ALTER COLUMN source SET DEFAULT 'aprs'::message_source;

COMMENT ON COLUMN raw_messages.source IS 'Protocol source: aprs (APRS/OGN text), beast (ADS-B Beast binary), or sbs (SBS-1 BaseStation CSV)';
