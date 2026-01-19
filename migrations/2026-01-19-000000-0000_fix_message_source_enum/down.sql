-- Revert message_source enum: aprs->ogn, beast->adsb, remove sbs
--
-- WARNING: This down migration cannot remove the 'sbs' enum value because
-- PostgreSQL does not support DROP VALUE from enums. The 'sbs' value will
-- remain in the enum but should not be used after rollback.
--
-- If there are any rows with source='sbs', this migration will fail.
-- You must first update or delete those rows before rolling back.

-- Rename values back (instant - metadata only)
ALTER TYPE message_source RENAME VALUE 'aprs' TO 'ogn';
ALTER TYPE message_source RENAME VALUE 'beast' TO 'adsb';

-- Note: Cannot remove 'sbs' from enum - PostgreSQL limitation
-- The value will remain but be unused

-- Update default back
ALTER TABLE raw_messages ALTER COLUMN source SET DEFAULT 'ogn'::message_source;

COMMENT ON COLUMN raw_messages.source IS 'Protocol source: ogn or adsb';
