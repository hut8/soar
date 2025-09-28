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

## APRS Packet Routing

* In `aprs_client::AprsClient`, an APRS sentence is received and parsed into an `AprsPacket` via `ogn_parser` (my fork of that crate)
* The packet is then dispatched via a PacketRouter:
  * APRS Status -> routed to ReceiverStatusProcessor
  * APRS Position -> determine the position source type (defined in `ogn_parser::PositionSourceType`)
    * Aircraft -> routed to AircraftPositionProcessor, which will:
      * parse the AprsPacket
      * create a Fix, if the device is present in the DDB
      * use the FlightDetectionProcessor to associate the fix with a flight, if applicable
      * insert it into the database
      * send it to NATS so that web clients will see it if they are subscribed
    * Receiver -> log via `trace!()` then ignore
    * Weather Station -> log via `trace!()` then ignore
  * APRS Server Comment -> routed to ServerStatusProcessor, which will:
    * Parse out useful fields (timestamp, from which lag is computed; server name; endpoint; software version)
    * Insert these into the database

## Provisioning

### Database

Needs PostgreSQL with PostGIS and pg_trgm. Tested on PostgreSQL 17 but should work on any modern version as long as those extensions are available. Use the environment variable `DATABASE_URL` in the form `postgres://user:password@server/database` as a connection string.

### NATS

NATS allows the OGN data ingester to relay updates to interested subscribers in the web server process. To install, head over to [the NATS releases on GitHub](https://github.com/nats-io/nats-server/releases/) and download the latest AMD64 .deb package and install it via `dpkg -i`. For example:

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

Deployment is accomplished via the `deploy` script in the root of the project. The first time this is run, a file is created with appropriate permissions at `/etc/soar/env` which contains necessary environment variables. Edit these as needed. Two systemd files will be installed: `soar-run.service` and `soar-web.service`. Their content is self-explanatory.

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

### Database Migrations

Install Diesel CLI (done automatically by `install-dev-tools.sh`):
```bash
cargo install diesel_cli --no-default-features --features postgres
```

## Data

- Airport data should be pulled from https://www.openaip.net/data/exports?page=1&limit=50&sortBy=createdAt&sortDesc=true&contentType=airport&format=ndgeojson&country=US or directly from the bucket at https://console.cloud.google.com/storage/browser/29f98e10-a489-4c82-ae5e-489dbcd4912f;tab=objects?pli=1&prefix=&forceOnObjectsSortingFiltering=false
- Alternative airport data: https://geodata.bts.gov/datasets/usdot::aviation-facilities/about (US only)
- FAA Data (aircraft registrations and models) - https://www.faa.gov/licenses_certificates/aircraft_certification/aircraft_registry/releasable_aircraft_download
- Ourairports (open, worldwide data) - https://ourairports.com/data/
- FAA NASR: This seems like the best for the USA - https://www.faa.gov/air_traffic/flight_info/aeronav/aero_data/NASR_Subscription/2025-08-07/

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
