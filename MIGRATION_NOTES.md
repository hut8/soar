# Locations Table Normalization - Migration Notes

## Overview
The locations table normalization removes redundant address columns from both `clubs` and `aircraft_registrations` tables, replacing them with references to a centralized `locations` table.

## Database Changes

### New Table: `locations`
- `id` (UUID, Primary Key)
- `street1`, `street2`, `city`, `state`, `zip_code`, `region_code`, `county_mail_code`, `country_mail_code` (TEXT)
- `geolocation` (POINT, nullable)
- `created_at`, `updated_at` (TIMESTAMPTZ)

### Modified Tables:
- `clubs` → Added `location_id` (UUID, Foreign Key)
- `aircraft_registrations` → Added `location_id` (UUID, Foreign Key)

### Removed Columns (after migrations 20250907000004 and 20250907000005):
**From `clubs`:**
- `street1`, `street2`, `city`, `state`, `zip_code`, `region_code`, `county_mail_code`, `country_mail_code`, `base_location`

**From `aircraft_registrations`:**
- `street1`, `street2`, `city`, `state`, `zip_code`, `region_code`, `county_mail_code`, `country_mail_code`, `registered_location`

## Required Rust Code Changes

### 1. Update Club Struct (`src/clubs.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub is_soaring: Option<bool>,
    pub home_base_airport_id: Option<i32>,
    pub location_id: Option<Uuid>,  // NEW: Reference to locations table
    // REMOVED: All address fields and base_location
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 2. Update Aircraft Struct (`src/faa/aircraft_registrations.rs`)
```rust
pub struct Aircraft {
    // ... existing fields ...
    pub location_id: Option<Uuid>,  // NEW: Reference to locations table
    // REMOVED: All address fields and registered_location
}
```

### 3. Update Repository Queries
All queries that previously accessed address fields directly will need to JOIN with the locations table:

```sql
-- Example: Get club with address
SELECT c.*, l.street1, l.city, l.state, l.geolocation
FROM clubs c
LEFT JOIN locations l ON c.location_id = l.id
WHERE c.id = $1
```

### 4. Update Club Creation Logic
The `find_or_create_club` method in `aircraft_registrations_repo.rs` will need to:
1. Use `LocationsRepository::find_or_create()` to get a location
2. Create the club with the `location_id`

### 5. Update Geocoding Logic
Geocoding should now work with the `locations` table directly:
- `LocationsRepository::get_locations_for_geocoding()`
- `LocationsRepository::update_geolocation()`

## Migration Order
1. `20250907000002` - Create locations table and add foreign keys
2. `20250907000004` - Drop redundant columns from clubs
3. `20250907000005` - Drop redundant columns from aircraft_registrations

## Benefits After Migration
- ✅ Eliminates data redundancy
- ✅ Centralized geocoding (when aircraft address is geocoded, club gets coordinates too)
- ✅ Consistent address data across tables
- ✅ Better query performance with proper indexes
- ✅ Single source of truth for location data

## Rollback
Each migration has a corresponding `.down.sql` file that can restore the previous structure if needed.