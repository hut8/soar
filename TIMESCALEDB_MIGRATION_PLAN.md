# TimescaleDB Migration Plan

## Overview

This document outlines the plan to convert our existing pg_partman-managed partitioned tables (`fixes` and `raw_messages`) to TimescaleDB hypertables.

## Current State

### Tables to Migrate
1. **fixes**: 277,949 rows
   - Partitioned by: `received_at` (timestamp with time zone)
   - Partition interval: 1 day
   - Current partitions: 7 (Dec 11-17, 2025)
   - Retention: 30 days (detach, keep table)
   - Primary key: `(id, received_at)`
   - Foreign keys: 4 (aircraft_id, flight_id, raw_message_id, receiver_id)
   - Indexes: 9 (including GIST for spatial data)

2. **raw_messages**: 483,063 rows
   - Partitioned by: `received_at` (timestamp with time zone)
   - Partition interval: 1 day
   - Current partitions: 7 (Dec 11-17, 2025) + default
   - Retention: 30 days (detach, keep table)
   - Primary key: `(id, received_at)`
   - Foreign keys: 1 (receiver_id)
   - Referenced by: fixes, receiver_statuses

### Current Partitioning Strategy
- Using native PostgreSQL partitioning with RANGE on `received_at`
- Managed by `pg_partman` extension
- Daily partitions created automatically
- Old partitions detached (not dropped) after 30 days

## Benefits of TimescaleDB Migration

### Performance Improvements
1. **Automatic partition management**: No need for pg_partman cron jobs
2. **Compression**: Can compress old partitions (chunks) to save 90%+ disk space
3. **Continuous aggregates**: Materialized views that update automatically
4. **Better query planning**: TimescaleDB optimizer is time-series aware
5. **Parallel chunk operations**: Better parallelization for bulk operations

### Operational Benefits
1. **Simplified architecture**: One extension instead of pg_partman + custom scripts
2. **Built-in retention policies**: Automatic chunk deletion (no detach/manual drop)
3. **Native compression**: Automatic compression of old data
4. **Better monitoring**: Built-in chunk statistics and health checks

### Feature Additions
1. **Continuous aggregates**: Pre-compute hourly/daily statistics for analytics
2. **Hierarchical storage**: Move old chunks to cheaper storage
3. **Distributed hypertables**: Future-proof for multi-node scaling

## Migration Strategy

### High-Level Approach

We'll use a **phased migration with minimal downtime**:

1. **Phase 1**: Install and test TimescaleDB (DONE via provision script)
2. **Phase 2**: Create new hypertables alongside existing tables
3. **Phase 3**: Migrate data from partitioned tables to hypertables
4. **Phase 4**: Switch application to use hypertables
5. **Phase 5**: Clean up old partitioned tables

### Detailed Migration Steps

#### Phase 1: Install TimescaleDB ✅ (COMPLETED)
- [x] Add TimescaleDB installation to provision script
- [x] Configure `shared_preload_libraries = 'timescaledb'`
- [x] Restart PostgreSQL
- [x] Commit and push changes

#### Phase 2: Create Migration (Development & Testing)

**Create migration file**: `2025-12-XX-XXXXXX-0000_convert_to_timescaledb`

**Migration steps** (`up.sql`):

1. **Enable TimescaleDB extension**
   ```sql
   CREATE EXTENSION IF NOT EXISTS timescaledb;
   ```

2. **Rename existing tables**
   ```sql
   ALTER TABLE raw_messages RENAME TO raw_messages_partman;
   ALTER TABLE fixes RENAME TO fixes_partman;
   ```

3. **Create new non-partitioned tables**
   ```sql
   -- Create raw_messages (same schema, no PARTITION BY clause)
   CREATE TABLE raw_messages (
       id uuid NOT NULL DEFAULT gen_random_uuid(),
       received_at timestamp with time zone NOT NULL,
       receiver_id uuid NOT NULL,
       unparsed text,
       raw_message_hash bytea NOT NULL,
       raw_message bytea NOT NULL,
       source message_source NOT NULL DEFAULT 'ogn'::message_source
   );

   -- Create fixes (same schema, no PARTITION BY clause)
   CREATE TABLE fixes (
       id uuid NOT NULL DEFAULT gen_random_uuid(),
       source character varying(9) NOT NULL,
       aprs_type character varying(9) NOT NULL,
       via text[] NOT NULL,
       timestamp timestamp with time zone NOT NULL,
       latitude double precision NOT NULL,
       longitude double precision NOT NULL,
       location geography(Point,4326) GENERATED ALWAYS AS (st_point(longitude, latitude)::geography) STORED,
       altitude_msl_feet integer,
       flight_number character varying(20),
       squawk character varying(4),
       ground_speed_knots real,
       track_degrees real CHECK (track_degrees >= 0 AND track_degrees < 360),
       climb_fpm integer,
       turn_rate_rot real,
       flight_id uuid,
       aircraft_id uuid NOT NULL,
       received_at timestamp with time zone NOT NULL,
       is_active boolean NOT NULL DEFAULT true,
       altitude_agl_feet integer,
       receiver_id uuid NOT NULL,
       raw_message_id uuid NOT NULL,
       altitude_agl_valid boolean NOT NULL DEFAULT false,
       location_geom geometry(Point,4326) GENERATED ALWAYS AS (st_setsrid(st_makepoint(longitude, latitude), 4326)) STORED,
       time_gap_seconds integer,
       source_metadata jsonb
   );
   ```

4. **Convert to hypertables**
   ```sql
   -- Convert raw_messages to hypertable
   SELECT create_hypertable(
       'raw_messages',
       'received_at',
       chunk_time_interval => INTERVAL '1 day',
       if_not_exists => TRUE
   );

   -- Convert fixes to hypertable
   SELECT create_hypertable(
       'fixes',
       'received_at',
       chunk_time_interval => INTERVAL '1 day',
       if_not_exists => TRUE
   );
   ```

5. **Copy data from old tables**
   ```sql
   -- Copy raw_messages data
   INSERT INTO raw_messages
   SELECT * FROM raw_messages_partman;

   -- Copy fixes data (exclude generated columns)
   INSERT INTO fixes (
       id, source, aprs_type, via, timestamp, latitude, longitude,
       altitude_msl_feet, flight_number, squawk, ground_speed_knots,
       track_degrees, climb_fpm, turn_rate_rot, flight_id, aircraft_id,
       received_at, is_active, altitude_agl_feet, receiver_id,
       raw_message_id, altitude_agl_valid, time_gap_seconds, source_metadata
   )
   SELECT
       id, source, aprs_type, via, timestamp, latitude, longitude,
       altitude_msl_feet, flight_number, squawk, ground_speed_knots,
       track_degrees, climb_fpm, turn_rate_rot, flight_id, aircraft_id,
       received_at, is_active, altitude_agl_feet, receiver_id,
       raw_message_id, altitude_agl_valid, time_gap_seconds, source_metadata
   FROM fixes_partman;
   ```

6. **Add primary keys**
   ```sql
   -- Note: TimescaleDB requires partition key in PK
   ALTER TABLE raw_messages ADD PRIMARY KEY (id, received_at);
   ALTER TABLE fixes ADD PRIMARY KEY (id, received_at);
   ```

7. **Recreate indexes on hypertables**
   ```sql
   -- raw_messages indexes
   -- (TimescaleDB automatically creates index on partition key)
   CREATE INDEX idx_raw_messages_receiver_id ON raw_messages (receiver_id);
   CREATE INDEX idx_raw_messages_hash ON raw_messages (raw_message_hash);

   -- fixes indexes
   CREATE INDEX idx_fixes_aircraft_received_at ON fixes (aircraft_id, received_at DESC);
   CREATE INDEX idx_fixes_location ON fixes USING GIST (location);
   CREATE INDEX idx_fixes_location_geom ON fixes USING GIST (location_geom);
   CREATE INDEX idx_fixes_source ON fixes (source);
   CREATE INDEX idx_fixes_protocol ON fixes ((source_metadata ->> 'protocol')) WHERE source_metadata IS NOT NULL;
   CREATE INDEX idx_fixes_source_metadata ON fixes USING GIN (source_metadata);
   CREATE INDEX idx_fixes_flight_id ON fixes (flight_id);
   ```

8. **Recreate foreign key constraints**
   ```sql
   -- raw_messages FKs
   ALTER TABLE raw_messages
       ADD CONSTRAINT raw_messages_receiver_id_fkey
       FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE;

   -- fixes FKs
   ALTER TABLE fixes
       ADD CONSTRAINT fixes_aircraft_id_fkey
       FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) ON DELETE SET NULL;

   ALTER TABLE fixes
       ADD CONSTRAINT fixes_flight_id_fkey
       FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE RESTRICT;

   ALTER TABLE fixes
       ADD CONSTRAINT fixes_raw_message_id_fkey
       FOREIGN KEY (raw_message_id, received_at) REFERENCES raw_messages(id, received_at);

   ALTER TABLE fixes
       ADD CONSTRAINT fixes_receiver_id_fkey
       FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE SET NULL;
   ```

9. **Update tables that reference old partitioned tables**
   ```sql
   -- receiver_statuses references raw_messages
   ALTER TABLE receiver_statuses DROP CONSTRAINT receiver_statuses_raw_message_id_fkey;
   ALTER TABLE receiver_statuses
       ADD CONSTRAINT receiver_statuses_raw_message_id_fkey
       FOREIGN KEY (raw_message_id, received_at) REFERENCES raw_messages(id, received_at);
   ```

10. **Set up retention policies**
    ```sql
    -- Add retention policy to raw_messages (30 days)
    SELECT add_retention_policy('raw_messages', INTERVAL '30 days');

    -- Add retention policy to fixes (30 days)
    SELECT add_retention_policy('fixes', INTERVAL '30 days');
    ```

11. **Optional: Enable compression**
    ```sql
    -- Enable compression on raw_messages (compress chunks older than 7 days)
    ALTER TABLE raw_messages SET (
        timescaledb.compress,
        timescaledb.compress_segmentby = 'receiver_id',
        timescaledb.compress_orderby = 'received_at DESC'
    );
    SELECT add_compression_policy('raw_messages', INTERVAL '7 days');

    -- Enable compression on fixes (compress chunks older than 7 days)
    ALTER TABLE fixes SET (
        timescaledb.compress,
        timescaledb.compress_segmentby = 'aircraft_id',
        timescaledb.compress_orderby = 'received_at DESC'
    );
    SELECT add_compression_policy('fixes', INTERVAL '7 days');
    ```

12. **Remove pg_partman configuration**
    ```sql
    -- Remove pg_partman entries
    DELETE FROM partman.part_config WHERE parent_table = 'public.raw_messages_partman';
    DELETE FROM partman.part_config WHERE parent_table = 'public.fixes_partman';
    ```

13. **Add helpful comments**
    ```sql
    COMMENT ON TABLE raw_messages IS 'TimescaleDB hypertable partitioned by received_at (1 day chunks). Automatic retention: 30 days. Compression enabled for chunks older than 7 days.';
    COMMENT ON TABLE fixes IS 'TimescaleDB hypertable partitioned by received_at (1 day chunks). Automatic retention: 30 days. Compression enabled for chunks older than 7 days.';
    ```

14. **Verify migration**
    ```sql
    -- Verify row counts match
    DO $$
    DECLARE
        raw_old_count bigint;
        raw_new_count bigint;
        fixes_old_count bigint;
        fixes_new_count bigint;
    BEGIN
        SELECT COUNT(*) INTO raw_old_count FROM raw_messages_partman;
        SELECT COUNT(*) INTO raw_new_count FROM raw_messages;
        SELECT COUNT(*) INTO fixes_old_count FROM fixes_partman;
        SELECT COUNT(*) INTO fixes_new_count FROM fixes;

        IF raw_old_count != raw_new_count THEN
            RAISE EXCEPTION 'raw_messages migration failed: count mismatch. Old: %, New: %', raw_old_count, raw_new_count;
        END IF;

        IF fixes_old_count != fixes_new_count THEN
            RAISE EXCEPTION 'fixes migration failed: count mismatch. Old: %, New: %', fixes_old_count, fixes_new_count;
        END IF;

        RAISE NOTICE 'Migration successful: raw_messages=% rows, fixes=% rows', raw_new_count, fixes_new_count;
    END $$;
    ```

**Migration down** (`down.sql`):

This would reverse the migration by:
1. Renaming hypertables back to temporary names
2. Renaming _partman tables back to original names
3. Re-enabling pg_partman configuration
4. Dropping hypertables

(Full down.sql to be written during implementation)

#### Phase 3: Testing Strategy

1. **Development testing**
   - Run migration on local dev database
   - Verify all data migrated correctly
   - Test all queries still work
   - Verify foreign key relationships intact
   - Test insert/update/delete operations

2. **Staging testing**
   - Run migration on staging server (smaller dataset)
   - Monitor performance during migration
   - Test application functionality
   - Verify monitoring and metrics
   - Test backup/restore procedures

3. **Performance testing**
   - Compare query performance: partitioned vs hypertable
   - Test compression effectiveness
   - Measure insert throughput
   - Test retention policy execution

#### Phase 4: Production Migration

1. **Pre-migration checklist**
   - [ ] Full database backup
   - [ ] Staging migration successful
   - [ ] All tests passing
   - [ ] Maintenance window scheduled
   - [ ] Rollback plan documented

2. **Migration execution**
   - Schedule maintenance window (estimated: 10-20 minutes for current data size)
   - Stop SOAR services (soar-run, soar-web, ingest-*)
   - Run migration
   - Verify data integrity
   - Start SOAR services
   - Monitor for errors

3. **Post-migration monitoring**
   - Monitor application logs for errors
   - Check database performance metrics
   - Verify data ingestion working
   - Monitor chunk creation
   - Verify retention policy execution

#### Phase 5: Cleanup

After successful migration and verification (1-2 weeks):

1. **Drop old partitioned tables**
   ```sql
   DROP TABLE fixes_partman CASCADE;
   DROP TABLE raw_messages_partman CASCADE;
   ```

2. **Remove pg_partman** (optional, if not used elsewhere)
   ```sql
   DROP EXTENSION pg_partman CASCADE;
   DROP SCHEMA partman CASCADE;
   ```

3. **Update documentation**
   - Update CLAUDE.md with TimescaleDB information
   - Update deployment docs
   - Update monitoring/alerting docs

## Risks and Mitigation

### Risk 1: Migration Time
- **Risk**: Migration takes longer than maintenance window
- **Mitigation**: Test on staging first, use batched inserts if needed
- **Rollback**: Rename tables back, restart services

### Risk 2: Data Loss
- **Risk**: Data lost during migration
- **Mitigation**: Full backup before migration, verify row counts
- **Rollback**: Restore from backup

### Risk 3: Foreign Key Issues
- **Risk**: Foreign keys from other tables break
- **Mitigation**: Update all referencing tables in same migration
- **Rollback**: Down migration restores original structure

### Risk 4: Application Compatibility
- **Risk**: Application code doesn't work with hypertables
- **Mitigation**: Test on staging first, hypertables are mostly transparent
- **Rollback**: Down migration

### Risk 5: Performance Regression
- **Risk**: Queries slower on hypertables
- **Mitigation**: Performance testing on staging, adjust chunk intervals if needed
- **Rollback**: Down migration if significant regression

## Success Criteria

- [ ] All data migrated (row counts match)
- [ ] All foreign keys working
- [ ] All indexes recreated
- [ ] Application functionality unchanged
- [ ] No performance regression
- [ ] Retention policy working
- [ ] Compression policy working (if enabled)
- [ ] Monitoring showing healthy chunks

## Timeline Estimate

- **Phase 1**: Completed ✅
- **Phase 2**: 4-6 hours (write migration, test locally)
- **Phase 3**: 1-2 days (staging testing, performance validation)
- **Phase 4**: 1 hour (production migration during maintenance window)
- **Phase 5**: 1 week monitoring, then cleanup

**Total**: ~1-2 weeks from start to cleanup

## Next Steps

1. Create the migration file
2. Test on local dev database
3. Document any issues found
4. Test on staging
5. Schedule production maintenance window
6. Execute production migration
7. Monitor and verify
8. Clean up old tables

## References

- [TimescaleDB Documentation](https://docs.timescale.com/)
- [TimescaleDB Hypertables](https://docs.timescale.com/use-timescale/latest/hypertables/)
- [TimescaleDB Compression](https://docs.timescale.com/use-timescale/latest/compression/)
- [TimescaleDB Retention](https://docs.timescale.com/use-timescale/latest/data-retention/)
- [Migration from pg_partman](https://docs.timescale.com/migrate/latest/pg-dump-and-restore/)
