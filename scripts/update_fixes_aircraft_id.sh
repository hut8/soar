#!/bin/bash
# Manual script to update fixes.aircraft_id for aircraft deduplication
#
# This script updates fixes in batches, committing after each batch.
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
BATCH_SIZE=500000

echo "=== Fixes Aircraft ID Update Script ==="
echo "Database: $DB"
echo "Batch size: $BATCH_SIZE"
echo ""

# Check prerequisites
echo "Checking prerequisites..."
EXISTS=$(psql -d "$DB" -tAc "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'aircraft_merge_mapping')")
if [ "$EXISTS" != "t" ]; then
    echo "ERROR: aircraft_merge_mapping table not found. Run migration 1 first."
    exit 1
fi

# Get initial count
REMAINING=$(psql -d "$DB" -tAc "
    SELECT COUNT(*)
    FROM fixes fx
    JOIN aircraft_merge_mapping m ON fx.aircraft_id = m.flarm_id
")
echo "Fixes remaining to update: $REMAINING"
echo ""

if [ "$REMAINING" -eq 0 ]; then
    echo "No fixes to update. Already complete!"
    exit 0
fi

TOTAL_UPDATED=0
BATCH_NUM=0
START_TIME=$(date +%s)

echo "Starting batched update..."
echo ""

while true; do
    BATCH_NUM=$((BATCH_NUM + 1))
    BATCH_START=$(date +%s)

    # Update a batch
    UPDATED=$(psql -d "$DB" -tAc "
        WITH batch AS (
            SELECT fx.ctid, m.icao_id
            FROM fixes fx
            JOIN aircraft_merge_mapping m ON fx.aircraft_id = m.flarm_id
            LIMIT $BATCH_SIZE
        )
        UPDATE fixes fx
        SET aircraft_id = batch.icao_id
        FROM batch
        WHERE fx.ctid = batch.ctid
        RETURNING 1
    " | wc -l)

    if [ "$UPDATED" -eq 0 ]; then
        break
    fi

    TOTAL_UPDATED=$((TOTAL_UPDATED + UPDATED))
    BATCH_END=$(date +%s)
    BATCH_DURATION=$((BATCH_END - BATCH_START))
    TOTAL_DURATION=$((BATCH_END - START_TIME))

    # Calculate rate
    if [ "$TOTAL_DURATION" -gt 0 ]; then
        RATE=$((TOTAL_UPDATED / TOTAL_DURATION))
    else
        RATE=0
    fi

    echo "Batch $BATCH_NUM: updated $UPDATED fixes (total: $TOTAL_UPDATED, ${BATCH_DURATION}s, ${RATE} fixes/sec)"
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
    echo "Next: Run migration 2 (deduplicate_aircraft_finish)"
else
    echo ""
    echo "WARNING: $REMAINING fixes still need updating. Re-run this script."
fi
