-- Remove received_at and lag columns from fixes table

ALTER TABLE fixes
DROP COLUMN received_at,
DROP COLUMN lag;
