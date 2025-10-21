# Flydent

A Rust port of the Python [flydenity](https://github.com/Meisterschueler/flydenity) library for parsing aircraft registration callsigns and ICAO 24-bit identifiers.

## Features

- **Parse aircraft callsigns** (e.g., "T6ABC" → Afghanistan)
- **Parse ICAO 24-bit identifiers** (e.g., "700123" → Afghanistan)
- **Identify countries** with ISO codes and descriptions
- **Identify organizations** (like ICAO, UN, etc.)
- **Zero runtime overhead** - CSV data is parsed at compile time using macros
- **No external files required** - all data is embedded in the binary
- **Command-line interface** compatible with the original Python version

## Installation

```bash
cargo build --release
```

## Usage

### As a Library

```rust
use flydent::{Parser, EntityResult};

let parser = Parser::new();

// Parse a callsign
if let Some(result) = parser.parse_simple("T6ABC") {
    match result {
        EntityResult::Country { nation, iso2, iso3, description } => {
            println!("Country: {} ({}/{}) - {}", nation, iso2, iso3, description);
        }
        EntityResult::Organization { name, description } => {
            println!("Organization: {} - {}", name, description);
        }
    }
}

// Parse ICAO 24-bit identifier
if let Some(result) = parser.parse("700123", false, true) {
    println!("ICAO result: {:?}", result);
}
```

### Command Line Interface

```bash
# Parse aircraft callsigns
./target/release/flydent T6ABC N123ABC 4Y123

# Parse ICAO 24-bit identifiers
./target/release/flydent --icao24bit 700123

# Show help
./target/release/flydent --help
```

## Examples

```bash
$ ./target/release/flydent T6ABC
{"T6ABC":{"description":"general","iso2":"AF","iso3":"AFG","nation":"Afghanistan"}}

$ ./target/release/flydent 4Y123
{"4Y123":{"description":"general","name":"International Civil Aviation Organization"}}

$ ./target/release/flydent --icao24bit 700123
{"700123":{"description":"general","iso2":"AF","iso3":"AFG","nation":"Afghanistan"}}
```

## Implementation Details

This Rust port uses compile-time macros to parse the CSV data files and generate efficient lookup structures:

- **Compile-time CSV parsing**: The CSV files are parsed at compile time using macros
- **Static lookup tables**: Hash maps are built once using `once_cell::sync::Lazy`
- **Regex compilation**: Regular expressions are compiled on-demand and cached
- **Zero-copy parsing**: String slices are used where possible to minimize allocations

The binary includes no external dependencies at runtime - all ITU data is embedded directly in the executable.

## Compatibility

This implementation maintains full compatibility with the original Python flydenity library:
- Same input/output format
- Same parsing logic and priority handling
- Same regex patterns and matching behavior
- Compatible JSON output format

## Performance

The Rust version offers significant performance improvements:
- **Faster startup**: No Python interpreter overhead
- **Lower memory usage**: Optimized data structures and zero-copy parsing
- **Faster parsing**: Native code execution and efficient hash lookups
- **Smaller binary**: Single executable with all data embedded

## Testing

```bash
cargo test
```

The test suite includes:
- CSV parsing validation
- Callsign parsing accuracy
- ICAO 24-bit identifier parsing
- Compatibility verification with Python version outputs

## Data Sources

Uses the same ITU (International Telecommunication Union) datasets as the original:
- `processed_itu_countries_regex.csv` - Country callsign patterns and ICAO ranges
- `processed_itu_organizations_regex.csv` - International organization patterns

## License and Prior Art

This project maintains the same license as the original [Flydenity](https://github.com/Collen-Roller/flydenity) project by Colleen Roller, from which this code was ported and data was copied.
