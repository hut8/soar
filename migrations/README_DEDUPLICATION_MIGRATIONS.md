# Deduplication Migrations - Fast Forward Strategy

## Overview

This migration adds deduplication support for `aprs_messages` to prevent duplicate data on JetStream message redelivery after crashes. Uses a **fast-forward strategy** that only computes hashes for NEW records going forward.

## Table Sizes (as of 2025-11-05)
- **aprs_messages**: 167 GB, ~385 million rows
- **fixes**: 110 GB, ~148 million rows

## Strategy: Fast Forward (No Backfill)

Instead of backfilling hashes for 385 million existing rows (which would take hours), we:

1. **Add the column** (fast, seconds)
2. **Let Rust compute hashes for new records** (already implemented in `NewAprsMessage::new()`)
3. **Old records stay NULL** (they never get hashed)
4. **Eventually add unique index** (later, when most records have hashes)

This means:
- ✅ Migration runs in seconds, not hours
- ✅ No downtime required
- ✅ New records are deduplicated immediately
- ⚠️ Old records (pre-migration) are not deduplicated
- ⚠️ Unique index will come later (separate migration)

## Current Migrations

### aprs_messages:

**2025-11-05-052305-0000_add_aprs_messages_hash_column** ✅ FAST (~1 second)
- Adds `raw_message_hash BYTEA` column (nullable, stays nullable)
- Enables pgcrypto extension
- Safe to run anytime

That's it! Just one migration.

### fixes:

No migrations yet. Will add unique index later after strategy is proven.

## Application Behavior

The Rust application (`src/aprs_messages_repo.rs`) already:
- Computes SHA-256 hash for all new APRS messages in `NewAprsMessage::new()`
- Handles `UniqueViolation` gracefully when index is eventually added
- Returns existing message ID on duplicate detection

So the code is ready - just need to add the column.

## Future Work (Not in This PR)

1. **Monitor hash coverage** - track what % of messages have hashes
2. **Add unique index** - once most records have hashes (maybe 90%+)
3. **Backfill old records** - optionally, if needed for complete deduplication
4. **Add fixes deduplication** - similar strategy

## Migration Order

1. Deploy this migration (adds column)
2. Deploy application code (already done in PR #279)
3. Monitor for a while
4. Add unique index later (separate PR)

## Old Migrations (Superseded)

- `2025-11-04-212029-0000_add_aprs_messages_deduplication.OLD` - Original combined migration (too slow)
- `2025-11-04-212102-0000_add_fixes_deduplication.OLD` - Original combined migration (too slow)

These are kept for reference but should not be run.
