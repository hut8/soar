CREATE TABLE push_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Web Push subscription fields (from PushSubscription JS API)
    endpoint TEXT NOT NULL,
    p256dh_key TEXT NOT NULL,
    auth_key TEXT NOT NULL,
    -- Notification preferences
    notify_takeoff BOOLEAN NOT NULL DEFAULT true,
    notify_landing BOOLEAN NOT NULL DEFAULT true,
    -- Device metadata
    user_agent TEXT,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Prevent duplicate subscriptions for same endpoint
    CONSTRAINT push_subscriptions_endpoint_unique UNIQUE (endpoint)
);

CREATE INDEX CONCURRENTLY idx_push_subscriptions_user_id ON push_subscriptions(user_id);
