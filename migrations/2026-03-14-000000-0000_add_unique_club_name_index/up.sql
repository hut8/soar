-- safety-assured:start
-- Add case-insensitive unique constraint on club names
CREATE UNIQUE INDEX idx_clubs_name_unique ON clubs (UPPER(name));
-- safety-assured:end
