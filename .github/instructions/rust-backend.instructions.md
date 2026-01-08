---
applyTo: "src/**/*.rs"
---

# Rust Backend Code Standards

## Critical Rules

1. **Always use Diesel ORM** - Never write raw SQL unless absolutely necessary
   ```rust
   // ✅ CORRECT: Use Diesel query builder
   use diesel::prelude::*;
   devices.filter(device_id.eq(id)).first::<Device>(conn)

   // ❌ WRONG: Don't use raw SQL
   diesel::sql_query("SELECT * FROM devices WHERE device_id = ?")
   ```

2. **Error Handling Pattern** - Always use `anyhow::Result` with context
   ```rust
   use anyhow::{Context, Result};

   pub async fn process_data() -> Result<ProcessedData> {
       let data = fetch_data()
           .await
           .context("Failed to fetch data")?;
       Ok(process(data))
   }
   ```

3. **Logging with tracing** - Use `tracing` crate, not `println!`
   ```rust
   use tracing::{info, warn, error, debug};

   info!("Processing flight {}", flight_id);
   error!("Failed to process fix: {}", err);
   ```

4. **Metrics Convention**
   - Use dot notation: `aprs.aircraft.device_upsert_ms`
   - Always update corresponding Grafana dashboards in `infrastructure/`
   - Document metric changes in commit messages

## Database Operations

### Diesel Query Patterns

```rust
// Simple query with optional result
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

## Axum API Handlers

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

## NATS Messaging Patterns

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

## Testing

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

## Common Pitfalls to Avoid

1. ❌ Writing raw SQL instead of using Diesel query builder
2. ❌ Using `println!` for logging instead of `tracing`
3. ❌ Missing error context in `Result` types
4. ❌ Forgetting to update Grafana dashboards when changing metrics
5. ❌ Not running `cargo fmt` after editing files
6. ❌ Using `.unwrap()` in production code without justification

## Before Committing

- Run `cargo fmt` to format code
- Run `cargo clippy` to catch common mistakes
- Run `cargo test` to ensure tests pass
- Update Grafana dashboards if metrics changed
