CREATE TYPE payment_type AS ENUM (
    'tow_charge',
    'membership_dues',
    'platform_subscription',
    'other'
);

CREATE TYPE payment_status AS ENUM (
    'pending',
    'processing',
    'succeeded',
    'failed',
    'refunded',
    'canceled'
);

CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    club_id UUID REFERENCES clubs(id) ON DELETE SET NULL,
    stripe_payment_intent_id VARCHAR(255) UNIQUE,
    stripe_invoice_id VARCHAR(255),
    stripe_charge_id VARCHAR(255),
    payment_type payment_type NOT NULL,
    status payment_status NOT NULL DEFAULT 'pending',
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'usd',
    platform_fee_cents INTEGER NOT NULL DEFAULT 0 CHECK (platform_fee_cents >= 0),
    description TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    idempotency_key VARCHAR(255) UNIQUE,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- stripe_payment_intent_id and idempotency_key already have UNIQUE constraint indexes
-- Non-unique indexes are created CONCURRENTLY in a separate migration
