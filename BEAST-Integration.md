# ADS-B Beast Integration Implementation Plan

## Overview

This document outlines the plan to integrate ADS-B Beast protocol data into SOAR alongside the existing APRS data stream. The key challenge is staying under PostgreSQL's 32-column limit for the `fixes` table while adding ADS-B-specific metadata.

**Solution**: Consolidate protocol-specific columns into a JSONB `source_metadata` column, freeing up 4 columns for future expansion.

## Current State Analysis

### Fix Table Column Count
- **Current columns**: 30/32 used
- **Headroom**: Only 2 columns available
- **Constraint**: PostgreSQL limit of 32 columns per table

### APRS-Specific Columns (to be consolidated)
1. `snr_db` - Signal-to-noise ratio (APRS receiver)
2. `bit_errors_corrected` - APRS receiver error correction
3. `freq_offset_khz` - Frequency offset at receiver
4. `gnss_horizontal_resolution` - GPS accuracy (APRS format)
5. `gnss_vertical_resolution` - GPS accuracy (APRS format)

### Universal Columns (used by both APRS and ADS-B)
- Position: `latitude`, `longitude`, `altitude_msl_feet`, `altitude_agl_feet`
- Movement: `ground_speed_knots`, `track_degrees`, `climb_fpm`
- Flight: `flight_number`, `squawk`, `flight_id`
- System: `device_id`, `receiver_id`, `received_at`, `timestamp`

## Phase 1: Database Schema Migration (JSONB Consolidation)

### Step 1.1: Add `source_metadata` JSONB Column

**Migration**: `migrations/YYYYMMDDHHMMSS_add_source_metadata_jsonb/up.sql`

```sql
-- Add JSONB column for protocol-specific metadata
ALTER TABLE fixes ADD COLUMN source_metadata JSONB;

-- Create GIN index for fast JSONB queries
CREATE INDEX idx_fixes_source_metadata ON fixes USING GIN (source_metadata);

-- Add partial index for protocol type (fast filtering)
CREATE INDEX idx_fixes_protocol ON fixes ((source_metadata->>'protocol'));
```

**JSONB Structure**:
```json
// APRS fixes:
{
  "protocol": "aprs",
  "snr_db": 12.5,
  "bit_errors_corrected": 3,
  "freq_offset_khz": 0.2,
  "gnss_horizontal_resolution": 15,
  "gnss_vertical_resolution": 20
}

// ADS-B fixes:
{
  "protocol": "adsb",
  "nic": 8,                    // Navigation Integrity Category (0-11)
  "nac_p": 9,                  // Navigation Accuracy Category - Position
  "nac_v": 2,                  // Navigation Accuracy Category - Velocity
  "sil": 3,                    // Surveillance Integrity Level (0-3)
  "emergency_status": "none",  // Emergency state
  "on_ground": false,          // Surface vs airborne
  "selected_altitude_ft": 35000,
  "selected_heading_deg": 270,
  "autopilot_engaged": true
}
```

### Step 1.2: Backfill Existing APRS Data

**Migration**: `migrations/YYYYMMDDHHMMSS_backfill_aprs_metadata/up.sql`

```sql
-- Backfill existing APRS data into source_metadata JSONB
-- Note: This must handle partitioned table (update per partition)

DO $$
DECLARE
    partition_name TEXT;
BEGIN
    FOR partition_name IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
        AND tablename LIKE 'fixes_%'
    LOOP
        EXECUTE format('
            UPDATE %I
            SET source_metadata = jsonb_build_object(
                ''protocol'', ''aprs'',
                ''snr_db'', snr_db,
                ''bit_errors_corrected'', bit_errors_corrected,
                ''freq_offset_khz'', freq_offset_khz,
                ''gnss_horizontal_resolution'', gnss_horizontal_resolution,
                ''gnss_vertical_resolution'', gnss_vertical_resolution
            )
            WHERE source_metadata IS NULL
        ', partition_name);
    END LOOP;
END $$;
```

### Step 1.3: Drop Consolidated Columns

**Migration**: `migrations/YYYYMMDDHHMMSS_drop_aprs_columns/up.sql`

```sql
-- Drop APRS-specific columns (now in JSONB)
ALTER TABLE fixes DROP COLUMN snr_db;
ALTER TABLE fixes DROP COLUMN bit_errors_corrected;
ALTER TABLE fixes DROP COLUMN freq_offset_khz;
ALTER TABLE fixes DROP COLUMN gnss_horizontal_resolution;
ALTER TABLE fixes DROP COLUMN gnss_vertical_resolution;

-- Net result: -5 columns, +1 column = 4 columns freed (26/32 used)
```

### Step 1.4: Rename and Generalize Message Reference

**Migration**: `migrations/YYYYMMDDHHMMSS_generalize_source_message/up.sql`

```sql
-- Rename aprs_message_id to source_message_id
ALTER TABLE fixes RENAME COLUMN aprs_message_id TO source_message_id;

-- Make nullable (ADS-B may have different message storage strategy)
ALTER TABLE fixes ALTER COLUMN source_message_id DROP NOT NULL;

-- Update foreign key constraint name
ALTER TABLE fixes RENAME CONSTRAINT fixes_aprs_message_id_fkey TO fixes_source_message_id_fkey;
```

## Phase 2: Beast Message Storage

### Step 2.1: Create `beast_messages` Table

**Migration**: `migrations/YYYYMMDDHHMMSS_create_beast_messages/up.sql`

```sql
-- Similar structure to aprs_messages table
CREATE TABLE beast_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    received_at TIMESTAMPTZ NOT NULL,
    raw_message TEXT NOT NULL,  -- Hex-encoded Beast frame
    message_type VARCHAR(50),   -- e.g., "DF17_AIRBORNE_POSITION"
    icao_address VARCHAR(6),    -- 24-bit ICAO in hex
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (received_at);

-- Create index on received_at for partition pruning
CREATE INDEX idx_beast_messages_received_at ON beast_messages (received_at);

-- Create index on ICAO for lookups
CREATE INDEX idx_beast_messages_icao ON beast_messages (icao_address);

-- Daily partitioning (same as fixes and aprs_messages)
-- Partitions created automatically by partition manager
```

### Step 2.2: Update Beast Client Message Format

**File**: `src/beast/client.rs`

Currently stores: `"2025-11-17T12:34:56.789Z <hex-frame>"`

Update to preserve more structure:
```rust
// New format: timestamp | message_type | icao | hex_frame
let message = format!(
    "{} {} {} {}",
    timestamp,
    message_type,  // From rs1090 decode
    icao_address,  // From rs1090 decode
    hex::encode(frame)
);
```

## Phase 3: ADS-B Decoding Infrastructure

### Step 3.1: Create CPR Decoder

**File**: `src/beast/decoder.rs`

```rust
use rs1090::{decode::cpr, Message};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// CPR decoder state machine
/// ADS-B positions use Compact Position Reporting (CPR) which requires
/// combining even/odd frames to get global position
pub struct CprDecoder {
    /// Pending positions waiting for even/odd pair
    /// Key: ICAO address, Value: (even_frame, odd_frame, last_update)
    pending: HashMap<String, (Option<CprFrame>, Option<CprFrame>, Instant)>,

    /// Timeout for stale positions (10 seconds)
    timeout: Duration,
}

#[derive(Clone)]
struct CprFrame {
    latitude_cpr: u32,
    longitude_cpr: u32,
    is_odd: bool,
    altitude: Option<i32>,
    timestamp: Instant,
}

impl CprDecoder {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            timeout: Duration::from_secs(10),
        }
    }

    /// Decode position from ADS-B message
    /// Returns Some((lat, lon)) if we have both even/odd frames
    pub fn decode_position(
        &mut self,
        icao: &str,
        msg: &rs1090::data::ADSBMessage,
    ) -> Option<(f64, f64, i32)> {
        // Extract CPR data from message
        let cpr_frame = self.extract_cpr(msg)?;

        // Get or create pending state
        let (even, odd, _) = self.pending
            .entry(icao.to_string())
            .or_insert((None, None, Instant::now()));

        // Store frame based on parity
        if cpr_frame.is_odd {
            *odd = Some(cpr_frame.clone());
        } else {
            *even = Some(cpr_frame.clone());
        }

        // If we have both, decode global position
        if let (Some(even_frame), Some(odd_frame)) = (even, odd) {
            match cpr::decode_position(
                even_frame.latitude_cpr,
                even_frame.longitude_cpr,
                odd_frame.latitude_cpr,
                odd_frame.longitude_cpr,
            ) {
                Ok((lat, lon)) => {
                    let altitude = cpr_frame.altitude.unwrap_or(0);
                    return Some((lat, lon, altitude));
                }
                Err(e) => {
                    warn!("CPR decode failed for {}: {}", icao, e);
                    return None;
                }
            }
        }

        None
    }

    /// Clean up stale positions
    pub fn cleanup_stale(&mut self) {
        let now = Instant::now();
        self.pending.retain(|_, (_, _, last_update)| {
            now.duration_since(*last_update) < self.timeout
        });
    }

    fn extract_cpr(&self, msg: &rs1090::data::ADSBMessage) -> Option<CprFrame> {
        // Extract CPR fields from rs1090 message
        // Implementation depends on rs1090 API
        todo!("Extract CPR from rs1090 message")
    }
}
```

### Step 3.2: Create ADS-B to Fix Converter

**File**: `src/beast/converter.rs`

```rust
use crate::fixes::Fix;
use rs1090::data::ADSBMessage;
use serde_json::json;
use uuid::Uuid;

pub struct AdsbToFixConverter {
    cpr_decoder: CprDecoder,
}

impl AdsbToFixConverter {
    pub fn new() -> Self {
        Self {
            cpr_decoder: CprDecoder::new(),
        }
    }

    /// Convert ADS-B message to Fix
    /// May return None if position cannot be decoded yet
    pub fn convert(
        &mut self,
        msg: &ADSBMessage,
        received_at: DateTime<Utc>,
        receiver_id: Uuid,
        beast_message_id: Uuid,
    ) -> Option<Fix> {
        let icao = format!("{:06X}", msg.icao_address);

        // Decode position using CPR
        let (latitude, longitude, altitude) =
            self.cpr_decoder.decode_position(&icao, msg)?;

        // Extract other fields
        let ground_speed = msg.ground_speed_knots();
        let track = msg.track_degrees();
        let vertical_rate = msg.vertical_rate_fpm();
        let callsign = msg.callsign();

        // Build source_metadata JSONB
        let source_metadata = json!({
            "protocol": "adsb",
            "nic": msg.navigation_integrity_category(),
            "nac_p": msg.navigation_accuracy_position(),
            "nac_v": msg.navigation_accuracy_velocity(),
            "sil": msg.surveillance_integrity_level(),
            "emergency_status": msg.emergency_status(),
            "on_ground": msg.is_on_ground(),
        });

        // Create Fix
        Some(Fix {
            id: Uuid::new_v4(),
            source: icao.clone(),
            aprs_type: "ADSB".to_string(),
            via: vec![],  // ADS-B has no digipeaters
            timestamp: received_at,
            latitude,
            longitude,
            altitude_msl_feet: Some(altitude),
            altitude_agl_feet: None,  // Calculated later
            flight_number: callsign,
            squawk: msg.squawk(),
            ground_speed_knots: ground_speed,
            track_degrees: track,
            climb_fpm: vertical_rate,
            turn_rate_rot: None,  // Not in ADS-B
            flight_id: None,  // Assigned by FlightTracker
            device_id: todo!("Lookup or create device from ICAO"),
            received_at,
            is_active: ground_speed.map(|gs| gs >= 25.0).unwrap_or(false),
            receiver_id,
            source_message_id: Some(beast_message_id),
            altitude_agl_valid: false,
            time_gap_seconds: None,
            source_metadata: Some(source_metadata),
        })
    }
}
```

### Step 3.3: Create Beast Message Processor

**File**: `src/beast/processor.rs`

```rust
use crate::beast::converter::AdsbToFixConverter;
use rs1090;

pub struct BeastProcessor {
    converter: AdsbToFixConverter,
}

impl BeastProcessor {
    pub fn new() -> Self {
        Self {
            converter: AdsbToFixConverter::new(),
        }
    }

    /// Process raw Beast message
    pub async fn process_message(
        &mut self,
        raw_message: &str,
        receiver_id: Uuid,
    ) -> Result<Option<Fix>> {
        // Parse: "timestamp message_type icao hex_frame"
        let parts: Vec<&str> = raw_message.split_whitespace().collect();
        if parts.len() != 4 {
            return Err(anyhow::anyhow!("Invalid Beast message format"));
        }

        let timestamp_str = parts[0];
        let _message_type = parts[1];
        let _icao = parts[2];
        let hex_frame = parts[3];

        // Parse timestamp
        let received_at = chrono::DateTime::parse_from_rfc3339(timestamp_str)?
            .with_timezone(&chrono::Utc);

        // Decode hex to bytes
        let bytes = hex::decode(hex_frame)?;

        // Decode using rs1090
        let msg = rs1090::decode(&bytes)?;

        // Store raw message in beast_messages table
        let beast_message_id = self.store_beast_message(
            received_at,
            raw_message,
            &msg,
        ).await?;

        // Convert to Fix
        let fix = self.converter.convert(
            &msg,
            received_at,
            receiver_id,
            beast_message_id,
        );

        Ok(fix)
    }

    async fn store_beast_message(
        &self,
        received_at: DateTime<Utc>,
        raw_message: &str,
        msg: &rs1090::data::ADSBMessage,
    ) -> Result<Uuid> {
        // Insert into beast_messages table
        // Return generated UUID
        todo!("Insert into beast_messages")
    }
}
```

## Phase 4: Run Command Integration

### Step 4.1: Add Beast JetStream Consumer

**File**: `src/commands/run.rs` (modifications)

```rust
// Add Beast stream constants
use soar::queue_config::{
    BEAST_RAW_STREAM, BEAST_RAW_STREAM_STAGING,
    BEAST_RAW_SUBJECT, BEAST_RAW_SUBJECT_STAGING,
    // ... existing APRS constants
};

// In handle_run():

// Determine Beast stream names based on environment
let (beast_stream_name, beast_subject) = if is_production {
    (BEAST_RAW_STREAM.to_string(), BEAST_RAW_SUBJECT.to_string())
} else {
    (BEAST_RAW_STREAM_STAGING.to_string(), BEAST_RAW_SUBJECT_STAGING.to_string())
};

// Create Beast intake queue
let (beast_intake_tx, beast_intake_rx) =
    flume::bounded::<String>(JETSTREAM_INTAKE_QUEUE_SIZE);

info!(
    "Created Beast intake queue with capacity {}",
    JETSTREAM_INTAKE_QUEUE_SIZE
);

// Spawn Beast JetStream consumer
let beast_consumer_handle = tokio::spawn({
    let nats_client = nats_client.clone();
    let beast_stream_name = beast_stream_name.clone();
    let beast_subject = beast_subject.clone();
    let beast_intake_tx = beast_intake_tx.clone();

    async move {
        consume_beast_jetstream(
            nats_client,
            &beast_stream_name,
            &beast_subject,
            beast_intake_tx,
        ).await
    }
});

// Spawn Beast message processor
let beast_processor_handle = tokio::spawn({
    let packet_router = packet_router.clone();

    async move {
        process_beast_intake(beast_intake_rx, packet_router).await
    }
});
```

### Step 4.2: Create Beast Message Processing Function

**File**: `src/commands/run.rs` (add new function)

```rust
/// Process Beast messages from the intake queue
async fn process_beast_intake(
    intake_rx: flume::Receiver<String>,
    packet_router: PacketRouter,
) {
    let mut beast_processor = BeastProcessor::new();

    while let Ok(message) = intake_rx.recv_async().await {
        metrics::counter!("beast.messages.received").increment(1);

        // Process Beast message
        match beast_processor.process_message(&message, receiver_id).await {
            Ok(Some(fix)) => {
                // Route Fix through existing PacketRouter
                packet_router.process_fix(fix).await;
                metrics::counter!("beast.fixes.created").increment(1);
            }
            Ok(None) => {
                // Position not ready yet (waiting for CPR pair)
                metrics::counter!("beast.position.pending").increment(1);
            }
            Err(e) => {
                warn!("Failed to process Beast message: {}", e);
                metrics::counter!("beast.process.failed").increment(1);
            }
        }
    }
}

/// Consume Beast messages from JetStream
async fn consume_beast_jetstream(
    nats_client: async_nats::Client,
    stream_name: &str,
    subject: &str,
    intake_tx: flume::Sender<String>,
) -> Result<()> {
    // Similar to APRS JetStream consumer
    // Pull from Beast stream, send to intake queue
    todo!("Implement Beast JetStream consumer")
}
```

### Step 4.3: Extend PacketRouter

**File**: `src/packet_processors/mod.rs`

```rust
impl PacketRouter {
    /// Process a Fix directly (for ADS-B)
    /// Bypasses APRS parsing since Fix is already created
    pub async fn process_fix(&self, fix: Fix) {
        // Send to aircraft queue for processing
        match self.aircraft_tx.send_async(fix).await {
            Ok(_) => {
                metrics::counter!("packet_router.fix.queued").increment(1);
            }
            Err(e) => {
                error!("Failed to queue Fix: {}", e);
                metrics::counter!("packet_router.fix.dropped").increment(1);
            }
        }
    }
}
```

## Phase 5: Data Model Updates

### Step 5.1: Update Fix Struct

**File**: `src/fixes.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::fixes)]
pub struct Fix {
    // ... existing fields ...

    /// Protocol-specific metadata stored as JSONB
    /// For APRS: snr_db, bit_errors_corrected, freq_offset_khz, gnss_*_resolution
    /// For ADS-B: nic, nac_p, nac_v, sil, emergency_status, etc.
    pub source_metadata: Option<serde_json::Value>,

    // Removed fields (now in source_metadata):
    // - snr_db
    // - bit_errors_corrected
    // - freq_offset_khz
    // - gnss_horizontal_resolution
    // - gnss_vertical_resolution
}

impl Fix {
    /// Check if this is an ADS-B fix
    pub fn is_adsb(&self) -> bool {
        self.source_metadata
            .as_ref()
            .and_then(|m| m.get("protocol"))
            .and_then(|p| p.as_str())
            .map(|p| p == "adsb")
            .unwrap_or(false)
    }

    /// Check if this is an APRS fix
    pub fn is_aprs(&self) -> bool {
        self.source_metadata
            .as_ref()
            .and_then(|m| m.get("protocol"))
            .and_then(|p| p.as_str())
            .map(|p| p == "aprs")
            .unwrap_or(true)  // Default to APRS for backward compatibility
    }

    /// Get ADS-B Navigation Integrity Category
    pub fn get_nic(&self) -> Option<i32> {
        self.source_metadata
            .as_ref()
            .and_then(|m| m.get("nic"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
    }

    /// Get emergency status (ADS-B only)
    pub fn get_emergency_status(&self) -> Option<String> {
        self.source_metadata
            .as_ref()
            .and_then(|m| m.get("emergency_status"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}
```

### Step 5.2: Update Schema

**File**: `src/schema.rs`

```rust
diesel::table! {
    fixes (id, received_at) {
        id -> Uuid,
        source -> Varchar,
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        flight_number -> Nullable<Varchar>,
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        device_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        source_message_id -> Nullable<Uuid>,  // Renamed from aprs_message_id
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,   // NEW FIELD
        // REMOVED: snr_db, bit_errors_corrected, freq_offset_khz,
        //          gnss_horizontal_resolution, gnss_vertical_resolution
    }
}
```

## Phase 6: Device Management

### Step 6.1: ICAO Address Handling

**File**: `src/device_repo.rs`

```rust
impl DeviceRepository {
    /// Get or create device from ICAO address (ADS-B)
    pub async fn get_or_create_from_icao(&self, icao: &str) -> Result<Uuid> {
        // Check if device exists with this ICAO
        if let Some(device) = self.find_by_icao(icao).await? {
            return Ok(device.id);
        }

        // Create new device
        let new_device = NewDevice {
            registration: Some(format!("ICAO-{}", icao)),
            icao_address: Some(icao.to_string()),
            // ... other fields
        };

        let device = self.insert(&new_device).await?;
        Ok(device.id)
    }

    /// Find device by ICAO address
    async fn find_by_icao(&self, icao: &str) -> Result<Option<Device>> {
        // Query devices table by icao_address field
        todo!("Implement ICAO lookup")
    }
}
```

**Note**: May need to add `icao_address` column to `devices` table if not already present.

## Phase 7: Testing Strategy

### Unit Tests

**File**: `src/beast/decoder_test.rs`
- Test CPR even/odd frame combination
- Test timeout of stale positions
- Test edge cases (equator, poles, antimeridian)

**File**: `src/beast/converter_test.rs`
- Test ADS-B to Fix conversion
- Test JSONB metadata population
- Test different message types

### Integration Tests

**File**: `tests/beast_integration_test.rs`
- End-to-end: Beast message → JetStream → Database
- Verify Fix created with correct fields
- Verify JSONB metadata structure

### Performance Tests

- Measure ADS-B throughput (target: 5 Hz per aircraft × 1000 aircraft = 5000 msg/s)
- Measure JSONB query performance
- Measure database write performance

## Phase 8: Deployment Strategy

### Stage 1: Database Migration (Low Risk)
1. Deploy migration to add `source_metadata` column
2. Create GIN indexes
3. Monitor performance (should be no impact - column is nullable)

### Stage 2: Backfill APRS Data (Medium Risk)
1. Run backfill migration during low-traffic period
2. Monitor for partition lock issues
3. May need to backfill per-partition over several days

### Stage 3: Deploy Updated Code (Medium Risk)
1. Deploy code with `source_metadata` support
2. Both old columns and JSONB work simultaneously
3. Monitor for any parsing errors

### Stage 4: Enable Beast Consumer (High Risk)
1. Start Beast ingestion in staging first
2. Verify CPR decoder works correctly
3. Monitor database write rate
4. Roll out to production

### Stage 5: Drop Old Columns (Low Risk)
1. After confirming JSONB works for several days
2. Drop the 5 consolidated columns
3. Frees 4 columns for future use

## Monitoring & Metrics

### New Metrics to Add

**Beast Ingestion**:
- `beast.messages.received` - Total Beast messages from JetStream
- `beast.frames.decoded` - Successfully decoded frames
- `beast.parse.failed` - Failed rs1090 decodes
- `beast.cpr.pending` - Positions waiting for CPR pair
- `beast.cpr.decoded` - Successfully decoded positions
- `beast.cpr.timeout` - CPR pairs that timed out
- `beast.fixes.created` - Fixes created from ADS-B

**Database**:
- `fixes.protocol.aprs` - Count of APRS fixes
- `fixes.protocol.adsb` - Count of ADS-B fixes
- `fixes.jsonb.query_time_ms` - JSONB query latency

### Dashboards

**ADS-B Ingest Dashboard**:
- Beast message rate
- CPR decode success rate
- Position decode latency
- Fix creation rate

**Protocol Comparison Dashboard**:
- APRS vs ADS-B coverage map
- Message rates by protocol
- Aircraft tracked by each protocol
- Overlap (aircraft seen in both)

## Key Considerations

### CPR Complexity
CPR decoding is the most complex part:
- Requires state management (pending even/odd frames)
- Timeout handling (stale frames)
- Edge cases (antimeridian crossing, poles)
- Reference position for local decoding

**Recommendation**: Use rs1090's built-in CPR decoder if available, otherwise implement carefully with extensive testing.

### Message Rate Differences
- **APRS**: ~0.2 Hz (one update every 5 seconds)
- **ADS-B**: 2-5 Hz (multiple updates per second)

This means ADS-B will generate 10-25x more database writes. Ensure:
- Database has sufficient write capacity
- Partitioning is working correctly
- Indexes are optimized

### Device Mapping
ADS-B uses ICAO addresses (e.g., "A12345"), APRS uses callsigns (e.g., "N12345").

Need to:
- Link ICAO to FAA registration (N-number)
- Handle same aircraft appearing in both APRS and ADS-B
- Prefer ADS-B when both available (higher update rate, more accurate)

### Receiver Differences
- **APRS**: Network of receivers with telemetry
- **ADS-B**: Direct RF receiver (no telemetry in Beast protocol)

For ADS-B:
- No receiver status messages
- No receiver position updates (unless configured separately)
- `receiver_id` must be configured or inferred

## Future Enhancements

### Phase 9: Advanced ADS-B Features
- Target state (autopilot altitude/heading)
- TCAS resolution advisories
- Mode S Comm-B messages
- Aircraft operational status

### Phase 10: Protocol Merging
- Merge APRS and ADS-B tracks for same aircraft
- Prefer ADS-B for position accuracy
- Use APRS for coverage in areas without ADS-B
- Conflict detection (same aircraft, different positions)

### Phase 11: Analytics
- ADS-B coverage maps
- Position accuracy comparison (APRS vs ADS-B)
- Emergency event tracking
- Autopilot state analysis

## References

- **rs1090 Documentation**: https://docs.rs/rs1090/
- **ADS-B Specification**: RTCA DO-260B / ICAO Annex 10
- **CPR Algorithm**: RTCA DO-242A Section 2.6
- **Beast Protocol**: https://wiki.modesbeast.com/Radarcape:Firmware_Versions
