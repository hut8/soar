CREATE INDEX CONCURRENTLY idx_receivers_latest_packet_at
    ON receivers (latest_packet_at DESC NULLS LAST);
