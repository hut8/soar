-- safety-assured:start
-- Add case-insensitive unique constraint on club names
CREATE UNIQUE INDEX CONCURRENTLY idx_clubs_name_unique ON clubs (UPPER(name));
-- safety-assured:end
