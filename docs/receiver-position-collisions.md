# Receiver Position Collisions

Investigated 2026-03-17. Multiple OGN receivers share the same callsign, causing their position to "ping-pong" in our database as interleaved position beacons overwrite each other.

## How It Works

OGN receivers broadcast position beacons every 5 minutes via APRS. SOAR identifies receivers by callsign and updates their stored position on each beacon. When two physically distinct receivers share a callsign, each beacon overwrites the other's position, producing a warning like:

```
WARN soar::ogn::receiver_position: Receiver YARA moved 823.9 km: (-33.9180, 151.0985) -> (-37.3080, 142.9883)
```

The position update still proceeds (warning only), so the DB position oscillates continuously.

## Impact

- **Database churn**: receiver location flips every few minutes
- **Log spam**: 340 warnings in 2 days from ~12 receivers
- **Incorrect receiver metadata**: downstream uses of receiver position are unreliable for affected receivers
- **No impact on aircraft data**: aircraft positions come from the aircraft's own report, not the receiver

## Confirmed Callsign Collisions

### YARA — 217 warnings, ongoing

Two receivers in Australia.

| | Receiver A (Sydney) | Receiver B (Victoria) |
|---|---|---|
| Position | 33°55.08'S, 151°05.91'E | 37°18.48'S, 142°59.30'E |
| Elevation | 98 ft | 984 ft |
| Gateway | GLIDERN1 | GLIDERN5 |
| RAM | 524 MB | 4,026 MB |
| CPU Temp | ~12°C | ~44°C |
| Distance | **823.9 km** apart ||

Raw APRS beacons:
```
YARA>OGNSDR,TCPIP*,qAC,GLIDERN1:/185037h3355.08SI15105.91E&/A=000098
YARA>OGNSDR,TCPIP*,qAC,GLIDERN5:/180324h3718.48SI14259.30E&/A=000984
```

### AVX993 — 59 warnings, stopped ~Mar 16

Two Avionix receivers: Netherlands and Norway.

| | Receiver A (Netherlands) | Receiver B (Norway) |
|---|---|---|
| Position | 51°33.27'N, 004°32.89'E | 59°54.15'N, 010°37.41'E |
| Elevation | 56 ft | 138 ft |
| Gateway | GLIDERN2 | GLIDERN3 |
| RAM | ~195 MB | ~290 MB |
| Distance | **1,002.2 km** apart ||

Raw APRS beacons:
```
AVX993>OGNSDR,TCPIP*,qAC,GLIDERN2:/225218h5133.27NI00432.89E&/A=000056
AVX993>OGNSDR,TCPIP*,qAC,GLIDERN3:/225232h5954.15NI01037.41E&/A=000138
```

### AVX986 — 19 warnings

Two Avionix receivers in Norway, 36 km apart.

| | Receiver A | Receiver B |
|---|---|---|
| Position | 60°48.16'N, 010°40.85'E | 61°06.75'N, 010°28.39'E |
| Elevation | 640 ft | 886 ft |
| NTP offset | +1.3 ppm | -101 ppm |
| Distance | **36.2 km** apart ||

## Likely Mobile/Shipborne Receivers

These show sequential incremental movement along the Norwegian coast, consistent with receivers on ferries or ships:

| Receiver | Warnings | Area | Movement |
|---|---|---|---|
| AVX1247 | 9 | Ålesund coast (62.4-62.8°N) | 17-74 km steps |
| AVX1044 | 4 | Southern Norway (58.1-58.4°N) | 22-28 km steps |
| AVX1048 | 2 | Norwegian coast (62.7-62.8°N) | 18-30 km steps |
| AVX1045 | 2 | Eastern Norway (60.0-60.3°N) | 21 km steps |

## Other

| Receiver | Warnings | Notes |
|---|---|---|
| AVX1231 | 9 | Multiple positions around Réunion/Mauritius (37-243 km jumps) |
| FNBFBAD40 | 13 | Flarm Network Tracker (OGNFNT), moving east across Germany/Austria at ~47.92°N |
| CYSA3 | 4 | Ontario, Canada, 3-4 positions within 18 km (Kitchener area) |
| AVX979 | 1 | One-time 1,758 km jump: Poland → Spain |
| OGN307223 | 1 | One-time 6 km jump, French Alps |

## Code Reference

The warning is emitted in `src/ogn/receiver_position.rs` at the 5 km threshold (`RECEIVER_MOVE_WARNING_THRESHOLD_M`). Currently the position update proceeds unconditionally after the warning — the move is not rejected.

## Diagnostic Queries

Check recent warnings:
```bash
sudo journalctl -u soar-run --since "1 hour ago" | grep "receiver.*moved"
```

Find raw APRS position beacons for a receiver:
```sql
SELECT rm.received_at, convert_from(rm.raw_message, 'UTF-8') as raw_msg
FROM raw_messages rm
JOIN receivers r ON rm.receiver_id = r.id
WHERE r.callsign = 'YARA'
  AND rm.received_at >= NOW() - INTERVAL '1 day'
  AND rm.source = 'aprs'
  AND convert_from(rm.raw_message, 'UTF-8') LIKE 'YARA>OGNSDR%/%'
ORDER BY rm.received_at DESC
LIMIT 20;
```

The APRS gateway field (e.g., `qAC,GLIDERN1` vs `qAC,GLIDERN5`) distinguishes which physical device sent the beacon.
