# GitHub Copilot Instructions for SOAR Project

This file provides specific instructions to help GitHub Copilot generate better code suggestions for the SOAR (Soaring Observation And Records) aircraft tracking system.

## Project Context

SOAR is a comprehensive aircraft tracking and club management system with:
- **Backend**: Rust with Axum web framework, PostgreSQL with PostGIS
- **Frontend**: SvelteKit 5 with TypeScript, Tailwind CSS, Skeleton UI
- **Real-time**: NATS messaging for live aircraft position updates
- **Data Sources**: OGN/APRS-IS integration, ADS-B Beast format, FAA aircraft registry

## Critical Code Quality Rules

### Rust Backend Standards

1. **Always use Diesel ORM** - Never write raw SQL unless absolutely necessary
   ```rust
   // ✅ CORRECT: Use Diesel query builder
   use diesel::prelude::*;
   devices.filter(device_id.eq(id)).first::<Device>(conn)

   // ❌ WRONG: Don't use raw SQL
   diesel::sql_query("SELECT * FROM devices WHERE device_id = ?")
   ```

2. **Error Handling Pattern**
   ```rust
   use anyhow::{Context, Result};

   pub async fn process_data() -> Result<ProcessedData> {
       let data = fetch_data()
           .await
           .context("Failed to fetch data")?;
       Ok(process(data))
   }
   ```

3. **Logging with tracing**
   ```rust
   use tracing::{info, warn, error, debug};

   info!("Processing flight {}", flight_id);
   error!("Failed to process fix: {}", err);
   ```

4. **Metrics Convention**
   - Use dot notation: `aprs.aircraft.device_upsert_ms`
   - Always update corresponding Grafana dashboards in `infrastructure/`
   - Document metric changes in commit messages

5. **Database Migrations**
   - Never use `CREATE INDEX CONCURRENTLY` in migrations (not supported in transactions)
   - Use regular `CREATE INDEX` instead
   - All schema changes must go through Diesel migrations

### Frontend Standards (SvelteKit 5)

1. **Svelte 5 Event Syntax** (CRITICAL)
   ```svelte
   <!-- ✅ CORRECT: Use Svelte 5 inline event handlers -->
   <button onclick={handleClick}>Click me</button>
   <input oninput={handleInput} onkeydown={handleKeydown} />

   <!-- ❌ WRONG: Don't use Svelte 4 syntax -->
   <button on:click={handleClick}>Click me</button>
   ```

2. **Icons from Lucide**
   ```svelte
   <!-- ✅ CORRECT: Use @lucide/svelte exclusively -->
   <script lang="ts">
       import { Search, User, Settings } from '@lucide/svelte';
   </script>
   ```

3. **Skeleton UI Components**
   ```svelte
   <script lang="ts">
       import { Button, Modal } from '@skeletonlabs/skeleton-svelte';
   </script>
   ```

4. **Static Site Generation**
   - NO Server-Side Rendering (SSR) - frontend must be fully static
   - Use `export const ssr = false;` in `+page.ts` files
   - All pages work as pure client-side SPA
   - Authentication handled client-side

5. **API Communication**
   ```typescript
   import { serverCall } from '$lib/api/server';

   const response = await serverCall<AircraftResponse>('/devices', {
       method: 'GET',
       params: { limit: 50 }
   });
   ```

## Common Patterns

### Database Queries (Diesel)

```rust
// Simple query
use crate::schema::devices::dsl::*;
devices
    .filter(device_id.eq(id))
    .first::<Device>(conn)
    .optional()?

// Join with filter
use crate::schema::{devices, flights};
devices::table
    .inner_join(flights::table)
    .filter(devices::club_id.eq(club_id))
    .select(Device::as_select())
    .load(conn)?

// Spatial query with PostGIS
use postgis_diesel::geography::Geography;
airports::table
    .filter(
        dsl::sql::<diesel::sql_types::Bool>(
            &format!("ST_DWithin(location, ST_GeographyFromText('SRID=4326;POINT({} {})'), {})",
                lon, lat, radius_meters)
        )
    )
    .load(conn)?
```

### Axum API Handlers

```rust
use axum::{
    extract::{State, Query, Path},
    response::{IntoResponse, Json},
    http::StatusCode,
};

pub async fn get_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let device = state.device_repo
        .find_by_id(&device_id)
        .await
        .context("Failed to fetch device")?
        .ok_or_else(|| ApiError::NotFound("Device not found".into()))?;

    Ok(Json(device))
}
```

### TypeScript Error Handling

```typescript
try {
    const response = await serverCall<AircraftResponse>('/devices');
    devices = response.devices || [];
} catch (err) {
    const errorMessage = err instanceof Error ? err.message : 'Unknown error';
    error = `Failed to load devices: ${errorMessage}`;
}
```

### NATS Messaging

```rust
// Publishing messages
#[derive(Serialize, Deserialize)]
pub struct LiveFix {
    pub aircraft_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: String,
}

nats_client
    .publish(
        format!("aircraft.fix.{}", aircraft_id),
        serde_json::to_vec(&fix)?.into()
    )
    .await?;

// Subscribing to messages
let mut subscriber = nats_client
    .subscribe("ogn.raw")
    .await?;

while let Some(msg) = subscriber.next().await {
    let packet = parse_aprs_packet(&msg.payload)?;
    process_packet(packet).await?;
}
```

## Testing Patterns

### Rust Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callsign() {
        let callsign = parse_callsign("FLRDD1234").unwrap();
        assert_eq!(callsign.prefix, "FLR");
        assert_eq!(callsign.id, "DD1234");
    }

    #[tokio::test]
    async fn test_aircraft_upsert() {
        let pool = setup_test_db().await;
        let device = create_test_device();
        let result = upsert_device(&pool, &device).await;
        assert!(result.is_ok());
    }
}
```

### Playwright E2E Tests

```typescript
import { test, expect } from '@playwright/test';

test('aircraft search filters results', async ({ page }) => {
    await page.goto('/devices');
    await page.fill('[placeholder="Search devices"]', 'FLRDD');
    await page.click('button[type="submit"]');
    await expect(page.locator('.device-card')).toHaveCount(1);
});
```

## Architecture Patterns

### Data Flow

1. **Ingestion** (`soar ingest-ogn`): OGN APRS-IS → Raw Queue → NATS Publisher
2. **Processing** (`soar run`): NATS Subscriber → Router → Type-specific Queues → Processors → Database
3. **Real-time** (Fix Processor): Database → Live Fix Queue → NATS Publisher → WebSocket clients
4. **API** (`soar web`): REST endpoints → Database queries → JSON responses

### Repository Pattern

```rust
pub struct DeviceRepository {
    pool: PgPool,
}

impl DeviceRepository {
    pub async fn find_by_id(&self, device_id: &str) -> Result<Option<Device>> {
        use crate::schema::devices::dsl::*;
        let mut conn = self.pool.get()?;
        devices
            .filter(device_id.eq(device_id))
            .first::<Device>(&mut conn)
            .optional()
            .context("Failed to query device")
    }

    pub async fn upsert(&self, device: &Device) -> Result<Device> {
        use crate::schema::devices::dsl::*;
        let mut conn = self.pool.get()?;
        diesel::insert_into(devices)
            .values(device)
            .on_conflict(device_id)
            .do_update()
            .set(device)
            .get_result(&mut conn)
            .context("Failed to upsert device")
    }
}
```

### State Management (Axum)

```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub nats_client: Arc<async_nats::Client>,
    pub config: Arc<Config>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/devices", get(list_devices))
        .route("/devices/:id", get(get_device))
        .with_state(state)
}
```

## Performance Considerations

1. **Database Indexes** - Always create indexes for frequently queried columns
   ```sql
   CREATE INDEX idx_devices_club_id ON devices(club_id);
   CREATE INDEX idx_fixes_device_timestamp ON fixes(device_id, timestamp DESC);
   ```

2. **Spatial Indexes** - Use GIST indexes for PostGIS columns
   ```sql
   CREATE INDEX idx_airports_location ON airports USING GIST (location);
   ```

3. **Query Pagination** - Always limit results for large datasets
   ```rust
   devices
       .limit(50)
       .offset(page * 50)
       .load(conn)?
   ```

4. **Caching Strategy** - Use Moka cache with TTL
   ```rust
   use moka::future::Cache;
   let cache: Cache<String, Device> = Cache::builder()
       .time_to_live(Duration::from_secs(60))
       .build();
   ```

## Security Requirements

1. **Input Validation** - Validate all user inputs
2. **SQL Injection Prevention** - Use Diesel query builder only
3. **XSS Prevention** - Svelte automatically escapes HTML
4. **Authentication** - JWT tokens for API access
5. **HTTPS Only** - All production traffic encrypted

## Git Workflow

1. **Never commit to main** - Always use feature branches
2. **Branch naming**:
   - `feature/description` for new features
   - `fix/description` for bug fixes
   - `refactor/description` for refactoring
   - `docs/description` for documentation

3. **Commit messages**: Clear, descriptive, present tense
   - "Add aircraft search endpoint"
   - "Fix flight detection timeout issue"
   - "Update Grafana dashboard for new metrics"

4. **Pre-commit hooks** - Never use `--no-verify`

## Documentation Requirements

1. **Update documentation with code changes** - Documentation is part of the implementation
2. **API documentation** - Document all public endpoints
3. **Metric changes** - Update Grafana dashboards when changing metrics
4. **Migration notes** - Document complex database migrations

## Common Pitfalls to Avoid

1. ❌ Using Svelte 4 syntax (`on:click`) instead of Svelte 5 (`onclick`)
2. ❌ Writing raw SQL instead of using Diesel query builder
3. ❌ Forgetting to update Grafana dashboards when changing metrics
4. ❌ Using `CREATE INDEX CONCURRENTLY` in Diesel migrations
5. ❌ Committing directly to main branch
6. ❌ Using `git commit --no-verify` to skip pre-commit hooks
7. ❌ Not running `cargo fmt` after editing Rust files
8. ❌ Enabling SSR in SvelteKit pages (must be static only)

## File Naming Conventions

- **Rust**: `snake_case.rs` (e.g., `flight_tracker.rs`)
- **TypeScript/Svelte**: `kebab-case.ts`, `PascalCase.svelte` for components
- **Routes**: `+page.svelte`, `+page.ts`, `+layout.svelte`
- **Tests**: `*.test.ts` for frontend, `#[cfg(test)]` modules for Rust

## Environment Variables

Required in `.env`:
```bash
DATABASE_URL=postgres://user:password@localhost:5432/soar_dev
NATS_URL=nats://localhost:4222
JWT_SECRET=your-secret-here
SMTP_SERVER=localhost
SMTP_PORT=1025
FROM_EMAIL=noreply@soar.local
FROM_NAME=SOAR
BASE_URL=http://localhost:4173
```

## Quick Reference

### Build Commands
- `cargo build` - Debug build (fast, for development)
- `cargo build --release` - Release build (slow, optimized)
- `cd web && npm run build` - Frontend build

### Test Commands
- `cargo test` - Rust tests
- `cargo nextest run` - Faster test runner
- `cd web && npm test` - E2E tests

### Quality Commands
- `cargo fmt` - Format Rust code
- `cargo clippy` - Rust linter
- `cd web && npm run lint` - Frontend linter
- `pre-commit run --all-files` - All checks

### Development Commands
- `cargo run -- web` - Start web server
- `cargo run -- ingest-ogn` - Start OGN ingestion
- `cargo run -- run` - Start message processor
- `cd web && npm run dev` - Start SvelteKit dev server

## Additional Resources

- **CLAUDE.md**: Comprehensive AI assistant guide with all critical rules
- **README.md**: Project overview and setup
- **CI.md**: CI/CD pipeline documentation
- **docs/FLIGHT-DETECTION-TESTING.md**: Flight tracking testing guide
- **web/e2e/README.md**: E2E testing guide
- **Skeleton UI Docs**: https://www.skeleton.dev/llms-svelte.txt
