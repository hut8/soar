-- Create receiver_statuses table
CREATE TABLE receiver_statuses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    receiver_id INTEGER NOT NULL REFERENCES receivers(id),
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Status fields from StatusComment
    version TEXT,
    platform TEXT,
    cpu_load DECIMAL,
    ram_free DECIMAL,
    ram_total DECIMAL,
    ntp_offset DECIMAL,
    ntp_correction DECIMAL,
    voltage DECIMAL,
    amperage DECIMAL,
    cpu_temperature DECIMAL,
    visible_senders SMALLINT,
    latency DECIMAL,
    senders SMALLINT,
    rf_correction_manual SMALLINT,
    rf_correction_automatic DECIMAL,
    noise DECIMAL,
    senders_signal_quality DECIMAL,
    senders_messages INTEGER,
    good_senders_signal_quality DECIMAL,
    good_senders SMALLINT,
    good_and_bad_senders SMALLINT,
    geoid_offset SMALLINT,
    name TEXT,
    demodulation_snr_db DECIMAL,
    ognr_pilotaware_version TEXT,
    unparsed_data TEXT,

    -- Computed lag column (milliseconds between packet timestamp and received_at)
    -- This will be computed based on the packet timestamp when inserting
    lag INTEGER,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on receiver_id for efficient queries
CREATE INDEX idx_receiver_statuses_receiver_id ON receiver_statuses(receiver_id);

-- Create index on received_at for time-based queries
CREATE INDEX idx_receiver_statuses_received_at ON receiver_statuses(received_at);

-- Create composite index for receiver + time queries
CREATE INDEX idx_receiver_statuses_receiver_received_at ON receiver_statuses(receiver_id, received_at);
