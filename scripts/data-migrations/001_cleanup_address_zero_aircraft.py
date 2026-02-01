#!/usr/bin/env python3
"""
Data migration 001: Clean up aircraft with address=0 and all dependent data.

These aircraft have invalid placeholder addresses. The schema migration
(2026-01-30-000000-0000_replace_address_with_typed_columns) drops the FK
constraints from fixes with NOT VALID, deletes the aircraft and flights,
but leaves orphaned fix rows in compressed hypertable chunks.

This script cleans up those orphaned fixes by processing each compressed
chunk individually: decompress -> delete -> recompress.

Usage:
    python3 scripts/data-migrations/001_cleanup_address_zero_aircraft.py <database> [parallelism]
    python3 scripts/data-migrations/001_cleanup_address_zero_aircraft.py soar_staging 2
"""

import sys

from _lib import get_chunks, parse_delete_count, process_chunks_parallel, run_sql, run_sql_command

if len(sys.argv) < 2:
    print(f"Usage: {sys.argv[0]} <database> [parallelism]")
    print(f"Example: {sys.argv[0]} soar_staging 2")
    sys.exit(1)

DB = sys.argv[1]
PARALLELISM = int(sys.argv[2]) if len(sys.argv) > 2 else 2


def main():
    print(f"=== Data Migration 001: Cleanup address=0 aircraft ===")
    print(f"Database: {DB}")
    print(f"Parallelism: {PARALLELISM}")
    print()

    # Discover aircraft with address=0
    output = run_sql(DB, "SELECT id FROM aircraft WHERE address = 0", return_output=True)
    if not output:
        print("No aircraft with address=0 found. Nothing to do.")
        return

    bad_ids = [line.strip() for line in output.strip().split("\n") if line.strip()]
    bad_ids_sql = "'" + "','".join(bad_ids) + "'"

    print(f"Found {len(bad_ids)} aircraft with address=0:")
    for aid in bad_ids:
        print(f"  {aid}")
    print()

    # Count orphaned fixes
    total_fixes = run_sql(
        DB,
        f"SELECT count(*) FROM fixes WHERE aircraft_id IN ({bad_ids_sql})",
        return_output=True,
    )
    print(f"Total fixes to delete: {total_fixes}")

    if total_fixes == "0":
        print("Nothing to do!")
        return

    print()

    # Get chunk info
    chunks = get_chunks(DB)
    compressed = [c for c in chunks if c["compressed"]]
    uncompressed = [c for c in chunks if not c["compressed"]]
    print(f"Chunks: {len(chunks)} total, {len(compressed)} compressed, {len(uncompressed)} uncompressed")

    # Step 1: Delete from uncompressed chunks (fast, no decompress needed)
    print()
    print("Step 1: Deleting from uncompressed chunks...")
    if uncompressed:
        import time
        start = time.time()
        output = run_sql_command(DB, f"""
            SET timescaledb.max_tuples_decompressed_per_dml_transaction = 0;
            DELETE FROM fixes
            WHERE aircraft_id IN ({bad_ids_sql})
              AND received_at >= '{uncompressed[0]["range_start"]}'::timestamptz
        """)
        duration = time.time() - start
        deleted = parse_delete_count(output)
        print(f"  Deleted {deleted} fixes ({duration:.1f}s)")
    else:
        print("  No uncompressed chunks.")

    # Step 2: Process compressed chunks in parallel
    if compressed:
        print()
        print(f"Step 2: Processing {len(compressed)} compressed chunks (parallelism={PARALLELISM})...")

        def delete_sql(chunk):
            return f"""
                DELETE FROM fixes
                WHERE aircraft_id IN ({bad_ids_sql})
                  AND received_at >= '{chunk["range_start"]}'::timestamptz
                  AND received_at < '{chunk["range_end"]}'::timestamptz
            """

        process_chunks_parallel(DB, compressed, delete_sql, PARALLELISM)

    # Step 3: Verify
    print()
    print("Step 3: Verifying...")
    remaining = run_sql(
        DB,
        f"SELECT count(*) FROM fixes WHERE aircraft_id IN ({bad_ids_sql})",
        return_output=True,
    )
    print(f"Remaining orphaned fixes: {remaining}")

    if remaining == "0":
        print()
        print("SUCCESS! All orphaned fixes cleaned up.")
    else:
        print()
        print(f"WARNING: {remaining} orphaned fixes remain. Re-run this script.")


if __name__ == "__main__":
    main()
