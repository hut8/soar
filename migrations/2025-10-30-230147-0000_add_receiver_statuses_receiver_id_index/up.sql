-- Add index on receiver_statuses(receiver_id) to improve query performance
-- This is a foreign key column, so queries filtering/joining on it will benefit
CREATE INDEX idx_receiver_statuses_receiver_id ON receiver_statuses (receiver_id);
