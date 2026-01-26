#!/bin/bash
# Manual script to update fixes.aircraft_id for aircraft deduplication
#
# This script updates fixes in batches by time range, committing after each batch.
# It can be interrupted (Ctrl+C) and restarted - it will resume from where it left off.
#
# Usage:
#   ./scripts/update_fixes_aircraft_id.sh [database_name]
#   ./scripts/update_fixes_aircraft_id.sh soar_staging
#
# Prerequisites:
#   - Migration 1 (deduplicate_aircraft_setup) must be run first
#   - The aircraft_merge_mapping table must exist

set -e

DB="${1:-soar_staging}"
# Process 6 hours at a time - balances batch size with commit frequency
HOURS_PER_BATCH=6

echo "=== Fixes Aircraft ID Update Script ==="
echo "Database: $DB"
echo "Hours per batch: $HOURS_PER_BATCH"
echo ""

# Check prerequisites
echo "Checking prerequisites..."
EXISTS=$(psql -d "$DB" -tAc "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'aircraft_merge_mapping')")
if [ "$EXISTS" != "t" ]; then
    echo "ERROR: aircraft_merge_mapping table not found. Run migration 1 first."
    exit 1
fi

# Get time range
TIME_RANGE=$(psql -d "$DB" -tAc "
    SELECT MIN(fx.received_at)::text || '|' || MAX(fx.received_at)::text
    FROM fixes fx
    JOIN aircraft_merge_mapping m ON fx.aircraft_id = m.flarm_id
")

if [ -z "$TIME_RANGE" ] || [ "$TIME_RANGE" = "|" ]; then
    echo "No fixes to update. Already complete!"
    exit 0
fi

MIN_TS=$(echo "$TIME_RANGE" | cut -d'|' -f1)
MAX_TS=$(echo "$TIME_RANGE" | cut -d'|' -f2)

echo "Time range: $MIN_TS to $MAX_TS"

# Get initial count
REMAINING=$(psql -d "$DB" -tAc "
    SELECT COUNT(*)
    FROM fixes fx
    JOIN aircraft_merge_mapping m ON fx.aircraft_id = m.flarm_id
")
echo "Fixes remaining to update: $REMAINING"
echo ""

TOTAL_UPDATED=0
BATCH_NUM=0
START_TIME=$(date +%s)

# Start from the minimum timestamp, truncated to hour
BATCH_START="$MIN_TS"

echo "Starting batched update..."
echo ""

while true; do
    BATCH_NUM=$((BATCH_NUM + 1))
    BATCH_TIME_START=$(date +%s)

    # Calculate batch end (HOURS_PER_BATCH hours later)
    BATCH_END=$(psql -d "$DB" -tAc "SELECT ('$BATCH_START'::timestamptz + interval '$HOURS_PER_BATCH hours')::text")

    # Update this batch (disable decompression limit for this session)
    UPDATED=$(psql -d "$DB" -tAc "
        SET timescaledb.max_tuples_decompressed_per_dml_transaction = 0;
        UPDATE fixes fx
        SET aircraft_id = m.icao_id
        FROM aircraft_merge_mapping m
        WHERE fx.aircraft_id = m.flarm_id
          AND fx.received_at >= '$BATCH_START'::timestamptz
          AND fx.received_at < '$BATCH_END'::timestamptz
    " | grep -oP 'UPDATE \K\d+' || echo "0")

    TOTAL_UPDATED=$((TOTAL_UPDATED + UPDATED))
    BATCH_TIME_END=$(date +%s)
    BATCH_DURATION=$((BATCH_TIME_END - BATCH_TIME_START))
    TOTAL_DURATION=$((BATCH_TIME_END - START_TIME))

    # Calculate rate
    if [ "$TOTAL_DURATION" -gt 0 ]; then
        RATE=$((TOTAL_UPDATED / TOTAL_DURATION))
    else
        RATE=0
    fi

    echo "Batch $BATCH_NUM ($BATCH_START): updated $UPDATED fixes (total: $TOTAL_UPDATED, ${BATCH_DURATION}s, ${RATE}/sec)"

    # Move to next batch
    BATCH_START="$BATCH_END"

    # Check if we've passed the max timestamp
    PAST_MAX=$(psql -d "$DB" -tAc "SELECT '$BATCH_START'::timestamptz > '$MAX_TS'::timestamptz")
    if [ "$PAST_MAX" = "t" ]; then
        break
    fi
done

END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))

echo ""
echo "=== Fixes update complete ==="
echo "Total fixes updated: $TOTAL_UPDATED"
echo "Total time: ${TOTAL_DURATION}s"
echo ""

# Verify
REMAINING=$(psql -d "$DB" -tAc "
    SELECT COUNT(*)
    FROM fixes fx
    JOIN aircraft_merge_mapping m ON fx.aircraft_id = m.flarm_id
")
echo "Remaining fixes with old aircraft_id: $REMAINING"

if [ "$REMAINING" -eq 0 ]; then
    echo ""
    echo "SUCCESS! All fixes updated."
    echo "Next: Run 'diesel migration run' to complete migration 2"
else
    echo ""
    echo "WARNING: $REMAINING fixes still need updating. Re-run this script."
fi
