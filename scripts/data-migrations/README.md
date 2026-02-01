# Data Migrations

Data migrations live here, separate from Diesel schema migrations.

**Schema migrations** (`migrations/`) change the database structure (DDL) and run
automatically on deploy via `soar migrate`. They run inside a transaction and must
complete quickly.

**Data migrations** (`scripts/data-migrations/`) perform bulk data changes (DML)
that are too large or complex for schema migrations. They are run manually and may
take minutes to hours. Common reasons to use a data migration:

- Deleting or updating rows in the TimescaleDB `fixes` hypertable (compressed chunks
  must be decompressed first, which is slow and memory-intensive).
- Backfilling data across millions of rows.
- Any operation that would cause a schema migration to time out.

## Naming Convention

Scripts are numbered sequentially with a short description:

```
001_description.py
002_another_description.py
```

## Running

```bash
# Always specify the target database explicitly
python3 scripts/data-migrations/001_cleanup_orphaned_fixes.py soar_staging
python3 scripts/data-migrations/001_cleanup_orphaned_fixes.py soar

# Most scripts accept a parallelism argument
python3 scripts/data-migrations/001_cleanup_orphaned_fixes.py soar_staging 4
```

## Writing a New Data Migration

Use `_lib.py` for shared helpers (chunk discovery, parallel decompress/recompress,
SQL execution). Each migration script should:

1. Print what it will do before doing it (dry-run friendly).
2. Show progress as it works.
3. Verify the result at the end.
4. Be idempotent (safe to re-run).
