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

## Usage

### Basic Configuration

```rust
use soar::{AprsClient, AprsClientConfigBuilder, MessageProcessor};
use std::sync::Arc;
use ogn_parser::AprsPacket;

// Implement your message processor
struct MyProcessor;

impl MessageProcessor for MyProcessor {
    fn process_message(&self, message: AprsPacket) {
        println!("Received: {:?}", message);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AprsClientConfigBuilder::new()
        .server("aprs.glidernet.org")
        .port(14580)
        .callsign("N0CALL")  // Use your actual callsign
        .password(None)      // Use None for read-only access
        .build();

    let processor = Arc::new(MyProcessor);
    let mut client = AprsClient::new(config, processor);

    client.start().await?;

    // Keep running...
    tokio::signal::ctrl_c().await?;
    client.stop().await;

    Ok(())
}
```

### With Message Archiving

To enable message archiving, simply add the `archive_base_dir` configuration:

```rust
let config = AprsClientConfigBuilder::new()
    .server("aprs.glidernet.org")
    .port(14580)
    .callsign("N0CALL")
    .password(None)
    .filter(Some("r/47.0/-122.0/100".to_string())) // Optional filter
    .archive_base_dir(Some("./aprs_logs".to_string())) // Enable archiving
    .build();
```

This will:
- Create the `./aprs_logs` directory if it doesn't exist
- Create daily log files named `YYYY-MM-DD.log` (e.g., `2025-08-29.log`)
- Log each incoming APRS message with a timestamp: `[HH:MM:SS] <APRS message>`
- Automatically roll over to a new file at UTC midnight

### Configuration Options

- `server`: APRS-IS server hostname (default: "aprs.glidernet.org")
- `port`: APRS-IS server port (default: 14580)
- `callsign`: Your amateur radio callsign (required)
- `password`: APRS-IS password (use `None` for read-only access)
- `filter`: APRS-IS filter string (optional)
- `max_retries`: Maximum connection retry attempts (default: 5)
- `retry_delay_seconds`: Delay between retry attempts (default: 5)
- `archive_base_dir`: Base directory for message archives (optional)

### Log File Format

When archiving is enabled, each message is logged with the following format:

```
[14:30:25] N0CALL>APRS,TCPIP*:>Hello World
[14:30:26] W1ABC-9>APRS,TCPIP*,qAC,T2SYDNEY:=3745.00N/12200.00W-Test Position
```

- `[HH:MM:SS]`: UTC timestamp when the message was received
- Followed by the raw APRS message as received from the server

## Development

This is necessary to run migrations the first time (sqlx requires a correct schema in order to build).

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## License

This project is licensed under the MIT License.
