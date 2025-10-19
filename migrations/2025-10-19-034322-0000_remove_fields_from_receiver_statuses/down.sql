-- Reverse: Add back created_at, updated_at, and raw_data columns to receiver_statuses table
ALTER TABLE receiver_statuses ADD COLUMN created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL;
ALTER TABLE receiver_statuses ADD COLUMN updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL;
ALTER TABLE receiver_statuses ADD COLUMN raw_data TEXT NOT NULL DEFAULT '';
