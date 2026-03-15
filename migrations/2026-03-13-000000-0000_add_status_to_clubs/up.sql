-- safety-assured:start
-- Add status and created_by columns to clubs table for manual club creation with approval workflow

ALTER TABLE clubs
    ADD COLUMN status TEXT NOT NULL DEFAULT 'approved'
        CHECK (status IN ('pending', 'approved', 'rejected')),
    ADD COLUMN created_by UUID REFERENCES users(id) ON DELETE SET NULL;

-- Index on status for filtering pending clubs
CREATE INDEX idx_clubs_status ON clubs (status);

-- safety-assured:end
