-- safety-assured:start
CREATE TABLE club_join_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    club_id UUID NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    message TEXT,
    reviewed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

CREATE INDEX idx_club_join_requests_club_id ON club_join_requests (club_id);
CREATE INDEX idx_club_join_requests_user_id ON club_join_requests (user_id);
CREATE INDEX idx_club_join_requests_status ON club_join_requests (status);

-- Only one pending request per user per club
CREATE UNIQUE INDEX idx_club_join_requests_pending ON club_join_requests (user_id, club_id) WHERE status = 'pending';
-- safety-assured:end
