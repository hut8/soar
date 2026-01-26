-- Re-add the timestamp column and populate it from received_at
ALTER TABLE fixes ADD COLUMN timestamp TIMESTAMPTZ;
UPDATE fixes SET timestamp = received_at;
ALTER TABLE fixes ALTER COLUMN timestamp SET NOT NULL;
