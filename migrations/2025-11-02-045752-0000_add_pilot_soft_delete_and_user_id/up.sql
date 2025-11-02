-- Add soft delete and user_id to pilots table
ALTER TABLE pilots ADD COLUMN deleted_at TIMESTAMPTZ;
ALTER TABLE pilots ADD COLUMN user_id UUID REFERENCES users(id);

-- Create index for soft delete queries (to efficiently filter out deleted records)
CREATE INDEX idx_pilots_deleted_at ON pilots(deleted_at) WHERE deleted_at IS NULL;
