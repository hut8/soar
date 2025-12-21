# SOAR - Soaring Observation And Records

SOAR is an application under active development that will automate many duty-manager functions for glider clubs, as well as provide a glider tracker.

## Features

- **APRS-IS Connection**: Connect to any APRS-IS server with authentication
- **Message Processing**: Flexible message processing through trait implementation
- **Message Archiving**: Optional archiving of all incoming APRS messages to daily log files
- **UTC Date-based Logging**: Creates new log files daily based on UTC dates (YYYY-MM-DD.log)
- **Automatic Directory Creation**: Creates archive directories automatically
- **Midnight Rollover**: Automatically switches to new log files at UTC midnight
- **Configurable Filters**: Support for APRS-IS filters to limit received messages
- **Retry Logic**: Built-in connection retry with configurable parameters
- **Airspace Boundaries**: Import and display airspace data from OpenAIP on the operations map
  - Color-coded by classification (controlled, uncontrolled, special use)
  - Interactive polygons with detailed information
  - Incremental sync support for efficient updates
  - See [OpenAIP Setup Guide](docs/OPENAIP_SETUP.md) for configuration

## Data Processing Flow

The following diagram shows the complete data flow through SOAR, including all processing steps and queue sizes:

```mermaid
flowchart TB
    %% External Systems
    APRS[OGN APRS-IS Network<br/>~500 msg/sec]
    WebClients[Web Clients<br/>WebSocket]

    %% Ingestion Process (soar ingest-ogn)
    subgraph Ingest ["APRS Ingestion (soar ingest-ogn)"]
        AprsClient[APRS Client]
        RawQueue["Raw Message Queue<br/>(1,000 messages)"]
        AprsNatsPublisher[NATS Publisher]
    end

    %% NATS Pub/Sub (Lightweight Message Bus)
    NatsPubSub["NATS Pub/Sub<br/>Subject: ogn.raw<br/>(Fire-and-forget)"]

    %% Processing Process (soar run)
    subgraph Processing ["Message Processing (soar run)"]
        NatsSubscriber[NATS Subscriber]

        subgraph RouterBox ["Packet Router"]
            Router[Router Logic]
            GenericProc[Generic Processor<br/>Archiving & Recording]
        end

        %% Processing Queues
        subgraph Queues ["Processing Queues"]
            AircraftQueue["Aircraft Queue<br/>(1,000 messages)"]
            RecvStatusQueue["Receiver Status Queue<br/>(50 messages)"]
            RecvPosQueue["Receiver Position Queue<br/>(50 messages)"]
            ServerQueue["Server Status Queue<br/>(50 messages)"]
        end

        %% Processors
        subgraph Processors ["Worker Processors"]
            AircraftProc[Aircraft Position<br/>Processor]
            RecvStatusProc[Receiver Status<br/>Processor]
            RecvPosProc[Receiver Position<br/>Processor]
            ServerProc[Server Status<br/>Processor]
        end

        %% Flight Processing
        FixProc[Fix Processor]
        FlightTracker[Flight Tracker]
    end

    %% Real-time Broadcasting (within Fix Processor)
    subgraph Broadcast ["Real-time Broadcasting to Web Clients"]
        LiveFixQueue["Live Fix Queue<br/>(1,000 messages)"]
        LiveFixPublisher[NATS Fix Publisher<br/>(in FixProcessor)]
        LiveNats["NATS Pub/Sub<br/>Subjects: aircraft.fix.*<br/>aircraft.area.*"]
    end

    %% Batch Processes
    subgraph Batch ["Batch Processes"]
        Sitemap[Sitemap Generator<br/>soar sitemap]
    end

    %% Storage
    Database[(PostgreSQL + PostGIS<br/>Database)]
    ArchiveFiles[(Daily Archive Files<br/>.log.zst)]

    %% Data Flow - Ingestion
    APRS --> AprsClient
    AprsClient --> RawQueue
    RawQueue --> AprsNatsPublisher
    AprsNatsPublisher --> NatsPubSub

    %% Data Flow - Processing Entry
    NatsPubSub --> NatsSubscriber
    NatsSubscriber --> Router

    %% Data Flow - Router & Generic Processing
    Router --> GenericProc
    GenericProc -->|Archives all<br/>messages| ArchiveFiles
    GenericProc -->|Records in<br/>aprs_messages| Database

    %% Data Flow - Type-specific Routing
    Router -->|Aircraft Position| AircraftQueue
    Router -->|Receiver Status| RecvStatusQueue
    Router -->|Receiver Position| RecvPosQueue
    Router -->|Server Messages| ServerQueue

    %% Data Flow - Worker Processing
    AircraftQueue --> AircraftProc
    RecvStatusQueue --> RecvStatusProc
    RecvPosQueue --> RecvPosProc
    ServerQueue --> ServerProc

    %% Data Flow - Aircraft Processing Chain
    AircraftProc --> FixProc
    FixProc --> FlightTracker
    FlightTracker --> Database

    %% Data Flow - Other Processors to Database
    RecvStatusProc --> Database
    RecvPosProc --> Database
    ServerProc --> Database

    %% Data Flow - Real-time Broadcasting
    FixProc --> LiveFixQueue
    LiveFixQueue --> LiveFixPublisher
    LiveFixPublisher --> LiveNats
    LiveNats --> WebClients

    %% Data Flow - Batch Processes
    Database --> Sitemap
    Sitemap -->|Generates| SitemapFiles[(sitemap.xml)]

    %% Styling
    classDef queueStyle fill:#e1f5ff,stroke:#01579b,stroke-width:2px
    classDef procStyle fill:#fff9c4,stroke:#f57f17,stroke-width:2px
    classDef storageStyle fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
    classDef externalStyle fill:#c8e6c9,stroke:#1b5e20,stroke-width:2px

    class RawQueue,AircraftQueue,RecvStatusQueue,RecvPosQueue,ServerQueue,LiveFixQueue queueStyle
    class AprsClient,AprsNatsPublisher,NatsSubscriber,Router,GenericProc,AircraftProc,RecvStatusProc,RecvPosProc,ServerProc,FixProc,FlightTracker,LiveFixPublisher,Sitemap procStyle
    class Database,NatsPubSub,LiveNats,ArchiveFiles,SitemapFiles storageStyle
    class APRS,WebClients externalStyle
```

### Key Components

**Ingestion (`soar ingest-ogn`)**
- Connects to OGN APRS-IS network (~500 messages/sec)
- Buffers messages in 1,000-message queue
- Publishes to NATS pub/sub (subject: `ogn.raw`) using fire-and-forget for maximum throughput

**Processing (`soar run`)**
- **NATS Subscriber**: Consumes messages from NATS pub/sub with automatic reconnection
- **Packet Router**: Routes messages based on type
- **Generic Processor**: Runs inline for every message
  - Archives all raw messages to compressed daily log files (.log.zst)
  - Inserts message records into `aprs_messages` table
  - Identifies and caches receiver information
- **Type-specific Queues**: Buffers for specialized processing (aircraft: 1,000 messages; receiver/server: 50 messages each)
- **Worker Processors**: Process aircraft positions, receiver status/position, and server messages
- **Flight Tracking**: Fix Processor and Flight Tracker analyze aircraft movement patterns

**Storage**
- **PostgreSQL + PostGIS**: All processed data (devices, fixes, flights, receivers, airports)
- **NATS Pub/Sub**: Lightweight message bus for decoupling ingestion from processing (subjects: `ogn.raw`, `beast.raw`)
- **Daily Archive Files**: Compressed raw APRS messages (.log.zst) with UTC-based rotation

**Real-time Broadcasting**
- Fix Processor contains embedded NATS Fix Publisher with 1,000-message buffer
- Publishes aircraft positions to NATS (subjects: `aircraft.fix.{device_id}`, `aircraft.area.{lat}.{lon}`)
- Web server's LiveFixService bridges NATS messages to WebSocket clients

**Batch Processes**
- **Sitemap Generator**: Runs via `soar sitemap` command
- Generates sitemap.xml from database for SEO
- Includes devices, clubs, airports, receivers, and static pages

## Provisioning

### Database

Needs PostgreSQL with PostGIS and pg_trgm. Tested on PostgreSQL 17 but should work on any modern version as long as those extensions are available. Use the environment variable `DATABASE_URL` in the form `postgres://user:password@server/database` as a connection string.

### NATS

NATS provides a lightweight message bus that decouples data ingestion from processing. The OGN and ADS-B ingesters publish messages to NATS pub/sub, which the processing service (`soar run`) subscribes to. This architecture uses fire-and-forget semantics for maximum throughput.

To install, head over to [the NATS releases on GitHub](https://github.com/nats-io/nats-server/releases/) and download the latest AMD64 .deb package and install it via `dpkg -i`. For example:

```bash
# Install nats-server to /usr/local/bin/
 wget https://github.com/nats-io/nats-server/releases/download/v2.11.8/nats-server-v2.11.8-amd64.deb && sudo dpkg -i nats-server-*.deb && rm nats-server-*.deb
# Install into systemd
sudo cp infrastructure/nats-server.service /etc/systemd/system/nats-server.service
sudo systemctl daemon-reload
sudo systemctl start nats-server
```

### Reverse proxy

Some reverse proxy should be put in front of the web service to terminate SSL and provide other benefits, like load balancing (if necessary). By default, the production web server will listen on localhost at port 61225. If using Caddy, you will simply need a block at `/etc/caddy/Caddyfile` that looks something like this:

```
glider.flights {
        reverse_proxy localhost:61225
        log {
                output file /var/log/caddy/glider.flights.log
        }
}
```

## Deployment

Deployment is accomplished via the `deploy` script in the root of the project. The first time this is run, a file is created with appropriate permissions at `/etc/soar/env` which contains necessary environment variables. Edit these as needed.

The following systemd services are available in `infrastructure/systemd/`:
- `soar-ingest-ogn.service` - Ingests OGN/APRS messages from APRS-IS into NATS (`soar ingest-ogn`)
- `soar-ingest-adsb.service` - Ingests ADS-B Beast format messages into NATS (`soar ingest-adsb`)
- `soar-run.service` - Main processing service that consumes from NATS and processes messages (`soar run`)
- `soar-web.service` - Web server for the frontend application (`soar web`)
- Additional services for staging environments and batch jobs (sitemap, backups, etc.)

## ADS-B Beast Feed Setup

SOAR can ingest ADS-B data in Beast binary format from sources like dump1090, readsb, or aggregators like APRS.LOL. If your Beast feed is restricted by IP address (e.g., only accessible from your home network), you'll need to set up a relay.

### Setting Up a Beast Relay with socat

Use `socat` to relay the Beast feed from a restricted source to a port that SOAR can connect to:

```bash
# Install socat (if not already installed)
sudo apt install socat  # Debian/Ubuntu
# OR
brew install socat      # macOS

# Relay Beast data from out.adsb.lol to local port 30005
# This connects to the remote Beast feed and makes it available locally
socat -d -d TCP-LISTEN:30005,fork,reuseaddr TCP:out.adsb.lol:30005
```

**Command breakdown:**
- `TCP-LISTEN:30005` - Listen on local port 30005 (standard Beast port)
- `fork` - Handle multiple concurrent connections
- `reuseaddr` - Allow restarting without "address already in use" errors
- `TCP:out.adsb.lol:30005` - Connect to the remote Beast feed
- `-d -d` - Debug output (shows connections and data flow)

### Running as a systemd Service

For production use, run the relay as a systemd service:

```bash
# Create service file
sudo tee /etc/systemd/system/beast-relay.service << 'EOF'
[Unit]
Description=ADS-B Beast Relay (APRS.LOL to local port 30005)
After=network.target

[Service]
Type=simple
User=soar
Restart=always
RestartSec=10
ExecStart=/usr/bin/socat -d -d TCP-LISTEN:30005,fork,reuseaddr TCP:out.adsb.lol:30005

# Logging
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Enable and start the service
sudo systemctl daemon-reload
sudo systemctl enable beast-relay.service
sudo systemctl start beast-relay.service

# Check status
sudo systemctl status beast-relay.service

# View logs
sudo journalctl -u beast-relay.service -f
```

### Connecting SOAR to the Beast Feed

Once the relay is running, configure SOAR to connect to it:

```bash
# In your SOAR environment configuration (/etc/soar/env or .env)
BEAST_HOST=localhost
BEAST_PORT=30005

# Or connect to a remote relay
BEAST_HOST=your-relay-server.example.com
BEAST_PORT=30005
```

Then start the Beast ingestion service:
```bash
soar ingest-adsb --server localhost --port 30005
```

### Notes

- **IP Restrictions**: The relay must run on a host that has access to the restricted Beast feed (e.g., your home IP for APRS.LOL feeders)
- **Port 30005**: This is the standard Beast binary protocol port used by dump1090/readsb
- **Firewall**: Ensure your firewall allows incoming connections on port 30005 if SOAR is on a different host
- **Performance**: socat is very lightweight and can handle the ~500-1000 msg/sec typical Beast feed with minimal overhead
- **Security**: Consider using SSH tunneling or VPN instead of exposing the port publicly

### Alternative: SSH Tunnel

If the relay host and SOAR host are on different networks, use SSH port forwarding:

```bash
# On the SOAR host, forward local port 30005 to the relay host
ssh -L 30005:localhost:30005 user@relay-host.example.com -N

# Then connect SOAR to localhost:30005
soar ingest-adsb --server localhost --port 30005
```

## Development

### Quick Start for New Developers

1. **Install prerequisites**:
   ```bash
   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install Node.js 20+ from https://nodejs.org/
   # Install PostgreSQL with PostGIS extension
   ```

2. **Clone and set up the project**:
   ```bash
   git clone <repository-url>
   cd soar

   # Install all development tools (Diesel CLI, cargo-audit, etc.)
   ./scripts/install-dev-tools.sh

   # Set up pre-commit hooks (matches CI pipeline exactly)
   ./scripts/setup-precommit.sh
   ```

3. **Configure environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your database credentials and other settings

   # Set up test database
   createdb soar_test
   psql -d soar_test -c "CREATE EXTENSION IF NOT EXISTS postgis;"

   # Run migrations
   export DATABASE_URL="postgres://user:password@localhost:5432/soar_test"
   diesel migration run
   ```

4. **Verify setup**:
   ```bash
   # Test Rust build
   cargo build

   # Test web build
   cd web && npm run build && cd ..

   # Run pre-commit on all files to ensure everything works
   pre-commit run --all-files
   ```

### Development Tools

This project uses **pre-commit hooks** that run the same checks as our CI pipeline:

- **Rust**: `cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo audit`
- **Web**: `npm run lint`, `npm run check`, `npm test`, `npm run build`
- **General**: trailing whitespace, file endings, YAML/JSON validation

The hooks run automatically on every commit. To run manually:
```bash
pre-commit run --all-files
```

### Flight Detection Testing

SOAR includes a comprehensive testing framework for debugging and validating flight detection logic using real APRS message sequences from the database.

**Quick start:**
```bash
# Extract messages from a problematic flight and generate test case
scripts/dump-flight-messages production <flight-id>

# Enter description when prompted (e.g., "timeout resurrection creates new flight")
# Script generates test data file and test case automatically
```

For complete documentation, see [Flight Detection Testing Guide](docs/FLIGHT-DETECTION-TESTING.md).

### Database Migrations

Install Diesel CLI (done automatically by `install-dev-tools.sh`):
```bash
cargo install diesel_cli --no-default-features --features postgres
```

## Data

- **Airspace data** - Imported via OpenAIP API (requires free API key)
  - Use `soar pull-airspaces` command for global or country-specific imports
  - Supports incremental sync with `--incremental` flag
  - Data licensed under CC BY-NC 4.0 (non-commercial use only)
  - See [OpenAIP Setup Guide](docs/OPENAIP_SETUP.md) for detailed instructions
- **Airport data** - https://www.openaip.net/data/exports?page=1&limit=50&sortBy=createdAt&sortDesc=true&contentType=airport&format=ndgeojson&country=US or directly from the bucket at https://console.cloud.google.com/storage/browser/29f98e10-a489-4c82-ae5e-489dbcd4912f;tab=objects?pli=1&prefix=&forceOnObjectsSortingFiltering=false
- **Alternative airport data**: https://geodata.bts.gov/datasets/usdot::aviation-facilities/about (US only)
- **FAA Data** (aircraft registrations and models) - https://www.faa.gov/licenses_certificates/aircraft_certification/aircraft_registry/releasable_aircraft_download
- **Ourairports** (open, worldwide data) - https://ourairports.com/data/
- **FAA NASR**: This seems like the best for the USA - https://www.faa.gov/air_traffic/flight_info/aeronav/aero_data/NASR_Subscription/2025-08-07/

## License

This project is licensed under the MIT License.

## Data Source Notes

FANET

- FNT11: [Seems to mean Manufacturer is FANET+](https://github.com/glidernet/ogn-aprs-protocol/blob/af7a1688d28f9c41fddf60c1105d92dc53adb4c1/FANET.protocol.txt#L248)
- sF1: syncword: 0xF1
- cr4: Seems to mean "Coding Rate 4"

```
In LoRa (which FANET usually uses):

The coding rate is expressed as CR = 4/(4+N), where N = 1…4.

So:

CR1 → 4/5

CR2 → 4/6

CR3 → 4/7

CR4 → 4/8

That means “CR4” = 4/8 = 1/2, so:

Half the transmitted bits are data, half are error-correction redundancy.

This is the most robust (but slowest) option in LoRa.

So if FANET says "CR4", they almost certainly mean LoRa coding rate 4/8.
```

## Related projects

- https://github.com/tobiz/OGN-Flight-Logger_V2 - this might have good takeoff/landing detection
