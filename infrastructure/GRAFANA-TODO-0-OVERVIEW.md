# Grafana Dashboard Improvements - Overview

## Completed ✅

1. **Fixed duplicate `_total_total` metric suffixes** (Commit: d3e8f058)
   - Coverage dashboard
   - Geocoding dashboard
   - Routing dashboard
   - Added Pelias metrics initialization

2. **Improved NATS Infrastructure dashboard** (Commit: 1ed604dc)
   - Added ingest-adsb metrics
   - Clear source labels for all metrics
   - Full-width panels, vertical stacking
   - Updated topic documentation

3. **Improved Flight Analytics dashboard** (Commit: c5084dde)
   - Added environment selector (staging/production)
   - Dynamic postgres datasource selection
   - Renamed Device → Aircraft terminology
   - Full-width panels, vertical stacking

## Remaining Work

### Priority 1: Code Changes (Required First)
**Document:** `GRAFANA-TODO-2-CODE-CHANGES.md`

Must implement these before dashboard changes can work:
- Add coalesce speed metric (NEW)
- Fix missing coalesce distance P50 data
- Fix missing aircraft processing latency data

**Estimated time:** 1-2 hours
**Dependencies:** None, can start immediately

### Priority 2: Flight Analytics SQL Changes
**Document:** `GRAFANA-TODO-1-ANALYTICS.md`

Complex SQL query updates:
- Fix glider flights logic (filter by ogn_aircraft_type)
- Add hex addresses to aircraft panels
- Convert distance to miles (integer)
- Add "truncated" flight type category
- Add new "Daily Flights (Last 30 Days)" panel

**Estimated time:** 1 hour
**Dependencies:** Database schema knowledge

### Priority 3: Individual Dashboard Fixes
**Document:** `GRAFANA-TODO-3-DASHBOARD-FIXES.md`

- BEAST → ADS-B renaming
- Coverage dashboard environment selector
- Core System latency fixes
- Queue closed errors documentation
- Move receiver cache panel

**Estimated time:** 1 hour
**Dependencies:** Priority 1 (code changes)

### Priority 4: Dashboard Reorganization
**Document:** `GRAFANA-TODO-4-REORGANIZATION.md`

Major structural changes:
- Merge Data Ingestion Pipeline into Ingest dashboards
- Move flight panels from Routing to Flight Tracking
- Consolidate related metrics

**Estimated time:** 2-3 hours
**Dependencies:** Should do last (affects multiple dashboards)

## Recommended Execution Order

### Session 1: Code Changes
Start here - enables everything else
- File: `GRAFANA-TODO-2-CODE-CHANGES.md`
- Implement coalesce speed metric
- Fix missing metrics issues
- Test with `cargo build` and verify metrics endpoint

### Session 2: Analytics Dashboard
High-value SQL improvements
- File: `GRAFANA-TODO-1-ANALYTICS.md`
- Fix glider logic
- Add hex addresses
- Distance unit conversions

### Session 3: Individual Fixes
Quick wins across dashboards
- File: `GRAFANA-TODO-3-DASHBOARD-FIXES.md`
- ADSB renaming
- Documentation panels
- Latency fixes

### Session 4: Reorganization
Final cleanup and consolidation
- File: `GRAFANA-TODO-4-REORGANIZATION.md`
- Move panels between dashboards
- Delete redundant dashboards
- Update documentation

## Testing Strategy

After each session:
1. Validate JSON: `python3 -m json.tool <file> > /dev/null`
2. Check metrics exist: `curl localhost:9092/metrics | grep <metric>`
3. Load in Grafana and verify panels show data
4. Commit and push changes

## Notes

- All dashboard files are in `infrastructure/`
- Metrics defined in `src/metrics.rs`
- Pre-commit hooks will format and validate JSON
- Use environment selector pattern from Analytics dashboard
- Test with both staging and production data sources
