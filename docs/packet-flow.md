# Packet Flow: APRS, Beast, and SBS

How each message type flows from ingestion through to database writes. This document exists to make the architecture legible for future simplification.

## Shared entry point

All three message types arrive as protobuf `Envelope` messages via the socket server or APRS/Beast/SBS clients. They enter a single **envelope queue** (capacity 200). A single **envelope router task** (`src/commands/run/mod.rs:388`) dequeues envelopes and routes them by `envelope.source()` to one of three intake queues.

This router is sequential — if any downstream intake queue is full, the entire router blocks, starving the other two protocols.

---

## APRS

### Queues and workers

| Stage | Queue | Capacity | Workers | File |
|-------|-------|----------|---------|------|
| 1. Envelope intake | `envelope_rx` | 200 | 1 (envelope router) | `commands/run/mod.rs:388` |
| 2. OGN intake | `ogn_intake` | 200 | **1** | `commands/run/workers.rs:21` |
| 3. PacketRouter internal | `internal_queue` | 1,000 | 50 | `packet_processors/router.rs:83` |
| 4. Aircraft position | `aircraft_position` | 500 | 80 | `commands/run/workers.rs:174` |

Plus 3 low-traffic queues: receiver status (50 cap, 6 workers), receiver position (50 cap, 4 workers), server status (50 cap, 2 workers).

### Call chain

```
Envelope Router
  └─ ogn_intake_tx.send_async()            # blocks if OGN intake full

OGN Intake Worker (1 worker)
  └─ process_aprs_message()                # commands/run/aprs.rs:7
     ├─ ogn_parser::parse(message)         # parse APRS
     └─ packet_router.process_packet()     # sends to PacketRouter internal queue
        └─ internal_queue_tx.send_async()  # blocks if internal queue full (capacity 1,000)

PacketRouter Worker (50 workers)
  └─ process_packet_internal()             # packet_processors/router.rs:262
     ├─ GenericProcessor.process_packet()  # DB: receiver upsert + raw_message insert
     │   ├─ archive raw message (if enabled)
     │   ├─ identify receiver callsign (from packet.via)
     │   ├─ DB: INSERT receiver (moka cache, 100k capacity, 24h TTL)
     │   ├─ DB: INSERT INTO raw_messages (type='aprs')
     │   └─ return PacketContext { raw_message_id, receiver_id, received_at }
     │
     └─ Route by packet type:
        ├─ Position + Aircraft → aircraft_position_tx.send_async()
        ├─ Position + Receiver → receiver_position_tx.send_async()
        └─ Status → receiver_status_tx.send_async()

Aircraft Position Worker (80 workers)
  └─ AircraftPositionProcessor.process_aircraft_position()  # packet_processors/aircraft.rs:47
     └─ FixProcessor.process_aprs_packet()                  # fix_processor.rs:160
        │
        ├─ Extract device address, address_type from OGN ID field
        ├─ Filter: suppressed APRS types, suppressed categories, zero address
        ├─ DB: AircraftCache.get_or_upsert(address)
        │   └─ Cache miss → DB: SELECT/INSERT aircraft + optional DDB lookup
        ├─ Fix::from_aprs_packet() → creates Fix struct
        │
        └─ process_fix_internal()                           # fix_processor.rs:346
           ├─ Elevation: ElevationService.query_elevation() (sync, from disk)
           │   └─ Recalculate is_active with AGL data
           │
           ├─ FlightTracker.process_and_insert_fix()        # flight_tracker/mod.rs:583
           │   ├─ DashMap: device_locks.entry(aircraft_id) → get per-device mutex
           │   ├─ Lock: per-device tokio::Mutex (held for entire function)
           │   ├─ DashMap: aircraft_states.get() → duplicate check (<1s)
           │   ├─ state_transitions::process_state_transition()
           │   │   ├─ DB: AircraftCache.get_by_id(aircraft_id)
           │   │   ├─ DashMap: aircraft_states.entry() → get/create AircraftState
           │   │   ├─ Flight state machine:
           │   │   │   ├─ Active + has flight → continue (or split on callsign change / gap)
           │   │   │   ├─ Active + no flight → DB: INSERT flight (takeoff)
           │   │   │   ├─ Inactive + has flight → DB: UPDATE flight landing_time
           │   │   │   └─ Inactive + no flight → no-op
           │   │   └─ DashMap: aircraft_states.get_mut() → update state
           │   │
           │   ├─ DB: INSERT fix
           │   ├─ Spawn: DB: UPDATE flights SET last_fix_at (async, not under lock)
           │   ├─ Geofence check (while still under lock):
           │   │   ├─ DashMap: aircraft_states.get() → previous geofence status
           │   │   ├─ DB: geofence queries
           │   │   └─ DashMap: aircraft_states.get_mut() → update geofence status
           │   └─ Unlock per-device mutex
           │
           ├─ Spawn: background flight completion (reverse geocoding, etc.)
           ├─ Spawn: DB: UPDATE receivers SET latest_packet_at
           ├─ DB: SELECT flight → check callsign, UPDATE if NULL
           └─ NATS: publish fix to soar.aircraft.{id}
```

### Database writes per APRS aircraft fix

1. **Receiver upsert** (GenericProcessor) — `INSERT INTO receivers ON CONFLICT DO UPDATE`
2. **Raw message insert** (GenericProcessor) — `INSERT INTO raw_messages`
3. **Aircraft upsert** (FixProcessor) — `SELECT` + conditional `INSERT INTO aircraft` + optional DDB lookup
4. **Fix insert** (FlightTracker, under lock) — `INSERT INTO fixes`
5. **Flight create** (FlightTracker, under lock, on takeoff) — `INSERT INTO flights`
6. **Flight landing update** (FlightTracker, under lock, on landing) — `UPDATE flights SET landing_time`
7. **Flight last_fix_at** (spawned async) — `UPDATE flights SET last_fix_at`
8. **Receiver latest_packet_at** (spawned async) — `UPDATE receivers SET latest_packet_at`
9. **Flight callsign** (after lock released) — `SELECT flight` + conditional `UPDATE flights SET callsign`

---

## Beast (ADS-B binary)

### Queues and workers

| Stage | Queue | Capacity | Workers | File |
|-------|-------|----------|---------|------|
| 1. Envelope intake | `envelope_rx` | 200 | 1 (envelope router) | `commands/run/mod.rs:388` |
| 2. Beast intake | `beast_intake` | 200 | **200** | `commands/run/workers.rs:59` |

No PacketRouter. No GenericProcessor. No aircraft/receiver/status sub-queues. Beast workers go directly to the database.

### Call chain

```
Envelope Router
  └─ beast_intake_tx.send_async()          # blocks if Beast intake full

Beast Intake Worker (200 workers)
  └─ process_beast_message()               # commands/run/beast.rs
     ├─ Validate frame length (min 11 bytes)
     ├─ rs1090: decode Mode S frame
     ├─ Extract ICAO address (24-bit hex)
     ├─ DB: AircraftRepository.get_or_insert_aircraft_by_address(icao, AddressType::Icao)
     │   └─ No DDB lookup for ADS-B
     ├─ DB: RawMessagesRepository.insert_beast()
     │   └─ INSERT INTO raw_messages (type='beast')
     │
     ├─ AdsbAccumulator.process_adsb_message()
     │   ├─ DashMap: states.entry(icao_address) → accumulate position/velocity/callsign
     │   ├─ CPR decode for lat/lon (requires even+odd frames)
     │   └─ try_emit_fix() → returns PartialFix if enough data accumulated
     │
     ├─ Build Fix from PartialFix + Aircraft
     │   └─ receiver_id = None (ADS-B has no receiver concept)
     │
     └─ FixProcessor.process_fix()         # fix_processor.rs:148
        └─ process_fix_internal()          # SAME shared path as APRS (see above)
           ├─ Elevation calculation
           ├─ FlightTracker.process_and_insert_fix() → INSERT fix, flight state machine
           ├─ No receiver update (receiver_id is None)
           ├─ Flight callsign update
           └─ NATS publish
```

### Database writes per Beast fix

1. **Aircraft upsert** — `SELECT` + conditional `INSERT INTO aircraft`
2. **Raw message insert** — `INSERT INTO raw_messages`
3. **Fix insert** (FlightTracker, under lock) — `INSERT INTO fixes`
4. **Flight create/update** (FlightTracker, under lock) — same state machine as APRS
5. **Flight last_fix_at** (spawned async) — `UPDATE flights SET last_fix_at`
6. **Flight callsign** (after lock released) — conditional `UPDATE flights SET callsign`

No receiver writes.

---

## SBS (ADS-B CSV / BaseStation)

### Queues and workers

| Stage | Queue | Capacity | Workers | File |
|-------|-------|----------|---------|------|
| 1. Envelope intake | `envelope_rx` | 200 | 1 (envelope router) | `commands/run/mod.rs:388` |
| 2. SBS intake | `sbs_intake` | 200 | **50** | `commands/run/workers.rs:114` |

Same as Beast — no PacketRouter, no GenericProcessor, direct to database.

### Call chain

```
Envelope Router
  └─ sbs_intake_tx.send_async()            # blocks if SBS intake full

SBS Intake Worker (50 workers)
  └─ process_sbs_message()                 # commands/run/sbs.rs
     ├─ Decode UTF-8 from bytes
     ├─ Parse CSV: soar::sbs::parse_sbs_message()
     │   └─ MSG types 1-8 (identification, position, velocity, squawk, etc.)
     ├─ Extract ICAO address
     ├─ DB: RawMessagesRepository.insert_sbs()
     │   └─ INSERT INTO raw_messages (type='sbs')
     │
     ├─ AdsbAccumulator.process_sbs_message()
     │   ├─ DashMap: states.entry(icao_address) → accumulate fields
     │   └─ try_emit_fix() → returns PartialFix if enough data accumulated
     │
     ├─ DB: AircraftRepository.get_or_insert_aircraft_by_address(icao, AddressType::Icao)
     ├─ Build Fix from PartialFix + Aircraft
     │   └─ receiver_id = None
     │
     └─ FixProcessor.process_fix()         # SAME shared path as APRS/Beast
        └─ process_fix_internal()
```

### Database writes per SBS fix

Same as Beast (no receiver writes).

---

## Comparison

| | APRS | Beast | SBS |
|---|---|---|---|
| Intake workers | **1** | 200 | 50 |
| Queues traversed | **4** (envelope → OGN → router → aircraft) | 2 (envelope → beast) | 2 (envelope → SBS) |
| Uses PacketRouter | Yes (50 workers, 1,000 capacity queue) | No | No |
| Uses GenericProcessor | Yes (receiver + raw_message) | No | No |
| Raw message insert | GenericProcessor | Beast worker directly | SBS worker directly |
| Aircraft lookup | FixProcessor (with DDB) | Beast worker directly (no DDB) | SBS worker directly (no DDB) |
| Accumulator | No | Yes (AdsbAccumulator, DashMap) | Yes (shared AdsbAccumulator) |
| Receiver tracking | Yes | No | No |
| DB writes per fix | up to 9 | up to 6 | up to 6 |

## In-memory state (DashMap)

| Map | Type | Used by |
|-----|------|---------|
| `aircraft_states` | `DashMap<Uuid, AircraftState>` | FlightTracker — all 3 protocols via `process_and_insert_fix` |
| `device_locks` | `DashMap<Uuid, Arc<Mutex<()>>>` | FlightTracker — per-aircraft serialization |
| `AdsbAccumulator.states` | `DashMap<u32, AccumulatedAircraftState>` | Beast + SBS workers — accumulate split ADS-B messages |
| `AircraftCache.by_address` | `DashMap<(AddressType, i32), Aircraft>` | FixProcessor + Beast/SBS workers — aircraft lookup |
| `AircraftCache.by_id` | `DashMap<Uuid, Aircraft>` | FlightTracker state transitions |

`aircraft_states.retain()` and `aircraft_states.iter()` are called periodically by `check_and_timeout_stale_flights()` and `cleanup_stale_aircraft_states()`. These acquire shard-level write/read locks across the entire map, blocking any concurrent `get()`/`get_mut()` on the same shard. Since DashMap uses parking_lot internally, this blocks the OS thread (and therefore the tokio worker thread).

## Known architectural problems

1. **Single OGN intake worker** — one blocked send stops all APRS processing.
2. **Envelope router is a single sequential task** — if any downstream intake queue is full, all protocols are blocked (head-of-line blocking).
3. **APRS traverses 4 queues** while Beast/SBS traverse 2 — the extra queues (PacketRouter, aircraft position) exist only for APRS but add latency and failure modes.
4. **DashMap shard locks block tokio threads** — `retain()` and `iter()` on `aircraft_states` hold synchronous write/read locks that can stall async workers sharing the same shard.
5. **Per-device tokio::Mutex held during DB writes and geofence checks** — serializes same-aircraft fixes but holds the lock for the entire `process_and_insert_fix` call including multiple DB round-trips.
