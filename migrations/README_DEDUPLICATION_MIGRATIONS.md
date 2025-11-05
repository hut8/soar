# Deduplication Migrations

## Overview

This directory contains split-up migrations for adding deduplication to `aprs_messages` and `fixes` tables to prevent duplicate data on JetStream message redelivery after crashes.

## Table Sizes (as of 2025-11-05)
- **aprs_messages**: 167 GB, ~385 million rows
- **fixes**: 110 GB, ~148 million rows

These are VERY LARGE tables, so migrations will be slow.

## Migration Order

### For aprs_messages:

1. **2025-11-05-052305-0000_add_aprs_messages_hash_column** ✅ FAST
   - Adds `raw_message_hash BYTEA` column (nullable)
   - Enables pgcrypto extension

2. **2025-11-05-052311-0000_backfill_aprs_messages_hashes** ⚠️ VERY SLOW
   - Computes SHA-256 hash for all 385M rows
   - Estimate: Could take hours
   - Run during low-traffic period

3. **2025-11-05-052318-0000_set_aprs_messages_hash_not_null** ✅ FAST
   - Makes `raw_message_hash` NOT NULL

4. **Manual duplicate cleanup** (if needed)
   - Use `find_duplicate_aprs_messages.sql` to check for duplicates
   - Use `delete_duplicate_aprs_messages.sql` to remove duplicates if needed

5. **2025-11-05-052332-0000_create_aprs_messages_unique_index** ⚠️ SLOW
   - Creates unique index on (receiver_id, received_at, raw_message_hash)
   - Will FAIL if duplicates still exist
   - Estimate: Could take 30+ minutes on 385M rows

### For fixes:

1. **Manual duplicate cleanup** (if needed)
   - Use `find_duplicate_fixes.sql` to check for duplicates
   - Use `delete_duplicate_fixes.sql` to remove duplicates if needed

2. **2025-11-05-052501-0000_create_fixes_unique_index** ⚠️ SLOW
   - Creates unique index on (device_id, timestamp)
   - Will FAIL if duplicates still exist
   - Estimate: Could take 20+ minutes on 148M rows

## Helper Scripts

### Finding Duplicates

```bash
# Check for duplicate APRS messages
psql soar -f migrations/find_duplicate_aprs_messages.sql

# Check for duplicate fixes
psql soar -f migrations/find_duplicate_fixes.sql
```

### Deleting Duplicates (if needed)

**WARNING: These scripts are IRREVERSIBLE. Back up your database first!**

```bash
# Delete duplicate APRS messages (dry-run with ROLLBACK by default)
psql soar -f migrations/delete_duplicate_aprs_messages.sql

# Delete duplicate fixes (dry-run with ROLLBACK by default)
psql soar -f migrations/delete_duplicate_fixes.sql
```

To actually commit the deletions, edit the `.sql` files and change `ROLLBACK;` to `COMMIT;`.

## Recommended Migration Strategy

1. **Run migrations 1-3 for aprs_messages** (add column, backfill, set NOT NULL)
   - Schedule backfill during low-traffic period
   - Monitor progress in database logs

2. **Check for duplicates**
   ```bash
   psql soar -f migrations/find_duplicate_aprs_messages.sql
   psql soar -f migrations/find_duplicate_fixes.sql
   ```

3. **Delete duplicates if needed** (manually, using helper scripts)

4. **Create unique indexes** (migrations 4-5)
   - These will fail if any duplicates remain
   - Can be run during normal operation but will be slow

## Old Migrations (Superseded)

- `2025-11-04-212029-0000_add_aprs_messages_deduplication.OLD` - Original combined migration (too slow)
- `2025-11-04-212102-0000_add_fixes_deduplication.OLD` - Original combined migration (too slow)

These are kept for reference but should not be run.
