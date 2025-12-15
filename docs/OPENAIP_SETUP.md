# OpenAIP Airspace Integration Setup Guide

This guide explains how to set up and use the OpenAIP airspace data integration in SOAR.

## Overview

SOAR integrates with OpenAIP's REST API to import airspace boundary data and display it on the operations map. This provides pilots with visual awareness of controlled airspace, restricted areas, and other airspace classifications.

## Important: Data License

**OpenAIP data is licensed under CC BY-NC 4.0 (Creative Commons Attribution-NonCommercial 4.0 International)**

This means:
- ✅ You may use the data for non-commercial purposes
- ✅ You must provide attribution to OpenAIP
- ❌ You may NOT use this data for commercial purposes without separate licensing

**If you plan to use SOAR commercially, you must obtain a commercial license from OpenAIP.**

For more information: https://www.openaip.net/legal

## Getting Your API Key

### Step 1: Create an OpenAIP Account

1. Go to https://www.openaip.net/
2. Click "Sign Up" or "Register"
3. Create a free account with your email address
4. Verify your email address

### Step 2: Generate API Client Credentials

1. Log in to your OpenAIP account
2. Navigate to https://www.openaip.net/users/clients
3. Click "Create New Client" or "Add Client"
4. Fill in the required information:
   - **Name**: SOAR Airspace Integration (or any descriptive name)
   - **Description**: Import airspace data for SOAR flight tracking system
5. Click "Create" or "Save"
6. **IMPORTANT**: Copy your API key immediately - it may only be shown once!

### Step 3: Configure SOAR

1. Open your `.env` file in the SOAR project root
2. Add your OpenAIP API key:
   ```bash
   OPENAIP_API_KEY=your_api_key_here
   ```
3. Save the file

**Security Note**: Never commit your `.env` file to version control. The API key should be kept secret.

## Importing Airspace Data

### Initial Import

To perform a complete import of all airspace data globally:

```bash
./target/release/soar pull-airspaces
```

This command will:
- Fetch all airspace data from OpenAIP's API
- Use pagination to retrieve all records (500 per page)
- Rate-limit requests (100ms delay between pages)
- Store airspace boundaries in PostgreSQL with PostGIS geometry
- Log the sync operation in the `airspace_sync_log` table

**Expected Duration**: 15-30 minutes depending on network speed and total airspace count.

### Country-Specific Import

To import airspaces for specific countries only:

```bash
./target/release/soar pull-airspaces --countries US,CA,MX
```

Use ISO 3166-1 alpha-2 country codes (US, CA, GB, DE, FR, etc.).

### Incremental Sync

To only fetch airspaces that have been updated since your last successful sync:

```bash
./target/release/soar pull-airspaces --incremental
```

This uses OpenAIP's `updatedAfter` parameter to minimize data transfer and processing time.

**Recommended**: Set up a cron job to run incremental syncs daily:

```bash
# Run at 3 AM daily
0 3 * * * cd /path/to/soar && ./target/release/soar pull-airspaces --incremental
```

## Viewing Airspaces on the Map

Once airspace data is imported:

1. Navigate to the Operations page (`/operations`)
2. Airspaces will automatically load when you zoom in to a specific area
3. **Visibility Threshold**: Airspaces only appear when the map viewport is less than 100,000 square miles
4. **Color Coding**:
   - **Red**: Class A, B, C, D (Controlled airspace)
   - **Amber**: Class E (Controlled airspace - Class E)
   - **Green**: Class F, G (Uncontrolled airspace)
   - **Gray**: Special Use Airspace (SUA) and other types

### Airspace Details

Click on any airspace boundary to see:
- Airspace name
- Classification (A, B, C, D, E, F, G, SUA)
- Type (CTR, Restricted, Danger, Prohibited, etc.)
- Lower altitude limit
- Upper altitude limit
- Remarks (if available)

### Toggle Airspace Display

You can enable/disable airspace boundaries via the Settings modal:

1. Click the Settings button (⚙️) on the operations map
2. Toggle "Show Airspace Boundaries" on or off
3. Setting is saved to your user preferences

## Database Schema

Airspace data is stored in two tables:

### `airspaces` Table

Stores airspace boundary geometries and metadata:

```sql
CREATE TABLE airspaces (
    id UUID PRIMARY KEY,
    openaip_id INTEGER UNIQUE,
    name TEXT,
    airspace_class airspace_class,  -- Enum: A, B, C, D, E, F, G, SUA
    airspace_type airspace_type,    -- Enum: CTR, Restricted, Danger, etc.
    country_code CHAR(2),

    -- Altitude limits
    lower_value INTEGER,
    lower_unit TEXT,
    lower_reference altitude_reference,  -- Enum: MSL, AGL, STD, GND, UNL
    upper_value INTEGER,
    upper_unit TEXT,
    upper_reference altitude_reference,

    -- PostGIS geometry (MultiPolygon)
    geometry GEOGRAPHY(MultiPolygon, 4326),
    geometry_geom GEOMETRY(MultiPolygon, 4326) GENERATED ALWAYS AS (geometry::geometry) STORED,

    -- Metadata
    remarks TEXT,
    activity_type TEXT,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    openaip_updated_at TIMESTAMPTZ
);
```

**Spatial Indexes**: GIST indexes on both `geometry` and `geometry_geom` for fast bounding box queries.

### `airspace_sync_log` Table

Tracks sync operations for auditing and incremental updates:

```sql
CREATE TABLE airspace_sync_log (
    id UUID PRIMARY KEY,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    success BOOLEAN,
    airspaces_fetched INTEGER,
    airspaces_inserted INTEGER,
    countries_filter TEXT[],
    updated_after TIMESTAMPTZ
);
```

## API Endpoint

Airspace data is available via REST API:

```
GET /data/airspaces?west={west}&south={south}&east={east}&north={north}&limit={limit}
```

**Parameters**:
- `west`: Western longitude boundary (-180 to 180)
- `south`: Southern latitude boundary (-90 to 90)
- `east`: Eastern longitude boundary (-180 to 180)
- `north`: Northern latitude boundary (-90 to 90)
- `limit`: Maximum results (optional, defaults to 500)

**Response**: GeoJSON FeatureCollection

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "MultiPolygon",
        "coordinates": [...]
      },
      "properties": {
        "id": "uuid",
        "openaip_id": 12345,
        "name": "San Francisco Class B",
        "airspace_class": "B",
        "airspace_type": "CTR",
        "lower_limit": "SFC",
        "upper_limit": "10000 FT MSL",
        "remarks": null,
        "country_code": "US",
        "activity_type": null
      }
    }
  ]
}
```

## Monitoring

Prometheus metrics are recorded for airspace operations:

### CLI Command Metrics

- `airspace_sync.total_fetched` - Total airspaces fetched from OpenAIP
- `airspace_sync.total_inserted` - Total airspaces inserted/updated in database
- `airspace_sync.last_run_timestamp` - Unix timestamp of last sync
- `airspace_sync.success` - Whether last sync succeeded (1) or failed (0)

### API Endpoint Metrics

- `api.airspaces.requests` - Total API requests to `/data/airspaces`
- `api.airspaces.duration_ms` - Query duration histogram
- `api.airspaces.results_count` - Number of airspaces returned
- `api.airspaces.errors` - Error count

Access metrics at: http://localhost:9090 (if Prometheus is configured)

## Troubleshooting

### API Key Issues

**Problem**: `OPENAIP_API_KEY environment variable not set`

**Solution**: Ensure you've added `OPENAIP_API_KEY=your_key_here` to your `.env` file.

**Problem**: `401 Unauthorized` or `403 Forbidden`

**Solution**:
- Verify your API key is correct
- Check that your OpenAIP account is active
- Ensure you haven't exceeded rate limits

### Import Issues

**Problem**: Import is very slow

**Solution**:
- This is normal for global imports (15-30 minutes)
- Consider country-specific imports for faster initial setup
- Use `--incremental` for subsequent updates

**Problem**: `Database connection failed`

**Solution**:
- Ensure PostgreSQL is running
- Verify `DATABASE_URL` in `.env` is correct
- Check that migrations have been run: `diesel migration run`

**Problem**: Airspaces don't appear on map

**Solution**:
1. Verify data was imported: `psql -d soar_dev -c "SELECT COUNT(*) FROM airspaces;"`
2. Check that you're zoomed in enough (< 100,000 sq miles viewport)
3. Verify "Show Airspace Boundaries" is enabled in Settings
4. Check browser console for JavaScript errors

### PostGIS Issues

**Problem**: `type "geography" does not exist`

**Solution**: Install PostGIS extension:
```sql
CREATE EXTENSION IF NOT EXISTS postgis;
```

**Problem**: Geometry conversion errors

**Solution**: Verify PostGIS version is 3.0+:
```sql
SELECT PostGIS_Version();
```

## Performance Optimization

### Database Tuning

For large airspace datasets (100,000+ records):

```sql
-- Increase work_mem for better query performance
SET work_mem = '256MB';

-- Analyze tables after import
ANALYZE airspaces;
ANALYZE airspace_sync_log;
```

### Frontend Performance

- Airspaces are only loaded when viewport is < 100,000 sq miles
- API requests are debounced (500ms) to prevent excessive calls during panning/zooming
- Results are limited to 500 airspaces per request
- Polygons use low z-index (50) to keep aircraft/airports visible on top

## OpenAIP API Reference

- **Base URL**: https://api.core.openaip.net
- **Authentication**: `x-openaip-api-key` header
- **Rate Limits**: Typically 100 requests per minute (check current limits in API docs)
- **Pagination**: `page` (1-indexed), `limit` (max 500)
- **Documentation**: https://www.openaip.net/api-docs

## Data Updates

OpenAIP updates airspace data regularly as NOTAMs and aeronautical publications change. Recommended sync schedule:

- **Daily**: Incremental sync (captures updates and new airspaces)
- **Weekly**: Full sync (ensures data consistency)
- **As needed**: Country-specific updates when notified of major airspace changes

## Support

For issues with:
- **SOAR integration**: File a GitHub issue at https://github.com/hut8/soar/issues
- **OpenAIP API**: Contact OpenAIP support at https://www.openaip.net/support
- **Data licensing**: Contact OpenAIP at https://www.openaip.net/contact

## Additional Resources

- OpenAIP Website: https://www.openaip.net/
- OpenAIP API Documentation: https://www.openaip.net/api-docs
- OpenAIP Data License: https://www.openaip.net/legal
- PostGIS Documentation: https://postgis.net/docs/
- GeoJSON Specification: https://geojson.org/
