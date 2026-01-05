-- Recreate the index if migration is rolled back
CREATE INDEX idx_watchlist_user_id ON watchlist(user_id);
