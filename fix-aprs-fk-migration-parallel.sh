#!/bin/bash
#
# Parallel APRS Messages Foreign Key Migration
#
# This script speeds up the migration by:
# 1. Dropping indexes before updates
# 2. Updating each partition in parallel
# 3. Rebuilding indexes concurrently after
#
# Usage: ./fix-aprs-fk-migration-parallel.sh
#

set -e
set -u
set -o pipefail

# Configuration
DB_USER="soar"
DB_NAME="soar"
MAX_PARALLEL_JOBS=8  # Adjust based on CPU count (use nproc to check)
LOG_DIR="/tmp/migration-logs-$(date +%Y%m%d-%H%M%S)"

# Color codes
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "\n${BLUE}==>${NC} ${BLUE}$1${NC}\n"
}

# Create log directory
mkdir -p "$LOG_DIR"
log_info "Logs will be written to: $LOG_DIR"

# ============================================================================
# STEP 1: Save current PostgreSQL settings and optimize for bulk operations
# ============================================================================
log_step "Step 1: Optimizing PostgreSQL settings for bulk operations"

# Save current settings
ORIGINAL_SETTINGS=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
    SELECT json_build_object(
        'synchronous_commit', current_setting('synchronous_commit'),
        'maintenance_work_mem', current_setting('maintenance_work_mem'),
        'max_wal_size', current_setting('max_wal_size'),
        'checkpoint_timeout', current_setting('checkpoint_timeout'),
        'wal_buffers', current_setting('wal_buffers')
    );
")

log_info "Current settings saved: $ORIGINAL_SETTINGS"

# Apply optimized settings for bulk operations
# Use ALTER SYSTEM for server-wide settings (will persist until we reset them)
psql -U "$DB_USER" -d "$DB_NAME" << 'EOF'
-- Server-wide settings (require reload)
ALTER SYSTEM SET max_wal_size = '16GB';
ALTER SYSTEM SET checkpoint_timeout = '30min';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;

-- Reload configuration to apply changes
SELECT pg_reload_conf();
EOF

log_info "Server settings updated and configuration reloaded"

# Show new settings
psql -U "$DB_USER" -d "$DB_NAME" -c "
    SELECT name, setting, unit
    FROM pg_settings
    WHERE name IN ('max_wal_size', 'checkpoint_timeout', 'checkpoint_completion_target')
    ORDER BY name;
" | tee "$LOG_DIR/postgres_settings.log"

# Disable autovacuum on partition leaf nodes
log_info "Disabling autovacuum on all partition leaf nodes..."

psql -U "$DB_USER" -d "$DB_NAME" << 'EOF'
-- Disable autovacuum on all fixes partitions
DO $$
DECLARE
    partition_name text;
BEGIN
    FOR partition_name IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'fixes_p%'
    LOOP
        EXECUTE format('ALTER TABLE %I SET (autovacuum_enabled = false)', partition_name);
        RAISE NOTICE 'Disabled autovacuum on %', partition_name;
    END LOOP;
END $$;

-- Disable autovacuum on all aprs_messages partitions
DO $$
DECLARE
    partition_name text;
BEGIN
    FOR partition_name IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'aprs_messages_p%'
    LOOP
        EXECUTE format('ALTER TABLE %I SET (autovacuum_enabled = false)', partition_name);
        RAISE NOTICE 'Disabled autovacuum on %', partition_name;
    END LOOP;
END $$;

-- Check if receiver_statuses is partitioned and disable accordingly
DO $$
DECLARE
    partition_name text;
    partition_count int;
BEGIN
    SELECT COUNT(*) INTO partition_count
    FROM pg_tables
    WHERE schemaname = 'public'
      AND tablename LIKE 'receiver_statuses_p%';

    IF partition_count > 0 THEN
        -- receiver_statuses is partitioned
        FOR partition_name IN
            SELECT tablename
            FROM pg_tables
            WHERE schemaname = 'public'
              AND tablename LIKE 'receiver_statuses_p%'
        LOOP
            EXECUTE format('ALTER TABLE %I SET (autovacuum_enabled = false)', partition_name);
            RAISE NOTICE 'Disabled autovacuum on %', partition_name;
        END LOOP;
    ELSE
        -- receiver_statuses is not partitioned
        ALTER TABLE receiver_statuses SET (autovacuum_enabled = false);
        RAISE NOTICE 'Disabled autovacuum on receiver_statuses';
    END IF;
END $$;
EOF

log_info "Autovacuum disabled on all partitions"

# ============================================================================
# STEP 2: Drop existing FK constraints
# ============================================================================
log_step "Step 2: Dropping existing FK constraints"

psql -U "$DB_USER" -d "$DB_NAME" << 'EOF'
-- Drop old FK constraints
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_aprs_message_id_fkey;
ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS receiver_statuses_aprs_message_id_fkey;

-- Drop any partition-specific constraints
DO $$
DECLARE
    constraint_rec RECORD;
BEGIN
    FOR constraint_rec IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid = 'fixes'::regclass
          AND conname LIKE 'fixes_aprs_message_id_%_fkey%'
    LOOP
        EXECUTE 'ALTER TABLE fixes DROP CONSTRAINT IF EXISTS ' || quote_ident(constraint_rec.conname);
        RAISE NOTICE 'Dropped constraint: %', constraint_rec.conname;
    END LOOP;

    FOR constraint_rec IN
        SELECT conname
        FROM pg_constraint
        WHERE conrelid = 'receiver_statuses'::regclass
          AND conname LIKE 'receiver_statuses_aprs_message_id_%_fkey%'
    LOOP
        EXECUTE 'ALTER TABLE receiver_statuses DROP CONSTRAINT IF EXISTS ' || quote_ident(constraint_rec.conname);
        RAISE NOTICE 'Dropped constraint: %', constraint_rec.conname;
    END LOOP;
END $$;
EOF

log_info "FK constraints dropped"

# ============================================================================
# STEP 3: Drop indexes from fixes table
# ============================================================================
log_step "Step 3: Dropping indexes (will rebuild later)"

# Note: Cannot use CONCURRENTLY with partitioned indexes
# Must drop without CONCURRENTLY (fast anyway, just a metadata change)
psql -U "$DB_USER" -d "$DB_NAME" << 'EOF'
DROP INDEX IF EXISTS idx_fixes_device_received_at;
DROP INDEX IF EXISTS idx_fixes_location_geom;
DROP INDEX IF EXISTS idx_fixes_location;
DROP INDEX IF EXISTS idx_fixes_source;
EOF

log_info "Indexes dropped"

# ============================================================================
# STEP 4: Get list of partitions
# ============================================================================
log_step "Step 4: Getting list of partitions"

FIXES_PARTITIONS=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
    SELECT tablename
    FROM pg_tables
    WHERE tablename LIKE 'fixes_p%'
    ORDER BY tablename;
" | tr -d ' ')

PARTITION_COUNT=$(echo "$FIXES_PARTITIONS" | wc -l)
log_info "Found $PARTITION_COUNT fixes partitions"

# ============================================================================
# STEP 5: Update fixes partitions in parallel
# ============================================================================
log_step "Step 5: Updating fixes partitions in parallel (8 workers)"

# Function to update a single partition
update_fixes_partition() {
    local partition=$1
    local log_file="$LOG_DIR/fixes_${partition}.log"

    {
        echo "Starting update for $partition at $(date)"

        # Count rows to update
        ROWS_TO_UPDATE=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
            SELECT COUNT(*)
            FROM $partition f
            JOIN aprs_messages am ON f.aprs_message_id = am.id
            WHERE f.received_at != am.received_at;
        " | tr -d ' ')

        echo "Rows to update in $partition: $ROWS_TO_UPDATE"

        if [ "$ROWS_TO_UPDATE" -gt 0 ]; then
            # Update the partition with optimized session settings
            psql -U "$DB_USER" -d "$DB_NAME" << EOF_UPDATE
                -- Optimize this session for bulk updates
                SET synchronous_commit = off;
                SET work_mem = '256MB';

                -- Update the partition
                UPDATE $partition f
                SET received_at = am.received_at
                FROM aprs_messages am
                WHERE f.aprs_message_id = am.id
                  AND f.received_at != am.received_at;
EOF_UPDATE
            echo "Updated $ROWS_TO_UPDATE rows in $partition at $(date)"
        else
            echo "No rows to update in $partition"
        fi

        echo "Completed $partition at $(date)"
    } > "$log_file" 2>&1

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $partition (updated $ROWS_TO_UPDATE rows)"
    else
        echo -e "${RED}✗${NC} $partition (FAILED - see $log_file)"
        return 1
    fi
}

export -f update_fixes_partition
export DB_USER DB_NAME LOG_DIR GREEN RED NC

# Run updates in parallel with limited concurrency
echo "$FIXES_PARTITIONS" | xargs -P "$MAX_PARALLEL_JOBS" -I {} bash -c 'update_fixes_partition "$@"' _ {}

log_info "All fixes partitions updated"

# ============================================================================
# STEP 6: Update receiver_statuses
# ============================================================================
log_step "Step 6: Updating receiver_statuses"

# Check if receiver_statuses is partitioned
IS_PARTITIONED=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
    SELECT COUNT(*)
    FROM pg_tables
    WHERE tablename LIKE 'receiver_statuses_p%';
" | tr -d ' ')

if [ "$IS_PARTITIONED" -gt 0 ]; then
    log_info "receiver_statuses is partitioned - updating partitions in parallel"

    RECEIVER_PARTITIONS=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
        SELECT tablename
        FROM pg_tables
        WHERE tablename LIKE 'receiver_statuses_p%'
        ORDER BY tablename;
    " | tr -d ' ')

    update_receiver_partition() {
        local partition=$1
        local log_file="$LOG_DIR/receiver_${partition}.log"

        {
            echo "Starting update for $partition at $(date)"

            ROWS_TO_UPDATE=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
                SELECT COUNT(*)
                FROM $partition rs
                JOIN aprs_messages am ON rs.aprs_message_id = am.id
                WHERE rs.received_at != am.received_at;
            " | tr -d ' ')

            echo "Rows to update in $partition: $ROWS_TO_UPDATE"

            if [ "$ROWS_TO_UPDATE" -gt 0 ]; then
                psql -U "$DB_USER" -d "$DB_NAME" -c "
                    UPDATE $partition rs
                    SET received_at = am.received_at
                    FROM aprs_messages am
                    WHERE rs.aprs_message_id = am.id
                      AND rs.received_at != am.received_at;
                "
                echo "Updated $ROWS_TO_UPDATE rows in $partition at $(date)"
            fi

            echo "Completed $partition at $(date)"
        } > "$log_file" 2>&1

        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✓${NC} $partition"
        else
            echo -e "${RED}✗${NC} $partition (FAILED)"
            return 1
        fi
    }

    export -f update_receiver_partition
    echo "$RECEIVER_PARTITIONS" | xargs -P "$MAX_PARALLEL_JOBS" -I {} bash -c 'update_receiver_partition "$@"' _ {}
else
    log_info "receiver_statuses is not partitioned - updating as single table"

    ROWS_TO_UPDATE=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
        SELECT COUNT(*)
        FROM receiver_statuses rs
        JOIN aprs_messages am ON rs.aprs_message_id = am.id
        WHERE rs.received_at != am.received_at;
    " | tr -d ' ')

    log_info "Rows to update in receiver_statuses: $ROWS_TO_UPDATE"

    if [ "$ROWS_TO_UPDATE" -gt 0 ]; then
        psql -U "$DB_USER" -d "$DB_NAME" -c "
            UPDATE receiver_statuses rs
            SET received_at = am.received_at
            FROM aprs_messages am
            WHERE rs.aprs_message_id = am.id
              AND rs.received_at != am.received_at;
        " | tee "$LOG_DIR/receiver_statuses.log"
    else
        log_info "No rows to update in receiver_statuses"
    fi
fi

log_info "receiver_statuses updated"

# ============================================================================
# STEP 7: Add FK constraints
# ============================================================================
log_step "Step 7: Adding composite FK constraints"
log_info "If there are any orphaned rows, this will fail (as expected)"

psql -U "$DB_USER" -d "$DB_NAME" << 'EOF'
ALTER TABLE fixes
    ADD CONSTRAINT fixes_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id, received_at)
    REFERENCES aprs_messages(id, received_at);

ALTER TABLE receiver_statuses
    ADD CONSTRAINT receiver_statuses_aprs_message_id_fkey
    FOREIGN KEY (aprs_message_id, received_at)
    REFERENCES aprs_messages(id, received_at);
EOF

log_info "FK constraints added"

# ============================================================================
# STEP 8: Rebuild indexes in parallel
# ============================================================================
log_step "Step 8: Rebuilding indexes in parallel"

# Note: Cannot use CONCURRENTLY with partitioned indexes
# Creating on parent table automatically creates on all partitions
# Run each in background for parallelism with optimized settings

# Helper function to create index with optimized settings
create_index() {
    local index_sql="$1"
    local log_file="$2"

    psql -U "$DB_USER" -d "$DB_NAME" > "$log_file" 2>&1 << EOF_INDEX
        -- Optimize for index creation
        SET maintenance_work_mem = '2GB';
        SET max_parallel_maintenance_workers = 4;
        SET synchronous_commit = off;

        -- Create the index
        $index_sql
EOF_INDEX
}

export -f create_index
export DB_USER DB_NAME

# Start all index creations in parallel
create_index "CREATE INDEX idx_fixes_device_received_at ON fixes(device_id, received_at DESC);" "$LOG_DIR/idx_device.log" &
create_index "CREATE INDEX idx_fixes_location_geom USING GIST (location_geom);" "$LOG_DIR/idx_geom.log" &
create_index "CREATE INDEX idx_fixes_location USING GIST (location);" "$LOG_DIR/idx_location.log" &
create_index "CREATE INDEX idx_fixes_source ON fixes(source);" "$LOG_DIR/idx_source.log" &

log_info "Index creation started in background (4 indexes building in parallel)..."
log_info "Using maintenance_work_mem=2GB and max_parallel_maintenance_workers=4 per index"
log_info "This may take a while for large tables. Check logs in $LOG_DIR/idx_*.log"

# Wait for all index rebuilds
wait
log_info "Indexes rebuilt"

# ============================================================================
# STEP 9: Re-enable autovacuum and restore PostgreSQL settings
# ============================================================================
log_step "Step 9: Re-enabling autovacuum on all partitions"

psql -U "$DB_USER" -d "$DB_NAME" << 'EOF'
-- Re-enable autovacuum on all fixes partitions
DO $$
DECLARE
    partition_name text;
BEGIN
    FOR partition_name IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'fixes_p%'
    LOOP
        EXECUTE format('ALTER TABLE %I SET (autovacuum_enabled = true)', partition_name);
        RAISE NOTICE 'Re-enabled autovacuum on %', partition_name;
    END LOOP;
END $$;

-- Re-enable autovacuum on all aprs_messages partitions
DO $$
DECLARE
    partition_name text;
BEGIN
    FOR partition_name IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'aprs_messages_p%'
    LOOP
        EXECUTE format('ALTER TABLE %I SET (autovacuum_enabled = true)', partition_name);
        RAISE NOTICE 'Re-enabled autovacuum on %', partition_name;
    END LOOP;
END $$;

-- Re-enable autovacuum on receiver_statuses partitions or table
DO $$
DECLARE
    partition_name text;
    partition_count int;
BEGIN
    SELECT COUNT(*) INTO partition_count
    FROM pg_tables
    WHERE schemaname = 'public'
      AND tablename LIKE 'receiver_statuses_p%';

    IF partition_count > 0 THEN
        FOR partition_name IN
            SELECT tablename
            FROM pg_tables
            WHERE schemaname = 'public'
              AND tablename LIKE 'receiver_statuses_p%'
        LOOP
            EXECUTE format('ALTER TABLE %I SET (autovacuum_enabled = true)', partition_name);
            RAISE NOTICE 'Re-enabled autovacuum on %', partition_name;
        END LOOP;
    ELSE
        ALTER TABLE receiver_statuses SET (autovacuum_enabled = true);
        RAISE NOTICE 'Re-enabled autovacuum on receiver_statuses';
    END IF;
END $$;
EOF

log_info "Autovacuum re-enabled on all partitions"

# Restore original PostgreSQL settings
log_info "Restoring original PostgreSQL settings..."

psql -U "$DB_USER" -d "$DB_NAME" << 'EOF'
-- Reset to default values (will use postgresql.conf defaults)
ALTER SYSTEM RESET max_wal_size;
ALTER SYSTEM RESET checkpoint_timeout;
ALTER SYSTEM RESET checkpoint_completion_target;

-- Reload configuration
SELECT pg_reload_conf();
EOF

log_info "PostgreSQL settings restored to defaults"

# Optional: Run VACUUM ANALYZE to clean up dead tuples and update statistics
log_info "Running VACUUM ANALYZE on fixes (this may take a while)..."
psql -U "$DB_USER" -d "$DB_NAME" -c "VACUUM ANALYZE fixes;" > "$LOG_DIR/vacuum_fixes.log" 2>&1 &

log_info "Running VACUUM ANALYZE on receiver_statuses..."
psql -U "$DB_USER" -d "$DB_NAME" -c "VACUUM ANALYZE receiver_statuses;" > "$LOG_DIR/vacuum_receiver.log" 2>&1 &

log_info "VACUUM ANALYZE started in background (check logs for progress)"

# Wait for vacuum to complete
wait
log_info "VACUUM ANALYZE completed"

# ============================================================================
# DONE
# ============================================================================
log_step "Migration Complete!"

log_info "Summary:"
log_info "  - Logs directory: $LOG_DIR"
log_info "  - Partitions updated: $PARTITION_COUNT"
log_info "  - Check individual partition logs for details"

# Show any errors from logs
if grep -r "ERROR" "$LOG_DIR/" > /dev/null 2>&1; then
    log_warn "Some errors occurred - check logs in $LOG_DIR/"
    grep -r "ERROR" "$LOG_DIR/" | head -20
else
    log_info "No errors detected"
fi

log_info "Done!"
