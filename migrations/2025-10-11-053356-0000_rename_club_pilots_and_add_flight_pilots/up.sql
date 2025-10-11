-- Rename club_pilots table to pilots and add club_id column
ALTER TABLE club_pilots RENAME TO pilots;

-- Add club_id column to pilots table
ALTER TABLE pilots ADD COLUMN club_id UUID REFERENCES clubs(id);

-- Remove role-related columns from pilots (they'll be in flight_pilots)
ALTER TABLE pilots DROP COLUMN is_tow_pilot;
ALTER TABLE pilots DROP COLUMN is_instructor;
ALTER TABLE pilots DROP COLUMN is_student;

-- Create flight_pilots linking table
CREATE TABLE flight_pilots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    flight_id UUID NOT NULL REFERENCES flights(id) ON DELETE CASCADE,
    pilot_id UUID NOT NULL REFERENCES pilots(id) ON DELETE CASCADE,
    is_tow_pilot BOOLEAN NOT NULL DEFAULT false,
    is_student BOOLEAN NOT NULL DEFAULT false,
    is_instructor BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(flight_id, pilot_id)
);

-- Create indexes for better query performance
CREATE INDEX idx_flight_pilots_flight_id ON flight_pilots(flight_id);
CREATE INDEX idx_flight_pilots_pilot_id ON flight_pilots(pilot_id);
CREATE INDEX idx_pilots_club_id ON pilots(club_id);
