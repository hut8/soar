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

import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

if len(sys.argv) < 2:
    print(f"Usage: {sys.argv[0]} <database> [parallelism]")
    print(f"Example: {sys.argv[0]} soar_staging 2")
    sys.exit(1)

DB = sys.argv[1]
PARALLELISM = int(sys.argv[2]) if len(sys.argv) > 2 else 2

# Will be set in main() after discovery
BAD_IDS_SQL = ""


def run_sql(sql, return_output=False):
    """Run a SQL statement via psql."""
    result = subprocess.run(
        ["psql", "-d", DB, "-tAc", sql],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"  SQL error: {result.stderr.strip()}")
        return None
    return result.stdout.strip() if return_output else result.returncode == 0


def run_sql_command(sql):
    """Run a SQL command via psql -c (for DML with row counts)."""
    result = subprocess.run(
        ["psql", "-d", DB, "-c", sql],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"  SQL error: {result.stderr.strip()}")
        return None
    return result.stdout.strip()


def get_chunks():
    """Get all fixes chunks with compression status."""
    sql = """
        SELECT chunk_schema || '.' || chunk_name, is_compressed::text,
               range_start::text, range_end::text
        FROM timescaledb_information.chunks
        WHERE hypertable_name = 'fixes'
        ORDER BY range_start
    """
    output = run_sql(sql, return_output=True)
    if not output:
        return []
    chunks = []
    for line in output.strip().split("\n"):
        parts = line.split("|")
        if len(parts) == 4:
            chunks.append({
                "name": parts[0].strip(),
                "compressed": parts[1].strip() == "true",
                "range_start": parts[2].strip(),
                "range_end": parts[3].strip(),
            })
    return chunks


def process_compressed_chunk(chunk):
    """Decompress a chunk, delete orphaned fixes, recompress."""
    name = chunk["name"]
    start = time.time()

    # Decompress
    run_sql(f"SELECT decompress_chunk('{name}');")

    # Delete orphaned fixes in this time range
    output = run_sql_command(f"""
        DELETE FROM fixes
        WHERE aircraft_id IN ({BAD_IDS_SQL})
          AND received_at >= '{chunk["range_start"]}'::timestamptz
          AND received_at < '{chunk["range_end"]}'::timestamptz
    """)

    deleted = 0
    if output and "DELETE" in output:
        try:
            deleted = int(output.split("DELETE")[1].strip())
        except (ValueError, IndexError):
            pass

    # Recompress
    run_sql(f"SELECT compress_chunk('{name}');")

    duration = time.time() - start
    return name, deleted, duration


def main():
    global BAD_IDS_SQL

    print(f"=== Data Migration 001: Cleanup address=0 aircraft ===")
    print(f"Database: {DB}")
    print(f"Parallelism: {PARALLELISM}")
    print()

    # Discover aircraft with address=0
    output = run_sql("SELECT id FROM aircraft WHERE address = 0", return_output=True)
    if not output:
        print("No aircraft with address=0 found. Nothing to do.")
        return

    bad_ids = [line.strip() for line in output.strip().split("\n") if line.strip()]
    BAD_IDS_SQL = "'" + "','".join(bad_ids) + "'"

    print(f"Found {len(bad_ids)} aircraft with address=0:")
    for aid in bad_ids:
        print(f"  {aid}")
    print()

    # Count orphaned fixes
    total_fixes = run_sql(
        f"SELECT count(*) FROM fixes WHERE aircraft_id IN ({BAD_IDS_SQL})",
        return_output=True,
    )
    print(f"Total fixes to delete: {total_fixes}")

    if total_fixes == "0":
        print("Nothing to do!")
        return

    print()

    # Get chunk info
    chunks = get_chunks()
    compressed = [c for c in chunks if c["compressed"]]
    uncompressed = [c for c in chunks if not c["compressed"]]
    print(f"Chunks: {len(chunks)} total, {len(compressed)} compressed, {len(uncompressed)} uncompressed")

    # Step 1: Delete from uncompressed chunks (fast, no decompress needed)
    print()
    print("Step 1: Deleting from uncompressed chunks...")
    start = time.time()
    output = run_sql_command(f"""
        SET timescaledb.max_tuples_decompressed_per_dml_transaction = 0;
        DELETE FROM fixes
        WHERE aircraft_id IN ({BAD_IDS_SQL})
          AND received_at >= '{uncompressed[0]["range_start"]}'::timestamptz
    """) if uncompressed else "DELETE 0"
    duration = time.time() - start
    print(f"  {output} ({duration:.1f}s)")

    # Step 2: Process compressed chunks in parallel
    if compressed:
        print()
        print(f"Step 2: Processing {len(compressed)} compressed chunks (parallelism={PARALLELISM})...")
        total_deleted = 0
        step2_start = time.time()

        with ThreadPoolExecutor(max_workers=PARALLELISM) as executor:
            futures = {
                executor.submit(process_compressed_chunk, chunk): chunk
                for chunk in compressed
            }
            completed = 0
            for future in as_completed(futures):
                completed += 1
                name, deleted, duration = future.result()
                total_deleted += deleted
                short_name = name.split(".")[-1]
                print(f"  [{completed}/{len(compressed)}] {short_name}: deleted {deleted} fixes ({duration:.1f}s)")

        step2_duration = time.time() - step2_start
        print(f"  Total: {total_deleted} fixes deleted from compressed chunks ({step2_duration:.1f}s)")

    # Step 3: Verify
    print()
    print("Step 3: Verifying...")
    remaining = run_sql(
        f"SELECT count(*) FROM fixes WHERE aircraft_id IN ({BAD_IDS_SQL})",
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
