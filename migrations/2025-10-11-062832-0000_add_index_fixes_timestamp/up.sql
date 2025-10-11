-- Add index on fixes(timestamp) for performance
CREATE INDEX idx_fixes_timestamp ON fixes(timestamp);
