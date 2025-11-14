#!/bin/bash
#
# Check if migration is needed and estimate impact
#

DB_USER="soar"
DB_NAME="soar"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}==> Checking Migration Status${NC}\n"

# Check if FK constraints exist
echo "1. Checking FK constraints..."
FK_EXISTS=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
    SELECT COUNT(*)
    FROM pg_constraint
    WHERE conname IN ('fixes_aprs_message_id_fkey', 'receiver_statuses_aprs_message_id_fkey')
      AND contype = 'f';
" | tr -d ' ')

if [ "$FK_EXISTS" -eq 2 ]; then
    echo -e "   ${GREEN}✓${NC} FK constraints already exist (migration may have already run)"
else
    echo -e "   ${YELLOW}!${NC} FK constraints missing ($FK_EXISTS of 2 found)"
fi

# Check how many rows need updates in fixes
echo ""
echo "2. Checking fixes table..."
FIXES_TO_UPDATE=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
    SELECT COUNT(*)
    FROM fixes f
    JOIN aprs_messages am ON f.aprs_message_id = am.id
    WHERE f.received_at != am.received_at;
" | tr -d ' ')

TOTAL_FIXES=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM fixes;" | tr -d ' ')

echo "   Total rows: $TOTAL_FIXES"
echo "   Rows needing timestamp sync: $FIXES_TO_UPDATE"
if [ "$FIXES_TO_UPDATE" -gt 0 ]; then
    echo -e "   ${YELLOW}!${NC} Migration needed"
else
    echo -e "   ${GREEN}✓${NC} No updates needed"
fi

# Check how many rows need updates in receiver_statuses
echo ""
echo "3. Checking receiver_statuses table..."
RECEIVER_TO_UPDATE=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
    SELECT COUNT(*)
    FROM receiver_statuses rs
    JOIN aprs_messages am ON rs.aprs_message_id = am.id
    WHERE rs.received_at != am.received_at;
" | tr -d ' ')

TOTAL_RECEIVER=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM receiver_statuses;" | tr -d ' ')

echo "   Total rows: $TOTAL_RECEIVER"
echo "   Rows needing timestamp sync: $RECEIVER_TO_UPDATE"
if [ "$RECEIVER_TO_UPDATE" -gt 0 ]; then
    echo -e "   ${YELLOW}!${NC} Migration needed"
else
    echo -e "   ${GREEN}✓${NC} No updates needed"
fi

# Check for orphaned rows
echo ""
echo "4. Checking for orphaned rows..."
ORPHANED_FIXES=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
    SELECT COUNT(*)
    FROM fixes f
    WHERE f.aprs_message_id IS NOT NULL
      AND NOT EXISTS (
          SELECT 1 FROM aprs_messages am
          WHERE am.id = f.aprs_message_id
      );
" | tr -d ' ')

ORPHANED_RECEIVER=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "
    SELECT COUNT(*)
    FROM receiver_statuses rs
    WHERE rs.aprs_message_id IS NOT NULL
      AND NOT EXISTS (
          SELECT 1 FROM aprs_messages am
          WHERE am.id = rs.aprs_message_id
      );
" | tr -d ' ')

echo "   Orphaned fixes: $ORPHANED_FIXES"
echo "   Orphaned receiver_statuses: $ORPHANED_RECEIVER"

# Estimate time
echo ""
echo -e "${BLUE}==> Migration Estimate${NC}\n"

TOTAL_UPDATES=$((FIXES_TO_UPDATE + RECEIVER_TO_UPDATE))
PARTITIONS=$(psql -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'fixes_p%';" | tr -d ' ')

echo "Total rows to update: $TOTAL_UPDATES"
echo "Partitions: $PARTITIONS"
echo ""

if [ "$TOTAL_UPDATES" -gt 0 ]; then
    # Rough estimate: 10,000 rows/sec with dropped indexes, divided by parallel jobs
    PARALLEL_JOBS=8
    SECONDS_ESTIMATE=$((TOTAL_UPDATES / 10000 / PARALLEL_JOBS))
    MINUTES=$((SECONDS_ESTIMATE / 60))

    if [ "$MINUTES" -gt 60 ]; then
        HOURS=$((MINUTES / 60))
        MINUTES=$((MINUTES % 60))
        echo "Estimated time: ~${HOURS}h ${MINUTES}m (with $PARALLEL_JOBS parallel jobs)"
    else
        echo "Estimated time: ~${MINUTES}m (with $PARALLEL_JOBS parallel jobs)"
    fi
    echo ""
    echo -e "${YELLOW}Ready to run migration:${NC} ./fix-aprs-fk-migration-parallel.sh"
else
    echo -e "${GREEN}No migration needed - data is already synced${NC}"
fi
