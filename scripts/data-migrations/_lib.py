"""
Shared helpers for data migration scripts.

Provides SQL execution, TimescaleDB chunk discovery, and parallel
decompress/recompress operations via psql.
"""

import subprocess
import time
from concurrent.futures import ThreadPoolExecutor, as_completed


def run_sql(db, sql, return_output=False):
    """Run a SQL statement via psql -tAc (unaligned, tuples-only)."""
    result = subprocess.run(
        ["psql", "-d", db, "-tAc", sql],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"  SQL error: {result.stderr.strip()}")
        return None
    return result.stdout.strip() if return_output else result.returncode == 0


def run_sql_command(db, sql):
    """Run a SQL command via psql -c (for DML with row counts)."""
    result = subprocess.run(
        ["psql", "-d", db, "-c", sql],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"  SQL error: {result.stderr.strip()}")
        return None
    return result.stdout.strip()


def parse_delete_count(output):
    """Extract the row count from a 'DELETE N' psql output."""
    if output and "DELETE" in output:
        try:
            return int(output.split("DELETE")[1].strip())
        except (ValueError, IndexError):
            pass
    return 0


def get_chunks(db, hypertable="fixes"):
    """Get all chunks for a hypertable with compression status."""
    sql = f"""
        SELECT chunk_schema || '.' || chunk_name, is_compressed::text,
               range_start::text, range_end::text
        FROM timescaledb_information.chunks
        WHERE hypertable_name = '{hypertable}'
        ORDER BY range_start
    """
    output = run_sql(db, sql, return_output=True)
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


def process_chunk(db, chunk, delete_sql_fn):
    """Decompress a chunk, run a delete, recompress.

    Args:
        db: Database name.
        chunk: Chunk dict from get_chunks().
        delete_sql_fn: Callable(chunk) -> SQL string for the DELETE statement.

    Returns:
        (chunk_name, deleted_count, duration_seconds)
    """
    name = chunk["name"]
    start = time.time()

    # Decompress
    run_sql(db, f"SELECT decompress_chunk('{name}');")

    # Delete
    output = run_sql_command(db, delete_sql_fn(chunk))
    deleted = parse_delete_count(output)

    # Recompress
    run_sql(db, f"SELECT compress_chunk('{name}');")

    duration = time.time() - start
    return name, deleted, duration


def process_chunks_parallel(db, chunks, delete_sql_fn, parallelism=2):
    """Process multiple compressed chunks in parallel.

    Args:
        db: Database name.
        chunks: List of chunk dicts (compressed only).
        delete_sql_fn: Callable(chunk) -> SQL string for the DELETE statement.
        parallelism: Number of concurrent workers.

    Returns:
        Total number of deleted rows.
    """
    total_deleted = 0
    start = time.time()

    with ThreadPoolExecutor(max_workers=parallelism) as executor:
        futures = {
            executor.submit(process_chunk, db, chunk, delete_sql_fn): chunk
            for chunk in chunks
        }
        completed = 0
        for future in as_completed(futures):
            completed += 1
            name, deleted, duration = future.result()
            total_deleted += deleted
            short_name = name.split(".")[-1]
            print(f"  [{completed}/{len(chunks)}] {short_name}: deleted {deleted} rows ({duration:.1f}s)")

    total_duration = time.time() - start
    print(f"  Total: {total_deleted} rows deleted from {len(chunks)} compressed chunks ({total_duration:.1f}s)")
    return total_deleted
