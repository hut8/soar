# Grafana TODO 4: Dashboard Reorganization

## Overview
Several dashboards have panels in wrong locations or duplicate content. This document tracks what needs to move where.

## 1. Data Ingestion Pipeline → Merge into Ingest Dashboards

**File:** `infrastructure/grafana-dashboard-run-ingestion.json`

### Problem
"SOAR Run - Data Ingestion Pipeline" dashboard is redundant - content belongs in:
- `grafana-dashboard-ingest-ogn.json` (OGN/APRS ingestion)
- `grafana-dashboard-ingest-adsb.json` (ADS-B/Beast ingestion)

### Action Plan

#### A. Identify all panels in run-ingestion.json
List panel titles and their metrics:
```bash
jq '.panels[] | {title: .title, id: .id}' grafana-dashboard-run-ingestion.json
```

#### B. Categorize panels
For each panel, determine:
- **OGN-related:** Metrics with `aprs.*`, `ogn.*` → move to `ingest-ogn.json`
- **ADS-B-related:** Metrics with `beast.*`, `adsb.*` → move to `ingest-adsb.json`
- **Generic/Both:** Duplicate into both dashboards if needed

#### C. Move panels
1. Copy panel JSON from `run-ingestion.json`
2. Paste into appropriate dashboard
3. Update `gridPos` (x, y coordinates) to stack properly
4. Ensure panel IDs don't conflict (increment if needed)

#### D. Delete source dashboard
Once all panels moved:
- Delete `grafana-dashboard-run-ingestion.json`
- Remove from `grafana-provisioning/dashboards/dashboards.yml`

## 2. Packet Processing and Routing → Move to Aircraft & Flight Tracking

**File:** `infrastructure/grafana-dashboard-run-routing.json`

### Problem
"SOAR Run - Packet Processing and Routing" has panels that belong in:
- `grafana-dashboard-run-flights.json` (Aircraft & Flight Tracking)

### Panels to Move

#### A. "Flight Creation" Panel
**Current location:** `run-routing.json`
**Should be in:** `run-flights.json` (Aircraft & Flight Tracking)
**Reason:** Flight creation is core flight tracking, not routing

**Metrics:**
- `flight_tracker.flight_created.takeoff_total`
- `flight_tracker.flight_created.airborne_total`

#### B. Other Flight-Related Panels
Search for panels with metrics starting with `flight_tracker.*`:
- These likely belong in the Flight Tracking dashboard
- Routing dashboard should focus on NATS message routing, not flight logic

### Action Plan

#### A. Audit run-routing.json
```bash
# List all panels and their metrics
jq '.panels[] | {
  title: .title,
  metrics: [.targets[]?.expr // empty]
}' grafana-dashboard-run-routing.json
```

#### B. Categorize panels

**Keep in routing.json (message routing/NATS):**
- NATS consumer metrics: `aprs.nats.consumed_total`, `beast.nats.consumed_total`
- Message processing rates
- Queue depths and errors
- NATS-specific latency

**Move to run-flights.json (flight tracking logic):**
- Flight creation/ending
- Flight state transitions
- Coalescing logic
- Flight timeout detection
- Aircraft-specific processing

#### C. Move panels
1. Copy panel JSON
2. Add to `run-flights.json`
3. Update `gridPos` for proper stacking
4. Verify panel IDs unique

#### D. Review remaining content
- If routing dashboard becomes too sparse, consider merging into run-core.json
- Keep focused: routing = NATS message flow, not business logic

## 3. Receiver Cache Panel → Move from Elevation to Core

**Source:** `infrastructure/grafana-dashboard-run-elevation.json`
**Target:** `infrastructure/grafana-dashboard-run-core.json`

### Panel Details
**Metrics:**
- `generic_processor.receiver_cache.hit_total`
- `generic_processor.receiver_cache.miss_total`

**Why move:** Receiver cache is general infrastructure, not elevation-specific

### Action
1. Find panel in elevation dashboard
2. Copy full panel JSON
3. Add to Core dashboard in "Infrastructure" or "Caching" section
4. Delete from elevation dashboard
5. Test both dashboards render correctly

## Implementation Order

1. **Start simple:** Move receiver cache panel (single panel, clear destination)
2. **Flight Creation panel:** Single panel from routing to flights
3. **Audit routing dashboard:** Categorize all panels systematically
4. **Audit ingestion dashboard:** Categorize all panels systematically
5. **Bulk moves:** Move multiple panels at once
6. **Cleanup:** Delete empty/redundant dashboards
7. **Test:** Verify all dashboards load and show data

## Testing Checklist

After reorganization:
- [ ] All dashboards load without errors in Grafana
- [ ] No duplicate panels across dashboards
- [ ] Panel IDs are unique within each dashboard
- [ ] All metrics still display correctly
- [ ] gridPos coordinates don't cause overlapping panels
- [ ] Dashboard navigation makes logical sense
- [ ] Update CLAUDE.md to reflect new dashboard structure

## Files to Update

- `infrastructure/grafana-dashboard-run-ingestion.json` (delete after migration)
- `infrastructure/grafana-dashboard-ingest-ogn.json` (add OGN panels)
- `infrastructure/grafana-dashboard-ingest-adsb.json` (add ADS-B panels)
- `infrastructure/grafana-dashboard-run-routing.json` (remove flight panels)
- `infrastructure/grafana-dashboard-run-flights.json` (add flight panels)
- `infrastructure/grafana-dashboard-run-elevation.json` (remove receiver cache)
- `infrastructure/grafana-dashboard-run-core.json` (add receiver cache)
- `infrastructure/grafana-provisioning/dashboards/dashboards.yml` (remove deleted dashboard)
- `CLAUDE.md` (update dashboard documentation)
