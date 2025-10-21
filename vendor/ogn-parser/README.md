
ogn-parser
==============================================================================

[![Build Status](https://travis-ci.org/Turbo87/aprs-parser-rs.svg?branch=master)](https://travis-ci.org/Turbo87/aprs-parser-rs)

[APRS] message parser for [Rust]

[APRS]: http://www.aprs.org/
[Rust]: https://www.rust-lang.org/

This is a fork of [aprs-parser-rs](https://github.com/Turbo87/aprs-parser-rs) v0.2.0.
Goal of this fork is to parse [OGN flavoured APRS strings](https://github.com/svoop/ogn_client-ruby/wiki/OGN-flavoured-APRS).

Usage
------------------------------------------------------------------------------

```rust
extern crate ogn_parser;

fn main() {
    let result = ogn_parser::parse(
        r"ICA3D1C35>OGFLR,qAS,Padova:/094220h4552.41N/01202.28E'110/099/A=003982 !W96! id053D1C35 -1187fpm +0.0rot 0.8dB 2e +4.5kHz gps1x2 s6.09 h32 rDD09D0"
    );

    println!("{:#?}", result);

    //  Ok(
    //     AprsPacket {
    //         from: Callsign {
    //             call: "ICA3D1C35",
    //             ssid: None,
    //         },
    //         to: Callsign {
    //             call: "OGFLR",
    //             ssid: None,
    //         },
    //         via: [
    //             Callsign {
    //                 call: "qAS",
    //                 ssid: None,
    //             },
    //             Callsign {
    //                 call: "Padova",
    //                 ssid: None,
    //             },
    //         ],
    //         data: Position(
    //             AprsPosition {
    //                 timestamp: Some(
    //                     HHMMSS(
    //                         9,
    //                         42,
    //                         20,
    //                     ),
    //                 ),
    //                 messaging_supported: false,
    //                 latitude: Latitude(
    //                     45.87365,
    //                 ),
    //                 longitude: Longitude(
    //                     12.0381,
    //                 ),
    //                 symbol_table: '/',
    //                 symbol_code: '\'',
    //                 comment: PositionComment {
    //                     course: Some(
    //                         110,
    //                     ),
    //                     speed: Some(
    //                         99,
    //                     ),
    //                     altitude: Some(
    //                         3982,
    //                     ),
    //                     additional_precision: Some(
    //                         AdditionalPrecision {
    //                             lat: 9,
    //                             lon: 6,
    //                         },
    //                     ),
    //                     id: Some(
    //                         ID {
    //                             address_type: 1,
    //                             aircraft_type: 1,
    //                             is_stealth: false,
    //                             is_notrack: false,
    //                             address: 4004917,
    //                         },
    //                     ),
    //                     climb_rate: Some(
    //                         -1187,
    //                     ),
    //                     turn_rate: Some(
    //                         0.0,
    //                     ),
    //                     signal_quality: Some(
    //                         0.8,
    //                     ),
    //                     error: Some(
    //                         2,
    //                     ),
    //                     frequency_offset: Some(
    //                         4.5,
    //                     ),
    //                     gps_quality: Some(
    //                         "1x2",
    //                     ),
    //                     flight_level: None,
    //                     signal_power: None,
    //                     software_version: Some(
    //                         6.09,
    //                     ),
    //                     hardware_version: Some(
    //                         50,
    //                     ),
    //                     original_address: Some(
    //                         14485968,
    //                     ),
    //                     unparsed: None,
    //                 },
    //             },
    //         ),
    //     },
    // )
}
```


License
------------------------------------------------------------------------------

This project is licensed under either of

 - Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   <http://www.apache.org/licenses/LICENSE-2.0>)

 - MIT license ([LICENSE-MIT](LICENSE-MIT) or
   <http://opensource.org/licenses/MIT>)

at your option.
