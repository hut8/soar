CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE server_messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    software TEXT NOT NULL,
    server_timestamp TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL,
    server_name TEXT NOT NULL,
    server_endpoint TEXT NOT NULL,
    lag INTEGER, -- milliseconds difference between received_at and server_timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_server_messages_server_timestamp ON server_messages (server_timestamp);
CREATE INDEX idx_server_messages_received_at ON server_messages (received_at);
CREATE INDEX idx_server_messages_server_name ON server_messages (server_name);