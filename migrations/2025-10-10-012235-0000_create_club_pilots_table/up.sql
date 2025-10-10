-- Create club_pilots table
CREATE TABLE club_pilots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    is_tow_pilot BOOLEAN NOT NULL DEFAULT FALSE,
    is_instructor BOOLEAN NOT NULL DEFAULT FALSE,
    is_licensed BOOLEAN NOT NULL DEFAULT FALSE,
    is_student BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on name for searching
CREATE INDEX idx_club_pilots_name ON club_pilots(last_name, first_name);

-- Create updated_at trigger
CREATE TRIGGER set_club_pilots_updated_at
    BEFORE UPDATE ON club_pilots
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
