# Grafana TODO 1: Flight Analytics Dashboard - Complex SQL Changes

**File:** `infrastructure/grafana-dashboard-analytics.json`

## Status
- ✅ Added environment selector (staging/production)
- ✅ Renamed Device → Aircraft terminology
- ✅ Made all panels full width
- ⏳ Complex SQL changes needed (below)

## Required Changes

### 1. Fix "Daily Flights" Panel - Add Glider Type Filtering
**Current:** Shows "Glider Flights" and "Tow Flights" but glider count is wrong (counts all non-tow flights)
**Problem:** Not filtering by `ogn_aircraft_type = 'glider'`

**Action:**
- Rename panel title: "Daily Flights (Last 30 Days)" → "**Daily Flights (Gliding)**"
- Keep both series: "Glider Flights" and "Tow Flights"
- Fix SQL to filter glider flights properly:
  ```sql
  SELECT
    date as time,
    (SELECT COUNT(*) FROM flights f
     JOIN devices d ON f.device_id = d.id
     WHERE f.takeoff_time::date = fad.date
     AND d.ogn_aircraft_type = 'glider') as "Glider Flights",
    tow_count as "Tow Flights"
  FROM flight_analytics_daily fad
  WHERE date >= CURRENT_DATE - 30
  ORDER BY date ASC
  ```

### 2. Add New Panel: "Daily Flights (Last 30 Days)" - All Flight Types
**Action:**
- Create NEW panel below the gliding panel
- Show total flights regardless of type
- Simple query:
  ```sql
  SELECT
    date as time,
    flight_count as "Total Flights"
  FROM flight_analytics_daily
  WHERE date >= CURRENT_DATE - 30
  ORDER BY date ASC
  ```

### 3. Top 10 Most Active Aircraft - Add Hex Addresses
**Current SQL:** Shows registration and model
**Action:** Add hex address column
```sql
SELECT
  d.registration || ' (' || d.address || ')' as "Aircraft",
  d.model as "Model",
  COUNT(*) as "Flights"
FROM devices d
JOIN flights f ON f.device_id = d.id
WHERE f.takeoff_time >= NOW() - INTERVAL '30 days'
GROUP BY d.id, d.registration, d.address, d.model
ORDER BY COUNT(*) DESC
LIMIT 10
```

### 4. Total Distance Flown - Convert to Miles (Integer)
**Current:** Uses metric prefix (km, Mm, etc.)
**Action:**
- Change unit to just "mi" (miles) without metric prefix
- Convert in SQL: `total_distance_km * 0.621371`
- Cast to integer: `CAST(total_distance_km * 0.621371 AS INTEGER)`
- Update panel field config: `"unit": "short"` (not "lengthmi")

### 5. Average Flight Duration Trend - Add P50
**Current:** Shows only mean
**Action:** Add P50 (median) series if possible without database changes
- Check if `flight_analytics_daily` has percentile data
- If not, note that this requires database schema changes (skip for now)

### 6. Flight Type Distribution - Add "Truncated" Category
**Current:** Shows "Local Flights" and "Cross-Country Flights"
**Problem:** Many flights missing takeoff/landing airports aren't shown
**Action:**
- Add third category: "Truncated" (missing takeoff_airport, landing_airport, or both)
- Update SQL:
  ```sql
  SELECT
    flight_type,
    COUNT(*) as count
  FROM (
    SELECT
      CASE
        WHEN takeoff_airport_id IS NULL OR landing_airport_id IS NULL THEN 'Truncated'
        WHEN takeoff_airport_id = landing_airport_id THEN 'Local'
        ELSE 'Cross-Country'
      END as flight_type
    FROM flights
    WHERE takeoff_time >= NOW() - INTERVAL '30 days'
  ) AS categorized
  GROUP BY flight_type
  ```

### 7. Anomalous Aircraft Activity - Add Hex Address
**Current:** Shows registration and model
**Action:** Include hex address in display
```sql
SELECT
  d.registration || ' (' || d.address || ')' as "Aircraft",
  d.model as "Model",
  z_score_30d as "Z-Score"
FROM aircraft_analytics aa
JOIN devices d ON aa.device_id = d.id
WHERE z_score_30d > 3.0
ORDER BY z_score_30d DESC
```

## Notes
- All SQL changes must work with both staging and production postgres
- Use `${postgres_datasource}` for datasource UID (already done)
- Test queries work with current database schema
