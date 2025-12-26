# CLAUDE.md - AI Assistant Guide for SOAR Project

This document provides essential guidance for AI assistants working on the SOAR (Soaring Observation And Records) project.

## Project Overview

SOAR is a comprehensive aircraft tracking and club management system built with:

- **Backend**: Rust with Axum web framework, PostgreSQL with PostGIS
- **Frontend**: SvelteKit with TypeScript, Tailwind CSS, Skeleton UI components
- **Real-time**: NATS messaging for live aircraft position updates
- **Data Sources**: APRS-IS integration, FAA aircraft registry, airport databases

## Critical Development Rules

### DOCUMENTATION PRIORITY
- **KEEP DOCUMENTATION UP TO DATE** - Documentation (including README.md, CLAUDE.md, and other docs) must be updated when features change
- When renaming services, commands, or changing architecture, update all relevant documentation
- Documentation changes should be part of the same PR that changes the implementation
- Outdated documentation is a bug - treat it with the same priority as code bugs

### NO BYPASSING QUALITY CONTROLS
- **NEVER commit directly to main** - The main branch is protected. ALWAYS create a feature/topic branch first
- **NEVER use `git commit --no-verify`** - All commits must pass pre-commit hooks
- **NEVER push to main** - Pushing to feature branches is okay, but never push directly to main
- **NEVER skip CI checks** - Local development must match GitHub Actions pipeline
- **ASK BEFORE removing large amounts of working code** - Get confirmation before major deletions
- **AVOID duplicate code** - Check for existing implementations before writing new code
- Pre-commit hooks run: `cargo fmt`, `cargo clippy`, `cargo test`, `npm lint`, `npm check`, `npm test`

### COMMIT AND DATABASE RULES
- **NEVER add Co-Authored-By lines** - Do not include Claude Code attribution in commits
- **AVOID raw SQL in Diesel** - Only use raw SQL if absolutely necessary, and ask first before using it
- Always prefer Diesel's query builder and type-safe methods over raw SQL
- **NEVER use CREATE INDEX CONCURRENTLY in Diesel migrations** - Diesel migrations run in transactions, which don't support CONCURRENTLY. Use regular CREATE INDEX instead

### SERVER ACCESS
- You are running on the staging server. The staging server is named "supervillain". You can always run commands that do not modify anything. Ask before running commands that modify something.
- You have access to the production server by running "ssh glider.flights". The user you are running as already has "sudo" access. Ask before connecting or using sudo unless I give you permission in advance.

### DATABASE SAFETY RULES (CRITICAL)
- **Development Database**: `soar_dev` - This is where you work
- **Staging Database**: `soar_staging` - This should be queried before the production database; its schema will be more up-to-date and it should contain approximately the same data. It is read-only for development purposes.
- **Production Database**: `soar` - This is read-only for development purposes
- **NEVER run UPDATE, INSERT, or DELETE on production database (`soar`)** - Only run these via Diesel migrations
- **ONLY DDL queries (CREATE, ALTER, DROP) via migrations** - Never run DDL queries manually on production
- **SELECT queries are allowed on both databases** - For investigation and analysis
- **All data modifications must go through migrations** - This ensures they're tracked and reproducible
- **Deleting data before adding constraints** - You can include DELETE statements in the same migration before constraint creation. The constraint validates against the final state of the transaction, so the DELETE will complete first.

### METRICS AND MONITORING

**CRITICAL - Grafana Dashboard Synchronization:**
- **ALWAYS update Grafana dashboards when changing metrics** - Any metric rename, addition, or removal MUST be reflected in the corresponding dashboard files in `infrastructure/`
- **Verify dashboard queries after changes** - After updating code, search all dashboard files for the old metric name and update them
- **Dashboard locations:**
  - `infrastructure/grafana-dashboard-run.json` - Main processing (`run` command)
  - `infrastructure/grafana-dashboard-ingest-ogn.json` - OGN/APRS ingestion (`ingest-ogn` command)
  - `infrastructure/grafana-dashboard-ingest-adsb.json` - ADS-B Beast ingestion (`ingest-adsb` command)
  - `infrastructure/grafana-dashboard-web.json` - Web server (`web` command)
  - `infrastructure/grafana-dashboard-nats.json` - NATS/JetStream metrics
  - `infrastructure/grafana-dashboard-analytics.json` - Analytics API and cache performance

**Metric Standards:**
- **Naming convention** - Use dot notation (e.g., `aprs.aircraft.device_upsert_ms`)
- **Document metric changes** - Note metric name changes in PR description for ops team awareness
- **Remove obsolete dashboard queries** - If a metric is removed from code, remove it from dashboards too

**Recent Metric Changes:**
- `aprs.aircraft.aircraft_lookup_ms` → `aprs.aircraft.aircraft_upsert_ms` (2025-01-07, PR #312)
  - Updated in code and Grafana dashboard (2025-01-12)
- **REMOVED**: `aprs.elevation.dropped` and `nats_publisher.dropped_fixes` (2025-01-12)
  - These metrics were removed from dashboard as messages can no longer be dropped

**Grafana Alerting:**
- **Alert Configuration** - Managed via infrastructure as code in `infrastructure/grafana-provisioning/alerting/`
- **Email Notifications** - Alerts sent via SMTP (credentials from `/etc/soar/env` or `/etc/soar/env-staging`)
- **Template Files** - Use `.template` suffix for files with credential placeholders (e.g., `contact-points.yml.template`)
- **Deployment** - `soar-deploy` script automatically processes templates and installs configs
- **Documentation** - See `infrastructure/GRAFANA-ALERTING.md` for complete guide
- **NEVER commit credentials** - Template files use placeholders, actual values extracted during deployment

### Frontend Development Standards

#### Static Site Generation (CRITICAL)
- **NO Server-Side Rendering (SSR) ANYWHERE** - The frontend MUST be compiled statically
- Use `export const ssr = false;` in `+page.ts` files to disable SSR for specific pages
- The compiled static site is embedded in the Rust binary for deployment
- All pages must work as a pure client-side Single Page Application (SPA)
- Authentication and route protection must be handled client-side

#### Svelte 5 Syntax (REQUIRED)
```svelte
<!--  CORRECT: Use Svelte 5 event handlers -->
<button onclick={handleClick}>Click me</button>
<input oninput={handleInput} onkeydown={handleKeydown} />

<!-- L WRONG: Don't use Svelte 4 syntax -->
<button on:click={handleClick}>Click me</button>
<input on:input={handleInput} on:keydown={handleKeydown} />
```

#### Icons (REQUIRED)
```svelte
<!--  CORRECT: Use @lucide/svelte exclusively -->
import { Search, User, Settings, ChevronDown } from '@lucide/svelte';

<!-- L WRONG: Don't use other icon libraries -->
```

#### Component Libraries
- **Skeleton UI**: Use `@skeletonlabs/skeleton-svelte` components (Svelte 5 compatible)
- **Tailwind CSS**: Use utility-first CSS approach
- **TypeScript**: Full type safety required

### Backend Development Standards

#### Rust Code Quality (REQUIRED)
- **ALWAYS run `cargo fmt`** after editing Rust files to ensure consistent formatting
- **Pre-commit hooks automatically run `cargo fmt`** - but format manually for immediate feedback
- **Use `cargo clippy`** to catch common issues and improve code quality
- All Rust code must pass formatting, clippy, and tests before commit
- **For local testing**: Use `cargo build` (debug build), not `cargo build --release` unless you specifically need release optimizations

#### Rust Patterns
```rust
//  Use anyhow::Result for error handling
use anyhow::Result;

//  Use tracing for logging
use tracing::{info, warn, error, debug};

//  Proper async function signatures
pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    // Handler implementation
}
```

#### Database Patterns
```rust
//  Use Diesel ORM patterns
use diesel::prelude::*;

//  PostGIS integration
use postgis_diesel::geography::Geography;
```

## Technology-Specific Documentation

### Svelte 5 + Skeleton UI

Reference the official Skeleton UI documentation for Svelte 5 components:
- **Skeleton UI Svelte 5 Guide**: https://www.skeleton.dev/llms-svelte.txt

## Project Architecture

### Database Layer (PostgreSQL + PostGIS)
```sql
--  Spatial data patterns
CREATE TABLE airports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    location GEOGRAPHY(POINT, 4326) NOT NULL,
    elevation_ft INTEGER
);

--  Indexes for spatial queries
CREATE INDEX CONCURRENTLY idx_airports_location ON airports USING GIST (location);
```

### API Layer (Rust + Axum)
```rust
//  Route structure
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub nats_client: Arc<async_nats::Client>,
}

//  Handler patterns
pub async fn get_aircraft(
    State(state): State<AppState>,
    Query(params): Query<DeviceSearchParams>,
) -> Result<impl IntoResponse, ApiError> {
    // Implementation
}
```

### Frontend Layer (SvelteKit + TypeScript)
```svelte
<!--  Component structure -->
<script lang="ts">
    import { Search, Filter } from '@lucide/svelte';
    import { Segment } from '@skeletonlabs/skeleton-svelte';

    let searchQuery = '';

    function handleSearch() {
        // Implementation using onclick, not on:click
    }
</script>

<button onclick={handleSearch} class="btn variant-filled-primary">
    <Search class="h-4 w-4" />
    Search
</button>
```

### Real-time Features (NATS)
```rust
//  NATS message patterns
#[derive(Serialize, Deserialize)]
pub struct LiveFix {
    pub aircraft_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: String,
}
```

### Analytics Layer
SOAR includes a comprehensive analytics system for tracking flight statistics and performance metrics.

**Database Tables:**
- `flight_analytics_daily` - Daily flight aggregations with automatic triggers
- `flight_analytics_hourly` - Hourly flight statistics for recent trends
- Materialized automatically via PostgreSQL triggers on flight insert/update

**Backend Components:**
- `src/analytics.rs` - Core data models for analytics responses
- `src/analytics_repo.rs` - Database queries using Diesel ORM
- `src/analytics_cache.rs` - 60-second TTL cache using Moka
- `src/actions/analytics.rs` - REST API endpoints

**API Endpoints (`/data/analytics/...`):**
- `/flights/daily` - Daily flight counts and statistics
- `/flights/hourly` - Hourly flight trends
- `/flights/duration-distribution` - Flight duration buckets
- `/devices/outliers` - Devices with anomalous flight patterns (z-score > threshold)
- `/devices/top` - Top devices by flight count
- `/clubs/daily` - Club-level analytics
- `/airports/activity` - Airport usage statistics
- `/summary` - Dashboard summary with key metrics

**Caching Strategy:**
- All queries cached for 60 seconds to reduce database load
- Cache hit/miss metrics tracked in `analytics.cache.hit` and `analytics.cache.miss`
- Query latency tracked per endpoint (e.g., `analytics.query.daily_flights_ms`)

**Metrics & Monitoring:**
```rust
// Analytics API metrics
metrics::counter!("analytics.api.daily_flights.requests").increment(1);
metrics::counter!("analytics.api.errors").increment(1);
metrics::histogram!("analytics.query.daily_flights_ms").record(duration_ms);
metrics::counter!("analytics.cache.hit").increment(1);

// Background task updates these gauges every 60 seconds
metrics::gauge!("analytics.flights.today").set(summary.flights_today as f64);
metrics::gauge!("analytics.flights.last_7d").set(summary.flights_7d as f64);
metrics::gauge!("analytics.aircraft.active_7d").set(summary.active_aircraft_7d as f64);
```

**Grafana Dashboard:**
- Location: `infrastructure/grafana-dashboard-analytics.json`
- Tracks: API request rates, cache hit rates, query latency percentiles, error rates
- Background task runs every 60 seconds to update summary metrics

**Adding New Analytics:**
1. Add database query to `analytics_repo.rs`
2. Add caching method to `analytics_cache.rs` with metrics
3. Add API handler to `actions/analytics.rs` with request/error metrics
4. Register route in `web.rs`
5. Add metrics to `metrics.rs::initialize_analytics_metrics()`
6. Update Grafana dashboard with new metric queries

## Code Quality Standards

### Pre-commit Hooks (REQUIRED)
All changes must pass these checks locally:

1. **Rust Quality**:
   - `cargo fmt --check` (formatting)
   - `cargo clippy --all-targets --all-features -- -D warnings` (linting)
   - `cargo test --verbose` (unit tests)
   - `cargo audit` (security audit)

2. **Frontend Quality**:
   - `npm run lint` (ESLint + Prettier)
   - `npm run check` (TypeScript validation)
   - `npm test` (Playwright E2E tests)
   - `npm run build` (build verification)

3. **File Quality**:
   - No trailing whitespace
   - Proper file endings
   - Valid YAML/JSON/TOML syntax

### MCP Server Setup (Optional - For Claude Code Database Access)

Claude Code can connect directly to the PostgreSQL database using the **pgEdge Postgres MCP Server**, enabling natural language database queries and schema introspection.

**Installation:**

1. **Clone and build the MCP server:**
   ```bash
   cd /tmp
   git clone https://github.com/pgEdge/pgedge-postgres-mcp.git
   cd pgedge-postgres-mcp
   go build -v -o bin/pgedge-postgres-mcp ./cmd/pgedge-pg-mcp-svr
   cp bin/pgedge-postgres-mcp ~/.local/bin/
   ```

2. **Configure for your project:**
   ```bash
   # Copy the example configuration
   cp .mcp.json.example .mcp.json

   # Edit .mcp.json with your settings
   # Update the "command" path to where you installed the binary
   # Update "PGUSER" to your PostgreSQL username
   ```

3. **Restart Claude Code** to load the MCP server

**What you get:**
- 🔍 Schema introspection - Query tables, columns, indexes, constraints
- 📊 Database queries - Execute SQL queries (read-only for safety)
- 📈 Performance metrics - Access `pg_stat_statements` and other stats
- 🧠 Natural language queries - Ask questions about the database in plain English

**Security Notes:**
- The MCP server runs in **read-only mode** by default
- `.mcp.json` is gitignored to prevent committing local paths and credentials
- Use `.pgpass` file for password management instead of storing in `.mcp.json`

**Documentation:** https://www.pgedge.com/blog/introducing-the-pgedge-postgres-mcp-server

## Common Patterns

### Error Handling
```rust
//  Rust error handling
use anyhow::{Context, Result};

pub async fn process_data() -> Result<ProcessedData> {
    let data = fetch_data()
        .await
        .context("Failed to fetch data")?;

    Ok(process(data))
}
```

```typescript
//  TypeScript error handling
try {
    const response = await serverCall<AircraftResponse>('/devices');
    devices = response.devices || [];
} catch (err) {
    const errorMessage = err instanceof Error ? err.message : 'Unknown error';
    error = `Failed to load devices: ${errorMessage}`;
}
```

### State Management
```svelte
<!--  Svelte stores -->
<script lang="ts">
    import { writable } from 'svelte/store';

    const aircraftStore = writable<Aircraft[]>([]);

    // Use $aircraftStore for reactive access
</script>
```

### API Integration
```typescript
//  Server communication
import { serverCall } from '$lib/api/server';

const response = await serverCall<AircraftListResponse>('/devices', {
    method: 'GET',
    params: { limit: 50 }
});
```

## Security Requirements

1. **Input Validation**: All user inputs must be validated
2. **SQL Injection Prevention**: Use Diesel ORM query builder
3. **XSS Prevention**: Proper HTML escaping in Svelte
4. **Authentication**: JWT tokens for API access
5. **HTTPS Only**: All production traffic encrypted

## Testing Requirements

### Rust Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aircraft_search() {
        // Test implementation
    }
}
```

### Frontend E2E Tests (Playwright)

- **Framework**: Playwright v1.56+
- **Test Directory**: `web/e2e/`
- **Documentation**: See `web/e2e/README.md` for comprehensive testing guide
- **Running tests**: `cd web && npm test`

## Performance Guidelines

1. **Database**: Use proper indexes, limit query results
2. **Frontend**: Lazy loading, virtual scrolling for large lists
3. **API**: Pagination for large datasets
4. **Real-time**: Efficient NATS subscription management

---

**Remember**: This project maintains high code quality standards. All changes must pass pre-commit hooks and CI/CD pipeline. When in doubt, check existing patterns and follow established conventions.
- The rust backend for this project is in src/ and the frontend is a Svelte 5 project in web/
- You should absolutely never use --no-verify
- When running clippy or cargo build, set the timeout to ten minutes
- Use a timeout of 10 minutes for running "cargo test" or "cargo clippy"

## Branch Protection Rules

**CRITICAL**: The `main` branch is protected and does not allow direct commits.

### Always Use Feature Branches
- **NEVER commit directly to main** - Always create a feature/topic branch first
- Use descriptive branch names:
  - `feature/description` for new features
  - `fix/description` for bug fixes
  - `refactor/description` for code refactoring
  - `docs/description` for documentation changes

### Proper Development Workflow
1. **Create a topic branch**: `git checkout -b feature/my-feature`
2. **Make changes and commit**: Only stage specific files you modified
3. **Push to remote**: `git push origin feature/my-feature`
4. **Create Pull Request**: Use GitHub UI to create PR for review

### If You Accidentally Commit to Main
If you accidentally commit to main, follow these steps to fix it:
1. `git reset --hard HEAD~N` (where N is the number of commits to undo)
2. `git checkout -b topic/branch-name commit-hash` (create branch for each commit)
3. `git checkout main` (return to main)

Example:
```bash
# You accidentally made 3 commits to main
git log --oneline -3  # See the commits
# c3c3c3c third commit
# b2b2b2b second commit
# a1a1a1a first commit

# Undo the commits on main
git reset --hard HEAD~3

# Create branches for each
git checkout -b feature/third-feature c3c3c3c
git checkout -b feature/second-feature b2b2b2b
git checkout -b fix/first-fix a1a1a1a

# Return to main
git checkout main
```
- Any time that you have enter a filename in the shell, use quotes. We are using zsh, and shell expansion treats our Svelte pages incorrectly if you do not because "[]" has a special meaning.
- Whenever you commit, you should also push with -u to set the upstream branch.
- When opening a PR, do not set it to squash commits. Set it to create a merge commit.

## Release Process

**IMPORTANT**: Version numbers are automatically derived from git tags. Do NOT manually edit version numbers in `Cargo.toml` or `package.json`.

### How Versioning Works

- **Rust**: Version is derived from git tags at build time using the `vergen` crate
- **SvelteKit**: Version is generated from git tags by `web/scripts/generate-version.js` during prebuild
- **Source of Truth**: Git tags (format: `v0.1.5`)
- **No Version Commits**: Version files (`Cargo.toml`, `package.json`) contain placeholder values only

### Creating a Release

Use the simplified release script:

```bash
# Semantic version bump (recommended)
./scripts/create-release patch    # 0.1.4 → 0.1.5
./scripts/create-release minor    # 0.1.4 → 0.2.0
./scripts/create-release major    # 0.1.4 → 1.0.0

# Explicit version (if needed)
./scripts/create-release v0.1.5

# Create as draft (for review before publishing)
./scripts/create-release patch --draft

# Create with custom release notes
./scripts/create-release patch --notes "Custom release notes here"
```

**What happens automatically:**
1. ✅ Script creates GitHub Release with tag `v0.1.5`
2. ✅ CI builds x64 and ARM64 static binaries (version derived from tag)
3. ✅ CI runs all tests and security audits
4. ✅ CI deploys to production automatically (via `ci.yml` `deploy-production` job)

### Manual Release via GitHub CLI

```bash
# Alternative: Use gh CLI directly
gh release create v0.1.5 --generate-notes
```

### Version Format

- **Tagged release**: `v0.1.4`
- **Development build**: `v0.1.4-2-ge930185` (2 commits after v0.1.4)
- **Dirty working tree**: `v0.1.4-dirty`
- **No git repo**: `0.0.0-dev`

### Checking Current Version

```bash
# Binary version (shows git-derived version)
./target/release/soar --version

# Git describe (shows current version from git)
git describe --tags --always --dirty
```

### Old Release Process (Deprecated)

The old `./scripts/release` script has been archived to `./scripts/archive/release-old`. It is no longer needed because:
- ❌ Required creating release branch → PR → auto-merge → tag push
- ❌ Manually edited version files and committed changes
- ❌ Complex multi-step process with potential for errors

The new process eliminates all of this complexity.
