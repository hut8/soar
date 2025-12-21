-- Recreate the data_quality_metrics_daily table
-- (from original migration 2025-11-16-015418-0000_create_analytics_tables)

CREATE TABLE data_quality_metrics_daily (
    metric_date DATE PRIMARY KEY,
    total_fixes BIGINT NOT NULL DEFAULT 0,
    fixes_with_gaps_60s INT NOT NULL DEFAULT 0,
    fixes_with_gaps_300s INT NOT NULL DEFAULT 0,
    unparsed_aprs_messages INT NOT NULL DEFAULT 0,
    flights_timed_out INT NOT NULL DEFAULT 0,
    avg_fixes_per_flight NUMERIC(10,2) NOT NULL DEFAULT 0,
    quality_score NUMERIC(5,2) NOT NULL DEFAULT 100.0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_data_quality_metrics_daily_date_desc ON data_quality_metrics_daily (metric_date DESC);
