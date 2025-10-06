-- Remove satellite tracking columns from fixes table

ALTER TABLE fixes DROP COLUMN IF EXISTS satellites_used;
ALTER TABLE fixes DROP COLUMN IF EXISTS satellites_visible;
