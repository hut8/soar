#!/bin/bash
#
# Cleanup script for orphaned test worker databases
#
# This script removes any leftover soar_test_worker_* databases that may remain
# from failed test runs or interrupted Playwright executions.
#
# Usage:
#   ./scripts/cleanup-test-workers.sh
#
# The script will:
# 1. Terminate all connections to worker databases
# 2. Drop all databases matching the pattern soar_test_worker_*
#
# This is safe to run anytime - it only affects test worker databases,
# not soar_test, soar_test_template, or production databases.

set -e

# PostgreSQL connection parameters (can be overridden by environment variables)
PGHOST="${PGHOST:-localhost}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-postgres}"
export PGPASSWORD="${PGPASSWORD:-postgres}"

echo "🧹 Cleaning up test worker databases..."
echo

# Terminate all connections to worker databases
echo "Terminating connections to worker databases..."
psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d postgres -c "
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname LIKE 'soar_test_worker_%'
  AND pid <> pg_backend_pid();
" 2>/dev/null || true

# Find and drop all worker databases
echo "Dropping worker databases..."
WORKER_DBS=$(psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d postgres -t -c "
SELECT datname
FROM pg_database
WHERE datname LIKE 'soar_test_worker_%';
")

if [ -z "$WORKER_DBS" ]; then
    echo "✅ No worker databases found"
else
    for DB in $WORKER_DBS; do
        echo "  Dropping $DB..."
        psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d postgres -c "DROP DATABASE IF EXISTS $DB WITH (FORCE);" 2>/dev/null || true
    done
    echo "✅ Cleaned up worker databases"
fi

echo
echo "✨ Cleanup complete"
