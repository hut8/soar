-- Index to efficiently find aircraft that need merging
CREATE INDEX CONCURRENTLY idx_aircraft_pending_registration ON aircraft (pending_registration)
    WHERE pending_registration IS NOT NULL;
