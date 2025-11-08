# Refactoring Plan: Too Many Arguments

This document analyzes all `#[allow(clippy::too_many_arguments)]` in the codebase and proposes refactoring solutions.

## Summary

Found 11 instances of `too_many_arguments` warnings across 7 files. They fall into these categories:

1. **Flight Processing Context** (4 instances) - Repository and state management
2. **Location Builders** (2 instances) - Address field construction
3. **View Builders** (2 instances) - Display model construction
4. **Command Handlers** (2 instances) - CLI entry points
5. **Database Updates** (1 instance) - Repository update methods

---

## Category 1: Flight Processing Context (PRIORITY 1)

### Problem
Multiple functions in the flight tracking system pass around the same 7-10 repositories and state maps:
- `flights_repo: &FlightsRepository`
- `device_repo: &DeviceRepository`
- `airports_repo: &AirportsRepository`
- `locations_repo: &LocationsRepository`
- `runways_repo: &RunwaysRepository`
- `fixes_repo: &FixesRepository`
- `elevation_db: &ElevationDB`
- `active_flights: &ActiveFlightsMap`
- `device_locks: &DeviceLocksMap`
- `aircraft_trackers: &AircraftTrackersMap`

### Affected Files
1. **src/flight_tracker/state_transitions.rs:74** - `process_state_transition()` (10 args)
2. **src/flight_tracker/flight_lifecycle.rs:24** - `create_flight()` (10 args)
3. **src/flight_tracker/flight_lifecycle.rs:170** - `complete_flight()` (10 args)
4. **src/flight_tracker/mod.rs:397** - Calling `process_state_transition()` (passes 10 args)

### Solution: Create `FlightProcessorContext` Struct

**Note:** `FlightTracker` struct at src/flight_tracker/mod.rs:85 already contains most of these fields! We should leverage this.

```rust
/// Context for flight processing operations
/// Contains all repositories and state needed for flight lifecycle management
pub struct FlightProcessorContext<'a> {
    pub flights_repo: &'a FlightsRepository,
    pub device_repo: &'a DeviceRepository,
    pub airports_repo: &'a AirportsRepository,
    pub locations_repo: &'a LocationsRepository,
    pub runways_repo: &'a RunwaysRepository,
    pub fixes_repo: &'a FixesRepository,
    pub elevation_db: &'a ElevationDB,
    pub active_flights: &'a ActiveFlightsMap,
    pub device_locks: &'a DeviceLocksMap,
    pub aircraft_trackers: &'a AircraftTrackersMap,
}
```

Or even better, add a method to `FlightTracker`:

```rust
impl FlightTracker {
    /// Get a context reference for flight processing operations
    pub fn context(&self) -> FlightProcessorContext {
        FlightProcessorContext {
            flights_repo: &self.flights_repo,
            device_repo: &self.device_repo,
            airports_repo: &self.airports_repo,
            locations_repo: &self.locations_repo,
            runways_repo: &self.runways_repo,
            fixes_repo: &self.fixes_repo,
            elevation_db: &self.elevation_db,
            active_flights: &self.active_flights,
            device_locks: &self.device_locks,
            aircraft_trackers: &self.aircraft_trackers,
        }
    }
}
```

**Refactored function signatures:**

```rust
// Before:
pub(crate) async fn process_state_transition(
    flights_repo: &FlightsRepository,
    device_repo: &DeviceRepository,
    airports_repo: &AirportsRepository,
    locations_repo: &LocationsRepository,
    runways_repo: &RunwaysRepository,
    fixes_repo: &FixesRepository,
    elevation_db: &ElevationDB,
    active_flights: &ActiveFlightsMap,
    device_locks: &DeviceLocksMap,
    aircraft_trackers: &AircraftTrackersMap,
    mut fix: Fix,
) -> Result<Fix>

// After:
pub(crate) async fn process_state_transition(
    ctx: &FlightProcessorContext<'_>,
    mut fix: Fix,
) -> Result<Fix>

// Or with self method:
impl FlightTracker {
    pub async fn process_fix(&self, mut fix: Fix) -> Result<Fix> {
        state_transitions::process_state_transition(&self.context(), fix).await
    }
}
```

**Impact:**
- Reduces 10-11 arguments to 2-3 arguments
- Makes the code much more maintainable
- Easier to add new dependencies in the future
- Clearer separation of concerns

---

## Category 2: Location Builders (PRIORITY 3)

### Problem
Location construction functions accept 8-9 optional address fields individually.

### Affected Files
1. **src/locations.rs:166** - `Location::new()` (8 args)
2. **src/locations_repo.rs:192** - `LocationsRepository::find_or_create()` (8 args)
3. **src/actions/views/club.rs:22** - `create_location_from_fields()` (12 args!)

### Solution: Create `AddressFields` Struct

```rust
/// Address components for location creation
#[derive(Debug, Clone, Default)]
pub struct AddressFields {
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub region_code: Option<String>,
    pub country_mail_code: Option<String>,
    pub geolocation: Option<Point>,
}

impl AddressFields {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_street1(mut self, street: impl Into<String>) -> Self {
        self.street1 = Some(street.into());
        self
    }

    // ... other builder methods
}
```

**Refactored function signatures:**

```rust
// Before:
impl Location {
    pub fn new(
        street1: Option<String>,
        street2: Option<String>,
        city: Option<String>,
        state: Option<String>,
        zip_code: Option<String>,
        region_code: Option<String>,
        country_mail_code: Option<String>,
        geolocation: Option<Point>,
    ) -> Self

// After:
impl Location {
    pub fn new(address: AddressFields) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            street1: address.street1,
            street2: address.street2,
            city: address.city,
            state: address.state,
            zip_code: address.zip_code,
            region_code: address.region_code,
            country_mail_code: address.country_mail_code,
            geolocation: address.geolocation,
            created_at: now,
            updated_at: now,
        }
    }
}
```

**Impact:**
- Reduces 8-12 arguments to 1 argument
- Provides builder pattern for ergonomic construction
- Makes it easier to pass address data around
- Could potentially reuse this struct in API models

---

## Category 3: View Builders (PRIORITY 4)

### Problem
`FlightView` constructors accept many optional display fields.

### Affected Files
1. **src/actions/views/flight.rs:99** - `FlightView::from_flight_full()` (9 args)
2. **src/actions/views/flight.rs:163** - `FlightView::from_flight_with_altitude()` (7 args)

### Solution: Create `FlightViewOptions` Struct

```rust
/// Optional display fields for FlightView construction
#[derive(Debug, Clone, Default)]
pub struct FlightViewOptions {
    pub departure_airport: Option<AirportInfo>,
    pub arrival_airport: Option<AirportInfo>,
    pub device_info: Option<DeviceInfo>,
    pub latest_altitude_msl_feet: Option<i32>,
    pub latest_altitude_agl_feet: Option<i32>,
    pub latest_fix_timestamp: Option<DateTime<Utc>>,
    pub previous_flight_id: Option<Uuid>,
    pub next_flight_id: Option<Uuid>,
}

impl FlightViewOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_departure_airport(mut self, airport: AirportInfo) -> Self {
        self.departure_airport = Some(airport);
        self
    }

    // ... other builder methods
}
```

**Refactored function signatures:**

```rust
// Before:
impl FlightView {
    pub fn from_flight_full(
        flight: Flight,
        departure_airport: Option<AirportInfo>,
        arrival_airport: Option<AirportInfo>,
        device_info: Option<DeviceInfo>,
        latest_altitude_msl_feet: Option<i32>,
        latest_altitude_agl_feet: Option<i32>,
        latest_fix_timestamp: Option<DateTime<Utc>>,
        previous_flight_id: Option<Uuid>,
        next_flight_id: Option<Uuid>,
    ) -> Self

// After:
impl FlightView {
    pub fn new(flight: Flight, options: FlightViewOptions) -> Self {
        let state = flight.state();
        let duration_seconds = match (flight.takeoff_time, flight.landing_time) {
            (Some(takeoff), Some(landing)) => Some((landing - takeoff).num_seconds()),
            _ => None,
        };

        Self {
            id: flight.id,
            departure_airport: options.departure_airport.unwrap_or_default(),
            arrival_airport: options.arrival_airport.unwrap_or_default(),
            device_info: options.device_info.unwrap_or_default(),
            latest_altitude_msl_feet: options.latest_altitude_msl_feet,
            // ... rest of fields
        }
    }
}
```

**Impact:**
- Reduces 7-9 arguments to 2 arguments
- Provides builder pattern for flexibility
- Makes it clear which fields are optional display enhancements

---

## Category 4: Command Handlers (PRIORITY 5 - OK AS IS)

### Problem
CLI entry points accept many configuration parameters.

### Affected Files
1. **src/commands/run.rs:108** - `handle_run()` (3 args currently, but marked)
2. **src/commands/load_data/mod.rs:41** - `handle_load_data()` (9 args)

### Assessment
These are CLI entry points that mirror command-line arguments. Having many parameters here is acceptable because:
- They're the boundary between CLI parsing and business logic
- Each parameter represents a distinct configuration option
- They're called from a single location (CLI parser)

### Alternative Solution (Optional)
If desired, could create config structs:

```rust
pub struct RunConfig {
    pub archive_dir: Option<String>,
    pub nats_url: String,
    pub diesel_pool: Pool<ConnectionManager<PgConnection>>,
}

pub struct LoadDataConfig {
    pub diesel_pool: Pool<ConnectionManager<PgConnection>>,
    pub aircraft_models_path: Option<String>,
    pub aircraft_registrations_path: Option<String>,
    pub airports_path: Option<String>,
    pub runways_path: Option<String>,
    pub receivers_path: Option<String>,
    pub devices_path: Option<String>,
    pub geocode: bool,
    pub link_home_bases: bool,
}
```

**Recommendation:** Low priority. These are fine as-is, but could be refactored for consistency if desired.

---

## Category 5: Database Updates (PRIORITY 2)

### Problem
Repository update method accepts 10 optional parameters for flight landing updates.

### Affected Files
1. **src/flights_repo.rs:74** - `FlightsRepository::update_flight_landing()` (10 args)

### Solution: Create `FlightLandingUpdate` Struct

```rust
/// Parameters for updating flight landing information
#[derive(Debug, Clone)]
pub struct FlightLandingUpdate {
    pub landing_time: DateTime<Utc>,
    pub arrival_airport_id: Option<i32>,
    pub landing_location_id: Option<Uuid>,
    pub landing_altitude_offset_ft: Option<i32>,
    pub landing_runway_ident: Option<String>,
    pub total_distance_meters: Option<f64>,
    pub maximum_displacement_meters: Option<f64>,
    pub runways_inferred: Option<bool>,
    pub last_fix_at: Option<DateTime<Utc>>,
}

impl FlightLandingUpdate {
    pub fn new(landing_time: DateTime<Utc>) -> Self {
        Self {
            landing_time,
            arrival_airport_id: None,
            landing_location_id: None,
            landing_altitude_offset_ft: None,
            landing_runway_ident: None,
            total_distance_meters: None,
            maximum_displacement_meters: None,
            runways_inferred: None,
            last_fix_at: None,
        }
    }

    pub fn with_airport(mut self, airport_id: i32) -> Self {
        self.arrival_airport_id = Some(airport_id);
        self
    }

    // ... other builder methods
}
```

**Refactored function signatures:**

```rust
// Before:
impl FlightsRepository {
    pub async fn update_flight_landing(
        &self,
        flight_id: Uuid,
        landing_time_param: DateTime<Utc>,
        arrival_airport_id_param: Option<i32>,
        landing_location_id_param: Option<Uuid>,
        landing_altitude_offset_ft_param: Option<i32>,
        landing_runway_ident_param: Option<String>,
        total_distance_meters_param: Option<f64>,
        maximum_displacement_meters_param: Option<f64>,
        runways_inferred_param: Option<bool>,
        last_fix_at_param: Option<DateTime<Utc>>,
    ) -> Result<()>

// After:
impl FlightsRepository {
    pub async fn update_flight_landing(
        &self,
        flight_id: Uuid,
        update: FlightLandingUpdate,
    ) -> Result<()> {
        // Use update.landing_time, update.arrival_airport_id, etc.
    }
}
```

**Impact:**
- Reduces 11 arguments to 2 arguments
- Makes it clear this is a cohesive update operation
- Easier to extend with new fields in the future
- Provides builder pattern for constructing updates

---

## Implementation Priority

1. **PRIORITY 1: Flight Processing Context** - High impact, used throughout flight processing
2. **PRIORITY 2: Database Updates** - Improves repository API clarity
3. **PRIORITY 3: Location Builders** - Moderate impact, used in multiple places
4. **PRIORITY 4: View Builders** - Low impact, mostly internal to views
5. **PRIORITY 5: Command Handlers** - Optional, acceptable as-is

## Next Steps

1. Start with **Flight Processing Context** refactoring:
   - Create `FlightProcessorContext` struct (or leverage existing `FlightTracker`)
   - Refactor `process_state_transition()` in state_transitions.rs
   - Refactor `create_flight()` in flight_lifecycle.rs
   - Refactor `complete_flight()` in flight_lifecycle.rs
   - Update call sites in mod.rs

2. Refactor **Database Updates**:
   - Create `FlightLandingUpdate` struct in flights_repo.rs or flights.rs
   - Update `update_flight_landing()` method
   - Update call sites in flight_lifecycle.rs

3. Refactor **Location Builders** if time permits:
   - Create `AddressFields` struct in locations.rs
   - Update `Location::new()`
   - Update `LocationsRepository::find_or_create()`
   - Update `create_location_from_fields()` in club.rs

4. Consider **View Builders** and **Command Handlers** as future improvements.
