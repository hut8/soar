#!/bin/bash
# Fast approach: Decompress chunks first, then bulk update, then recompress
#
# This is MUCH faster than transparent decompression because:
# 1. Decompression is done once per chunk (not per row)
# 2. UPDATE on uncompressed data is fast
# 3. Recompression is done once at the end
#
# Usage:
#   ./scripts/update_fixes_decompress_first.sh [database_name]

set -e

DB="${1:-soar_staging}"

echo "=== Fast Fixes Update (Decompress First) ==="
echo "Database: $DB"
echo ""

# Check prerequisites
echo "Checking prerequisites..."
EXISTS=$(psql -d "$DB" -tAc "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'aircraft_merge_mapping')")
if [ "$EXISTS" != "t" ]; then
    echo "ERROR: aircraft_merge_mapping table not found. Run migration 1 first."
    exit 1
fi

# Get count from mapping table (fast)
AIRCRAFT_COUNT=$(psql -d "$DB" -tAc "SELECT COUNT(*) FROM aircraft_merge_mapping")
echo "Aircraft to merge: $AIRCRAFT_COUNT"

echo ""
echo "Step 1: Get all compressed chunks..."

# Get list of ALL compressed chunks (faster than checking which have affected rows)
# Use fully qualified name: schema.table
CHUNKS=$(psql -d "$DB" -tAc "
    SELECT chunk_schema || '.' || chunk_name
    FROM timescaledb_information.chunks
    WHERE hypertable_name = 'fixes'
      AND is_compressed = true
    ORDER BY range_start
")

CHUNK_COUNT=$(echo "$CHUNKS" | grep -c . || echo "0")
echo "Found $CHUNK_COUNT compressed chunks"

if [ "$CHUNK_COUNT" -eq 0 ]; then
    echo "No compressed chunks found - trying direct update..."

    START_TIME=$(date +%s)
    UPDATED=$(psql -d "$DB" -tAc "
        UPDATE fixes fx
        SET aircraft_id = m.icao_id
        FROM aircraft_merge_mapping m
        WHERE fx.aircraft_id = m.flarm_id
    " | grep -oP 'UPDATE \K\d+' || echo "0")
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))

    echo "Updated $UPDATED fixes in ${DURATION}s"
    exit 0
fi

echo ""
echo "Step 2: Decompress all chunks..."

DECOMPRESS_START=$(date +%s)
DECOMP_NUM=0
for CHUNK in $CHUNKS; do
    DECOMP_NUM=$((DECOMP_NUM + 1))
    echo "  Decompressing $DECOMP_NUM/$CHUNK_COUNT: $CHUNK..."
    psql -d "$DB" -c "SELECT decompress_chunk('$CHUNK');" >/dev/null 2>&1
done
DECOMPRESS_END=$(date +%s)
DECOMPRESS_DURATION=$((DECOMPRESS_END - DECOMPRESS_START))
echo "Decompression complete in ${DECOMPRESS_DURATION}s"

echo ""
echo "Step 3: Bulk update (uncompressed data)..."

UPDATE_START=$(date +%s)
# Also disable decompression limit in case some chunks were recompressed by background job
UPDATED=$(psql -d "$DB" -tAc "
    SET timescaledb.max_tuples_decompressed_per_dml_transaction = 0;
    UPDATE fixes fx
    SET aircraft_id = m.icao_id
    FROM aircraft_merge_mapping m
    WHERE fx.aircraft_id = m.flarm_id
" | grep -oP 'UPDATE \K\d+' || echo "0")
UPDATE_END=$(date +%s)
UPDATE_DURATION=$((UPDATE_END - UPDATE_START))

echo "Updated $UPDATED fixes in ${UPDATE_DURATION}s"
if [ "$UPDATE_DURATION" -gt 0 ]; then
    RATE=$((UPDATED / UPDATE_DURATION))
    echo "Rate: ${RATE}/sec"
fi

# Skip recompression - let the compression policy handle it
# This minimizes downtime for soar-run
echo ""
echo "Skipping recompression - compression policy will handle this automatically."

echo ""
echo "=== Summary ==="
TOTAL_DURATION=$((UPDATE_END - DECOMPRESS_START))
echo "Fixes updated: $UPDATED"
echo "Chunks processed: $CHUNK_COUNT"
echo "Decompress time: ${DECOMPRESS_DURATION}s"
echo "Update time: ${UPDATE_DURATION}s"
echo "Compress time: ${COMPRESS_DURATION}s"
echo "Total time: ${TOTAL_DURATION}s"

# Verify with quick check on mapping table
REMAINING=$(psql -d "$DB" -tAc "
    SELECT COUNT(*)
    FROM aircraft_merge_mapping m
    WHERE EXISTS (
        SELECT 1 FROM fixes fx
        WHERE fx.aircraft_id = m.flarm_id
        LIMIT 1
    )
")
echo ""
echo "Aircraft IDs still needing fixes updated: $REMAINING"

if [ "$REMAINING" -eq 0 ]; then
    echo ""
    echo "SUCCESS! All fixes updated."
    echo "Next: Run 'diesel migration run' to complete migration 2"
else
    echo ""
    echo "WARNING: Some fixes may still need updating. Check manually."
fi
