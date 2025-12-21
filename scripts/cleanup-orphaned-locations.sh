#!/bin/bash
set -euo pipefail

# Script to remove orphaned location records that are not referenced by any other table
# Locations are referenced by:
# - aircraft_registrations.location_id
# - clubs.location_id
# - flights.landing_location_id
# - flights.takeoff_location_id

# Configuration
DATABASE="${DATABASE_URL:-postgresql://localhost/soar_staging}"
BATCH_SIZE="${BATCH_SIZE:-10000}"
DRY_RUN="${DRY_RUN:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Orphaned Locations Cleanup ===${NC}"
echo "Database: $DATABASE"
echo "Batch size: $BATCH_SIZE"
echo "Dry run: $DRY_RUN"
echo ""

# Count total locations
echo -e "${BLUE}Counting total locations...${NC}"
TOTAL_LOCATIONS=$(psql "$DATABASE" -t -c "SELECT COUNT(*) FROM locations;")
echo -e "Total locations: ${GREEN}${TOTAL_LOCATIONS}${NC}"

# Count referenced locations
echo -e "${BLUE}Counting referenced locations...${NC}"
REFERENCED_COUNT=$(psql "$DATABASE" -t -c "
SELECT COUNT(DISTINCT l.id)
FROM locations l
WHERE EXISTS (
    SELECT 1 FROM aircraft_registrations WHERE location_id = l.id
)
OR EXISTS (
    SELECT 1 FROM clubs WHERE location_id = l.id
)
OR EXISTS (
    SELECT 1 FROM flights WHERE landing_location_id = l.id OR takeoff_location_id = l.id
);
")
echo -e "Referenced locations: ${GREEN}${REFERENCED_COUNT}${NC}"

# Count orphaned locations
ORPHANED_COUNT=$((TOTAL_LOCATIONS - REFERENCED_COUNT))
echo -e "Orphaned locations: ${YELLOW}${ORPHANED_COUNT}${NC}"

if [ "$ORPHANED_COUNT" -eq 0 ]; then
    echo -e "${GREEN}No orphaned locations found. Nothing to clean up!${NC}"
    exit 0
fi

# Calculate percentage
ORPHANED_PCT=$(awk "BEGIN {printf \"%.1f\", ($ORPHANED_COUNT / $TOTAL_LOCATIONS) * 100}")
echo -e "Orphaned percentage: ${YELLOW}${ORPHANED_PCT}%${NC}"
echo ""

# Ask for confirmation unless in dry run mode
if [ "$DRY_RUN" = "false" ]; then
    echo -e "${RED}WARNING: This will delete ${ORPHANED_COUNT} location records!${NC}"
    read -p "Are you sure you want to continue? (yes/no): " -r
    echo
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        echo "Aborted."
        exit 1
    fi
fi

# Show sample of orphaned locations
echo -e "${BLUE}Sample of orphaned locations (first 10):${NC}"
psql "$DATABASE" -c "
SELECT id, street1, city, state, zip_code, country_code
FROM locations l
WHERE NOT EXISTS (
    SELECT 1 FROM aircraft_registrations WHERE location_id = l.id
)
AND NOT EXISTS (
    SELECT 1 FROM clubs WHERE location_id = l.id
)
AND NOT EXISTS (
    SELECT 1 FROM flights WHERE landing_location_id = l.id OR takeoff_location_id = l.id
)
LIMIT 10;
"
echo ""

if [ "$DRY_RUN" = "true" ]; then
    echo -e "${YELLOW}DRY RUN: Would delete ${ORPHANED_COUNT} orphaned locations${NC}"
    exit 0
fi

# Delete in batches
echo -e "${BLUE}Deleting orphaned locations in batches of ${BATCH_SIZE}...${NC}"
DELETED_TOTAL=0
BATCH_NUM=1

while true; do
    echo -e "${BLUE}Processing batch ${BATCH_NUM}...${NC}"

    # Delete a batch and capture the count
    DELETED=$(psql "$DATABASE" -t -c "
    WITH orphaned AS (
        SELECT id
        FROM locations l
        WHERE NOT EXISTS (
            SELECT 1 FROM aircraft_registrations WHERE location_id = l.id
        )
        AND NOT EXISTS (
            SELECT 1 FROM clubs WHERE location_id = l.id
        )
        AND NOT EXISTS (
            SELECT 1 FROM flights WHERE landing_location_id = l.id OR takeoff_location_id = l.id
        )
        LIMIT $BATCH_SIZE
    )
    DELETE FROM locations
    WHERE id IN (SELECT id FROM orphaned)
    RETURNING id;
    " | wc -l)

    DELETED_TOTAL=$((DELETED_TOTAL + DELETED))

    if [ "$DELETED" -eq 0 ]; then
        break
    fi

    PROGRESS=$(awk "BEGIN {printf \"%.1f\", ($DELETED_TOTAL / $ORPHANED_COUNT) * 100}")
    echo -e "  Deleted ${DELETED} records (Total: ${GREEN}${DELETED_TOTAL}${NC} / ${ORPHANED_COUNT} - ${PROGRESS}%)"

    BATCH_NUM=$((BATCH_NUM + 1))

    # Small delay to avoid overwhelming the database
    sleep 0.1
done

echo ""
echo -e "${GREEN}=== Cleanup Complete ===${NC}"
echo -e "Total deleted: ${GREEN}${DELETED_TOTAL}${NC}"
echo -e "Remaining locations: ${GREEN}$((TOTAL_LOCATIONS - DELETED_TOTAL))${NC}"

# Verify no orphans remain
REMAINING_ORPHANS=$(psql "$DATABASE" -t -c "
SELECT COUNT(*)
FROM locations l
WHERE NOT EXISTS (
    SELECT 1 FROM aircraft_registrations WHERE location_id = l.id
)
AND NOT EXISTS (
    SELECT 1 FROM clubs WHERE location_id = l.id
)
AND NOT EXISTS (
    SELECT 1 FROM flights WHERE landing_location_id = l.id OR takeoff_location_id = l.id
);
")

if [ "$REMAINING_ORPHANS" -gt 0 ]; then
    echo -e "${YELLOW}Warning: ${REMAINING_ORPHANS} orphaned locations still remain${NC}"
else
    echo -e "${GREEN}Success: No orphaned locations remain${NC}"
fi

# Run VACUUM to reclaim space
echo ""
echo -e "${BLUE}Running VACUUM ANALYZE on locations table to reclaim space...${NC}"
psql "$DATABASE" -c "VACUUM ANALYZE locations;"
echo -e "${GREEN}VACUUM complete${NC}"
