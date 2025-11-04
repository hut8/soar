# Post-Compact Work Instructions

## What Was Done

This branch (`feat/optimize-bbox-query`) optimizes the "device in bounding box" query and reduces the fixes table column count.

### Schema Changes (Migration `2025-11-04-040220-0000_optimize_fixes_table_drop_redundant_add_geom`)

#### Dropped Columns from `fixes` table:
- `address_type` - Now stored only on `devices` table
- `aircraft_type_ogn` - Now stored only on `devices` table
- `device_address` - Now stored only on `devices` table
- `registration` - Now stored only on `devices` table

**Rationale**: These fields were redundant - they're already stored on the `devices` table which has a foreign key relationship with `fixes`. This reduces the fixes table from 32 columns to 28 columns (we're now safely under PostgreSQL's 32-column soft limit).

#### Added Column to `fixes` table:
- `geom geometry(Point, 4326) GENERATED ALWAYS AS (ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)) STORED`
- Created GIST index: `CREATE INDEX fixes_geom_idx ON fixes USING GIST (geom)`

**Rationale**: The `geom` column is a PostGIS geometry type (planar coordinates) generated from latitude/longitude. This allows us to use the fast `&&` (bounding box overlaps) operator for spatial queries instead of the slower `ST_Intersects` on geography types.

### Query Optimization

**Before**:
```sql
WITH bbox AS (
    SELECT ST_MakeEnvelope($1, $2, $3, $4, 4326)::geography AS g
)
SELECT DISTINCT d.*
FROM devices d
CROSS JOIN bbox
INNER JOIN fixes f ON f.device_id = d.id
WHERE f.received_at >= $5
  AND ST_Intersects(f.location, bbox.g)
```

**After**:
```sql
WITH bbox AS (
    SELECT ST_MakeEnvelope($1, $2, $3, $4, 4326)::geometry AS g
)
SELECT d.*
FROM devices d
WHERE EXISTS (
    SELECT 1
    FROM fixes f, bbox
    WHERE f.device_id = d.id
      AND f.received_at >= $5
      AND f.geom && bbox.g
    LIMIT 1
)
```

**Improvements**:
1. Uses `EXISTS + LIMIT 1` pattern - more efficient than `JOIN + DISTINCT`
2. Uses `&&` operator on geometry types - faster than `ST_Intersects` on geography
3. Eliminates DISTINCT by using EXISTS (no duplicate devices)
4. Geometry casting for bounding box (geometry is faster for regional queries)

### Code Changes

#### Updated Files:
- `src/schema.rs` - Removed dropped columns, added `geom` field with Geometry type
- `src/fixes.rs` - Removed dropped fields from `Fix` struct, updated `from_aprs_packet()`
- `src/devices.rs` - Added helper methods `device_address_hex()` and `get_aircraft_identifier()` to both `Device` and `DeviceModel`
- `src/flights.rs` - Updated Flight creation methods to accept `&Device` parameter instead of individual fields
- `src/fixes_repo.rs` - Optimized bounding box query, updated FixDslRow/FixRow structs
- `src/device_repo.rs` - Added `update_last_fix_at()` method
- `src/fix_processor.rs` - Updated to use new `update_last_fix_at()` method
- `src/flight_tracker/runway.rs` - Removed aircraft_type_ogn optimization (now requires device fetch)
- `src/flight_tracker/state_transitions.rs` - Fetch device to check aircraft_type_ogn when needed
- `src/flight_tracker/towing.rs` - Pass device_repo through call chain to check aircraft_type
- `src/flight_tracker/mod.rs` - Updated logging to use device_id instead of device_address_hex()
- `src/flight_tracker/aircraft_tracker.rs` - Fixed test Fix creation

## Migration Timing Considerations

⚠️ **IMPORTANT**: This migration will take a significant amount of time to complete.

### Split into Two Migrations:

**Migration 1** (`2025-11-04-040220-0000_optimize_fixes_table_drop_redundant_add_geom`):
- Drops 4 columns (fast, requires ACCESS EXCLUSIVE lock briefly)
- Adds geom GENERATED column (moderate, computes geometry for all rows)

**Migration 2** (`2025-11-04-042950-0000_create_fixes_geom_index`):
- Creates GIST index on geom column (**SLOW** - could take hours for large tables)
- Requires SHARE lock (blocks writes during index build)

### Estimated Duration:
- **Migration 1 (schema changes)**: Fast to moderate (1-60 seconds depending on table size)
- **Migration 2 (index creation)**: **SLOW** (minutes to hours for large fixes table)

### Lock Behavior:
- Migration 1: ACCESS EXCLUSIVE lock during ALTER TABLE (blocks all queries, but brief)
- Migration 2: SHARE lock during CREATE INDEX (blocks writes, could be long)
- Each migration runs in its own transaction

### Recommendation:
For production deployment on large tables:
1. Run Migration 1 first (schema changes) - should be fast
2. Monitor application to verify everything works with new schema
3. Run Migration 2 (index creation) during a maintenance window
4. If needed, you can run the application without the index temporarily (queries will be slower but functional)

## Post-Migration Verification

After running the migration in production:

### 1. Verify Schema Changes
```sql
-- Check dropped columns are gone
\d fixes

-- Verify geom column exists and is populated
SELECT COUNT(*) as total,
       COUNT(geom) as with_geom,
       COUNT(*) - COUNT(geom) as missing_geom
FROM fixes;

-- Check index exists
\di fixes_geom_idx
```

### 2. Test Query Performance
```sql
-- Test the optimized bounding box query
EXPLAIN ANALYZE
WITH bbox AS (
    SELECT ST_MakeEnvelope(-122.0, 37.0, -121.0, 38.0, 4326)::geometry AS g
)
SELECT d.*
FROM devices d
WHERE EXISTS (
    SELECT 1
    FROM fixes f, bbox
    WHERE f.device_id = d.id
      AND f.received_at >= NOW() - INTERVAL '24 hours'
      AND f.geom && bbox.g
    LIMIT 1
);
```

Expected: Should use `fixes_geom_idx` GIST index and complete quickly.

### 3. Verify Application Functionality
- Test device search by location in web UI
- Verify flight tracking still works correctly
- Check that device information (aircraft_type, registration) displays correctly

## Rollback Instructions

If issues are discovered, the migration can be rolled back using the down.sql:

```bash
diesel migration revert
```

⚠️ **WARNING**: The rollback will restore the columns but **cannot restore data**. The columns will be added back with NULL values. You would need to repopulate them from the devices table if needed.

## Performance Monitoring

After deployment, monitor:
1. **Query performance**: Check slow query logs for bounding box queries
2. **Index usage**: Verify GIST index is being used (`pg_stat_user_indexes`)
3. **Table size**: Monitor fixes table size (should be slightly smaller after dropping 4 columns)

```sql
-- Check index usage
SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
FROM pg_stat_user_indexes
WHERE indexname = 'fixes_geom_idx';

-- Check table size
SELECT pg_size_pretty(pg_total_relation_size('fixes'));
```

## Next Steps After Deployment

1. Monitor query performance for 24-48 hours
2. If performance is good, consider removing unused functions:
   - `fixes_repo.rs::lookup_device_uuid_by_address()`
   - `fixes_repo.rs::lookup_device_uuid()`
   - `flight_tracker/runway.rs::uses_runways()`
3. If performance is significantly better, document the pattern for future spatial queries

## Files Modified

- migrations/2025-11-04-040220-0000_optimize_fixes_table_drop_redundant_add_geom/up.sql
- migrations/2025-11-04-040220-0000_optimize_fixes_table_drop_redundant_add_geom/down.sql
- src/schema.rs
- src/fixes.rs
- src/devices.rs
- src/flights.rs
- src/fixes_repo.rs
- src/device_repo.rs
- src/fix_processor.rs
- src/flight_tracker/runway.rs
- src/flight_tracker/state_transitions.rs
- src/flight_tracker/towing.rs
- src/flight_tracker/mod.rs
- src/flight_tracker/aircraft_tracker.rs

## Testing Status

✅ All compilation errors fixed
✅ All 111 unit tests passing
✅ Cargo clippy warnings resolved (only dead code warnings remain)
✅ Code formatted with cargo fmt

## Ready for Review

This branch is ready for code review and testing. After approval, it can be merged and the migration can be applied to production.
