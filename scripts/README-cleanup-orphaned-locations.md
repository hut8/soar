# Cleanup Orphaned Locations Script

## Purpose
Removes orphaned location records that are not referenced by any other table in the database.

## Background
Due to a bug in the `locations_address_unique_idx` unique index (which treats NULL values as distinct in PostgreSQL), the locations table accumulated massive duplicates. Out of 6.4M location records, only ~187k are unique addresses, and only ~3,900 are actually referenced by other tables.

## What Gets Deleted
Locations that are NOT referenced by any of these tables:
- `aircraft_registrations.location_id`
- `clubs.location_id`
- `flights.start_location_id`
- `flights.end_location_id`

## Usage

### Dry Run (Recommended First)
```bash
# Preview what would be deleted without making changes
DRY_RUN=true ./scripts/cleanup-orphaned-locations.sh
```

### Production Run
```bash
# On staging database (default)
./scripts/cleanup-orphaned-locations.sh

# On a specific database
DATABASE_URL="postgresql://localhost/soar_production" ./scripts/cleanup-orphaned-locations.sh

# With custom batch size (default: 10,000)
BATCH_SIZE=5000 ./scripts/cleanup-orphaned-locations.sh
```

## Configuration

Environment variables:
- `DATABASE_URL`: PostgreSQL connection string (default: `postgresql://localhost/soar_staging`)
- `BATCH_SIZE`: Number of records to delete per batch (default: `10000`)
- `DRY_RUN`: Set to `true` to preview without deleting (default: `false`)

## Safety Features

1. **Confirmation prompt**: Asks for explicit "yes" confirmation before deleting (unless in dry run mode)
2. **Batch processing**: Deletes in batches to avoid long-running transactions
3. **Progress reporting**: Shows deletion progress and percentage complete
4. **Verification**: Checks for remaining orphans after completion
5. **Sample preview**: Shows first 10 orphaned records before deletion
6. **VACUUM**: Runs `VACUUM ANALYZE` after deletion to reclaim disk space

## Output

The script provides:
- Total location count
- Referenced location count
- Orphaned location count and percentage
- Sample of orphaned locations (first 10)
- Batch-by-batch deletion progress
- Final verification of remaining orphans
- VACUUM status

## Example Output

```
=== Orphaned Locations Cleanup ===
Database: postgresql://localhost/soar_staging
Batch size: 10000
Dry run: false

Counting total locations...
Total locations: 6399652
Counting referenced locations...
Referenced locations: 3876
Orphaned locations: 6395776
Orphaned percentage: 99.9%

Sample of orphaned locations (first 10):
[... sample data ...]

WARNING: This will delete 6395776 location records!
Are you sure you want to continue? (yes/no): yes

Deleting orphaned locations in batches of 10000...
Processing batch 1...
  Deleted 10000 records (Total: 10000 / 6395776 - 0.2%)
Processing batch 2...
  Deleted 10000 records (Total: 20000 / 6395776 - 0.3%)
[...]

=== Cleanup Complete ===
Total deleted: 6395776
Remaining locations: 3876
Success: No orphaned locations remain

Running VACUUM ANALYZE on locations table to reclaim space...
VACUUM complete
```

## Expected Results

Based on current staging database:
- **Before**: 6,399,652 total locations
- **Referenced**: ~3,876 locations (0.06%)
- **Orphaned**: ~6,395,776 locations (99.9%)
- **After**: ~3,876 locations
- **Disk space reclaimed**: Significant (exact amount depends on TOAST data)

## Caution

- Always run with `DRY_RUN=true` first to verify the deletion set
- This script is idempotent - safe to run multiple times
- No data loss risk: only deletes truly orphaned records
- Recommended to run during low-traffic periods for large deletions
- After cleanup, consider running a full database backup

## Related Issues

- PR #486: Fixes the root cause of location duplication
- Migration `2025-12-14-211440-0000_fix_locations_unique_index`: Documents the index issue
