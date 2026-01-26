#!/usr/bin/env python3
"""
Parallel fixes update - decompresses/updates/recompresses chunks concurrently.

Usage:
    python3 scripts/update_fixes_parallel.py [database_name] [parallelism]
    python3 scripts/update_fixes_parallel.py soar_staging 4
"""

import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
import time

DB = sys.argv[1] if len(sys.argv) > 1 else "soar_staging"
PARALLELISM = int(sys.argv[2]) if len(sys.argv) > 2 else 4


def run_sql(sql: str, return_output: bool = False):
    """Run SQL command and optionally return output."""
    cmd = ["psql", "-d", DB, "-tAc", sql]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"SQL Error: {result.stderr}")
        return None
    return result.stdout.strip() if return_output else result.returncode == 0


def get_compressed_chunks():
    """Get list of all compressed chunks."""
    sql = """
        SELECT chunk_schema || '.' || chunk_name
        FROM timescaledb_information.chunks
        WHERE hypertable_name = 'fixes'
          AND is_compressed = true
        ORDER BY range_start
    """
    output = run_sql(sql, return_output=True)
    return [c.strip() for c in output.split('\n') if c.strip()]


def decompress_chunk(chunk: str) -> tuple[str, float, bool]:
    """Decompress a single chunk. Returns (chunk, duration, success)."""
    start = time.time()
    try:
        result = run_sql(f"SELECT decompress_chunk('{chunk}');")
        duration = time.time() - start
        return (chunk, duration, True)
    except Exception as e:
        duration = time.time() - start
        print(f"  Error decompressing {chunk}: {e}")
        return (chunk, duration, False)


def compress_chunk(chunk: str) -> tuple[str, float, bool]:
    """Compress a single chunk. Returns (chunk, duration, success)."""
    start = time.time()
    try:
        result = run_sql(f"SELECT compress_chunk('{chunk}');")
        duration = time.time() - start
        return (chunk, duration, True)
    except Exception as e:
        duration = time.time() - start
        print(f"  Error compressing {chunk}: {e}")
        return (chunk, duration, False)


def update_fixes_for_chunk(chunk: str) -> tuple[str, int, float]:
    """Update fixes in a specific chunk. Returns (chunk, count, duration)."""
    # Get the time range for this chunk
    chunk_name = chunk.split('.')[-1]
    time_range_sql = f"""
        SELECT range_start::text, range_end::text
        FROM timescaledb_information.chunks
        WHERE chunk_name = '{chunk_name}'
    """
    output = run_sql(time_range_sql, return_output=True)
    if not output:
        return (chunk, 0, 0)

    range_start, range_end = output.split('|') if '|' in output else (None, None)
    if not range_start:
        return (chunk, 0, 0)

    start = time.time()
    update_sql = f"""
        SET timescaledb.max_tuples_decompressed_per_dml_transaction = 0;
        UPDATE fixes fx
        SET aircraft_id = m.icao_id
        FROM aircraft_merge_mapping m
        WHERE fx.aircraft_id = m.flarm_id
          AND fx.received_at >= '{range_start}'::timestamptz
          AND fx.received_at < '{range_end}'::timestamptz
    """
    result = subprocess.run(
        ["psql", "-d", DB, "-c", update_sql],
        capture_output=True, text=True
    )
    duration = time.time() - start

    # Parse UPDATE count
    count = 0
    if "UPDATE" in result.stdout:
        try:
            count = int(result.stdout.split("UPDATE")[1].strip().split()[0])
        except:
            pass

    return (chunk, count, duration)


def main():
    print(f"=== Parallel Fixes Update ===")
    print(f"Database: {DB}")
    print(f"Parallelism: {PARALLELISM}")
    print()

    # Check prerequisites
    exists = run_sql(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'aircraft_merge_mapping')",
        return_output=True
    )
    if exists != 't':
        print("ERROR: aircraft_merge_mapping table not found.")
        sys.exit(1)

    aircraft_count = run_sql("SELECT COUNT(*) FROM aircraft_merge_mapping", return_output=True)
    print(f"Aircraft to merge: {aircraft_count}")
    print()

    # Step 1: Get all compressed chunks
    print("Step 1: Getting compressed chunks...")
    chunks = get_compressed_chunks()
    print(f"Found {len(chunks)} compressed chunks")

    if not chunks:
        print("No compressed chunks - running direct update...")
        start = time.time()
        result = subprocess.run([
            "psql", "-d", DB, "-c",
            """SET timescaledb.max_tuples_decompressed_per_dml_transaction = 0;
               UPDATE fixes fx
               SET aircraft_id = m.icao_id
               FROM aircraft_merge_mapping m
               WHERE fx.aircraft_id = m.flarm_id"""
        ], capture_output=True, text=True)
        print(f"Update complete in {time.time() - start:.1f}s")
        print(result.stdout)
        return

    # Step 2: Decompress all chunks in parallel
    print()
    print(f"Step 2: Decompressing {len(chunks)} chunks (parallelism={PARALLELISM})...")
    decompress_start = time.time()

    with ThreadPoolExecutor(max_workers=PARALLELISM) as executor:
        futures = {executor.submit(decompress_chunk, chunk): chunk for chunk in chunks}
        completed = 0
        for future in as_completed(futures):
            completed += 1
            chunk, duration, success = future.result()
            status = "OK" if success else "FAILED"
            print(f"  [{completed}/{len(chunks)}] {chunk.split('.')[-1]}: {duration:.1f}s ({status})")

    decompress_duration = time.time() - decompress_start
    print(f"Decompression complete in {decompress_duration:.1f}s")

    # Step 3: Bulk update
    print()
    print("Step 3: Bulk update (single query with decompression limit disabled)...")
    update_start = time.time()

    result = subprocess.run([
        "psql", "-d", DB, "-c",
        """SET timescaledb.max_tuples_decompressed_per_dml_transaction = 0;
           UPDATE fixes fx
           SET aircraft_id = m.icao_id
           FROM aircraft_merge_mapping m
           WHERE fx.aircraft_id = m.flarm_id"""
    ], capture_output=True, text=True)

    update_duration = time.time() - update_start
    print(f"Update complete in {update_duration:.1f}s")
    print(result.stdout.strip())

    # Step 4: Recompress all chunks in parallel
    print()
    print(f"Step 4: Recompressing {len(chunks)} chunks (parallelism={PARALLELISM})...")
    compress_start = time.time()

    # Need to get the list again since chunk names are the same
    with ThreadPoolExecutor(max_workers=PARALLELISM) as executor:
        futures = {executor.submit(compress_chunk, chunk): chunk for chunk in chunks}
        completed = 0
        for future in as_completed(futures):
            completed += 1
            chunk, duration, success = future.result()
            status = "OK" if success else "FAILED"
            print(f"  [{completed}/{len(chunks)}] {chunk.split('.')[-1]}: {duration:.1f}s ({status})")

    compress_duration = time.time() - compress_start
    print(f"Compression complete in {compress_duration:.1f}s")

    # Summary
    total_duration = time.time() - decompress_start
    print()
    print("=== Summary ===")
    print(f"Chunks processed: {len(chunks)}")
    print(f"Decompress time: {decompress_duration:.1f}s")
    print(f"Update time: {update_duration:.1f}s")
    print(f"Compress time: {compress_duration:.1f}s")
    print(f"Total time: {total_duration:.1f}s")

    # Verify
    remaining = run_sql("""
        SELECT COUNT(*)
        FROM aircraft_merge_mapping m
        WHERE EXISTS (
            SELECT 1 FROM fixes fx
            WHERE fx.aircraft_id = m.flarm_id
            LIMIT 1
        )
    """, return_output=True)
    print()
    print(f"Aircraft IDs still needing fixes updated: {remaining}")

    if remaining == "0":
        print()
        print("SUCCESS! All fixes updated.")
        print("Next: Run 'diesel migration run' to complete migration 2")


if __name__ == "__main__":
    main()
