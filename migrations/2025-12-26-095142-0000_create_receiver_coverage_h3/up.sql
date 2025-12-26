-- Receiver coverage aggregated by H3 hexagons
-- This table tracks position fix coverage by receiver, time, and altitude
CREATE TABLE receiver_coverage_h3 (
    -- H3 index stored as BIGINT for efficient indexing
    -- H3 indexes at res 6-8 fit comfortably in 64-bit signed integer
    h3_index BIGINT NOT NULL,

    -- H3 resolution (6, 7, or 8)
    resolution SMALLINT NOT NULL,

    -- Receiver that provided coverage
    receiver_id UUID NOT NULL,

    -- Time bucket for coverage (daily aggregation)
    date DATE NOT NULL,

    -- Coverage statistics
    fix_count INTEGER NOT NULL DEFAULT 0,
    first_seen_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,

    -- Altitude statistics (enables altitude filtering)
    min_altitude_msl_feet INTEGER,
    max_altitude_msl_feet INTEGER,
    avg_altitude_msl_feet INTEGER,

    -- Metadata
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (h3_index, resolution, receiver_id, date),
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE
);

-- Indexes for efficient querying

-- Index for querying by receiver and date
CREATE INDEX idx_coverage_h3_receiver_date
    ON receiver_coverage_h3 (receiver_id, date DESC);

-- Index for querying by resolution and date
CREATE INDEX idx_coverage_h3_resolution_date
    ON receiver_coverage_h3 (resolution, date DESC);

-- B-tree index on h3_index for range queries (bounding box lookups)
CREATE INDEX idx_coverage_h3_index
    ON receiver_coverage_h3 (h3_index, resolution);

-- Table and column comments
COMMENT ON TABLE receiver_coverage_h3 IS
    'Aggregated receiver coverage using Uber H3 hexagonal spatial indexes.
     Updated daily via aggregate-coverage command. Supports multiple resolutions for zoom-based visualization.';

COMMENT ON COLUMN receiver_coverage_h3.h3_index IS
    'H3 cell index stored as BIGINT. Convert to/from using h3o Rust crate.';

COMMENT ON COLUMN receiver_coverage_h3.resolution IS
    'H3 resolution level. Supported: 6 (~36km²), 7 (~5km²), 8 (~0.7km²).';
