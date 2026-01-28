#!/bin/bash
# Decompress all compressed fixes chunks before running migrations
#
# This is a pre-migration step that makes subsequent UPDATE operations
# on the fixes hypertable MUCH faster by avoiding transparent decompression.
#
# After the migration completes, the compression policy will automatically
# recompress chunks older than 7 days.
#
# Usage:
#   ./scripts/decompress-all-fixes-chunks.sh [database_name]
#   ./scripts/decompress-all-fixes-chunks.sh soar

set -e

DB="${1:-soar}"

echo "=== Decompress All Fixes Chunks ==="
echo "Database: $DB"
echo ""

# Get list of all compressed chunks
echo "Finding compressed chunks..."
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
    echo "No compressed chunks - nothing to do."
    exit 0
fi

echo ""
echo "Decompressing chunks..."

DECOMPRESS_START=$(date +%s)
DECOMP_NUM=0
for CHUNK in $CHUNKS; do
    DECOMP_NUM=$((DECOMP_NUM + 1))
    CHUNK_SHORT=$(echo "$CHUNK" | sed 's/.*\.//')
    echo "  [$DECOMP_NUM/$CHUNK_COUNT] $CHUNK_SHORT..."
    psql -d "$DB" -c "SELECT decompress_chunk('$CHUNK');" >/dev/null 2>&1
done
DECOMPRESS_END=$(date +%s)
DECOMPRESS_DURATION=$((DECOMPRESS_END - DECOMPRESS_START))

echo ""
echo "=== Complete ==="
echo "Decompressed $CHUNK_COUNT chunks in ${DECOMPRESS_DURATION}s"
echo ""
echo "You can now run the migration. Compression policy will recompress"
echo "chunks older than 7 days automatically."
