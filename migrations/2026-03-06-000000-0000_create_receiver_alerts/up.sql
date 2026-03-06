CREATE TABLE receiver_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    receiver_id UUID NOT NULL REFERENCES receivers(id) ON DELETE CASCADE,

    -- Alert conditions
    alert_on_down BOOLEAN NOT NULL DEFAULT true,
    down_after_minutes INTEGER NOT NULL DEFAULT 30,

    alert_on_high_cpu BOOLEAN NOT NULL DEFAULT false,
    cpu_threshold DECIMAL NOT NULL DEFAULT 0.9,

    alert_on_high_temperature BOOLEAN NOT NULL DEFAULT false,
    temperature_threshold_c DECIMAL NOT NULL DEFAULT 70.0,

    -- Notification delivery
    send_email BOOLEAN NOT NULL DEFAULT true,

    -- Exponential backoff state
    base_cooldown_minutes INTEGER NOT NULL DEFAULT 30,
    consecutive_alerts INTEGER NOT NULL DEFAULT 0,
    last_alerted_at TIMESTAMPTZ,
    last_condition TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (user_id, receiver_id)
);

