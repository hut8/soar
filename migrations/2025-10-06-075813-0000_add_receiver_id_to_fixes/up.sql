-- Add receiver_id column to fixes table
ALTER TABLE fixes ADD COLUMN receiver_id UUID;

-- Add foreign key constraint
ALTER TABLE fixes ADD CONSTRAINT fixes_receiver_id_fkey
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE SET NULL;

-- Create index for faster lookups
CREATE INDEX idx_fixes_receiver_id ON fixes(receiver_id);
