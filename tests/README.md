# Integration Tests

This directory contains integration tests for the SOAR project. Tests use **isolated per-test databases** to enable fast, parallel execution without interference.

## Quick Start

### First Time Setup

Create the test template database (one-time setup):

```bash
./scripts/setup-test-template.sh
```

This creates a `soar_test_template` database with all migrations applied. Each test will create its own database from this template.

### Running Tests

```bash
# Run all integration tests in parallel
cargo nextest run

# Run specific test file
cargo nextest run --test flight_detection_test

# Run a specific test
cargo nextest run --test flight_detection_test test_descended_out_of_range
```

Tests run in parallel by default, significantly faster than the old serial execution.

## How It Works

### Per-Test Database Isolation

Each integration test gets its own isolated PostgreSQL database:

1. **Template Database**: `soar_test_template` contains all migrations
2. **Test Execution**: Each test creates `soar_test_<random_id>` from template
3. **Automatic Cleanup**: Database is dropped when test completes (even on panic)

### Architecture

```
soar_test_template (created once)
    ↓ (template copy, ~100ms)
soar_test_abc123 → Test 1 runs
soar_test_def456 → Test 2 runs (parallel)
soar_test_xyz789 → Test 3 runs (parallel)
    ↓ (automatic cleanup via Drop trait)
All test databases dropped
```

## Writing New Tests

Use the `TestDatabase` helper from `tests/common/mod.rs`:

```rust
mod common;

use common::TestDatabase;
use soar::users_repo::UsersRepository;

#[tokio::test]
async fn test_my_feature() {
    // Create isolated test database
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let pool = test_db.pool();

    // Use the pool for your test
    let repo = UsersRepository::new(pool.clone());

    // ... test logic ...

    // Database automatically cleaned up when test_db goes out of scope
}
```

### Key Points

- **No manual cleanup needed**: Database is automatically dropped
- **No `#[serial]` needed**: Tests run in parallel by default
- **Fast database creation**: Template copy is ~100ms (vs. seconds for migrations)
- **Complete isolation**: No test can interfere with another

## Troubleshooting

### Error: Template database not found

```
Error: Failed to create test database from template.

The template database 'soar_test_template' may not exist.
Run: ./scripts/setup-test-template.sh
```

**Solution**: Run `./scripts/setup-test-template.sh` to create the template database.

### After Adding Migrations

When you add new database migrations, recreate the template:

```bash
./scripts/setup-test-template.sh
```

This updates the template with the latest schema changes.

### Leaked Databases

If tests are killed forcefully (SIGKILL), database cleanup may not run. To find and clean up leaked databases:

```sql
-- List test databases
SELECT datname FROM pg_database WHERE datname LIKE 'soar_test_%';

-- Drop all test databases (manual cleanup)
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT datname FROM pg_database WHERE datname LIKE 'soar_test_%' AND datname != 'soar_test_template'
    LOOP
        EXECUTE 'DROP DATABASE IF EXISTS ' || quote_ident(r.datname) || ' WITH (FORCE)';
    END LOOP;
END $$;
```

### PostgreSQL Version Requirements

The test infrastructure requires **PostgreSQL 13+** for `DROP DATABASE ... WITH (FORCE)` support.

On older PostgreSQL versions, you may see warnings about cleanup failures. This is not critical, but you may need to manually clean up leaked databases more frequently.

## Test Files

### Integration Tests

- **`flight_detection_test.rs`** (2 tests)
  - Tests flight detection and coalescing logic
  - Uses real APRS message sequences from `tests/data/flights/`
  - Slowest tests (~15s each), benefit most from parallelization

- **`pilot_invitation_workflow_test.rs`** (7 tests)
  - Tests user creation, invitation, and registration flows
  - Tests club membership management
  - Fast tests (~2-3s each)

- **`elevation_test.rs`** (~10 tests)
  - Tests elevation data processing
  - Doesn't use database (filesystem only)
  - Already parallelizable

### Test Data

Test data files are located in:

- `tests/data/flights/` - Real APRS message sequences for flight tests
- `tests/data/elevation/` - Elevation tiles for AGL calculations

## Performance

### Before (Serial Execution)

```
test-threads = 1
Total time: 60-90 seconds
```

### After (Parallel Execution)

```
Full parallelism enabled
Total time: 15-30 seconds (3-4x faster)
```

The speedup scales with the number of CPU cores available.

## Implementation Details

### TestDatabase Helper

See `tests/common/mod.rs` for the implementation of the `TestDatabase` struct.

Key features:

- **RAII Pattern**: Cleanup via Rust's `Drop` trait
- **Random Database Names**: Collision-resistant with 62^12 combinations
- **Connection Pooling**: Standard r2d2 pool for Diesel
- **Error Handling**: Helpful messages for common issues

### CI/CD Integration

The CI pipeline (`.github/workflows/ci.yml`) runs:

1. PostgreSQL service container starts
2. Migrations run on `soar_test` database
3. **Template setup**: `./scripts/setup-test-template.sh` creates template
4. Tests run in parallel using template

## Environment Variables

- `TEST_DATABASE_URL`: Base database URL (default: `postgresql://localhost/soar_test`)
- `PGHOST`, `PGPORT`, `PGUSER`, `PGPASSWORD`: PostgreSQL connection parameters

The test infrastructure automatically modifies the database URL to create isolated test databases.

## Maintenance

### Regular Maintenance

No regular maintenance needed! The test infrastructure is self-cleaning.

### When to Recreate Template

Recreate the template database when:

- ✅ You add new migrations
- ✅ You modify existing migrations (dev only)
- ✅ Schema changes are made

Just run: `./scripts/setup-test-template.sh`

### Monitoring for Leaks

Occasionally check for leaked databases (shouldn't happen in normal operation):

```bash
psql -U postgres -d postgres -c "SELECT datname FROM pg_database WHERE datname LIKE 'soar_test_%';"
```

Expected output:
```
       datname
----------------------
 soar_test_template
(1 row)
```

If you see more than one row, you have leaked databases that can be cleaned up.

## References

- **PostgreSQL Template Databases**: https://www.postgresql.org/docs/current/manage-ag-templatedbs.html
- **Diesel ORM**: https://diesel.rs/
- **Nextest**: https://nexte.st/
