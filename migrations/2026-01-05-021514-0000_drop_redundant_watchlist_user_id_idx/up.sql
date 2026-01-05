-- Drop redundant index on watchlist(user_id)
-- This index is redundant because the primary key (user_id, aircraft_id) already indexes user_id
-- as its left prefix, so queries on user_id can use the PK index directly
DROP INDEX IF EXISTS idx_watchlist_user_id;
