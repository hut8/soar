extern crate ogn_parser;

fn main() {
    // Example OGN packet with multi-word model, registration, and flexible !W position
    let packet = "ICA440356>OGADSB,qAS,SpainAVX:/034952h4133.38N\\00218.32E^199/372/A=012475 id25440356 -2112fpm 0rot !W07! fnA3:TAY1BC FL118 regOE-FBT modelTwin Star DA42";

    println!("Parsing OGN packet with multi-word model:");
    println!("Raw packet: {}\n", packet);

    match ogn_parser::parse(packet) {
        Ok(result) => {
            println!("Successfully parsed packet!\n");
            println!("{:#?}", result);

            // Display some key information
            if let ogn_parser::AprsData::Position(pos) = &result.data {
                println!("\n=== Key Information ===");
                println!("From: {:?}", result.from);
                println!("To: {:?}", result.to);
                println!("Via: {:?}", result.via);
                println!("Source type: {:?}", result.position_source_type());
                println!("Latitude: {:.5}°", *pos.latitude);
                println!("Longitude: {:.5}°", *pos.longitude);
                println!("Course: {:?}°", pos.comment.course);
                println!("Speed: {:?} knots", pos.comment.speed);
                println!("Altitude: {:?} feet", pos.comment.altitude);

                if let Some(id) = &pos.comment.id {
                    println!("\n=== Aircraft ID ===");
                    println!("Address: 0x{:06X}", id.address);
                    println!("Address Type: {}", id.address_type);
                    println!("Aircraft Type: {}", id.aircraft_type);
                    println!("Stealth: {}", id.is_stealth);
                    println!("No-track: {}", id.is_notrack);
                }

                println!("\n=== Telemetry ===");
                if let Some(climb_rate) = pos.comment.climb_rate {
                    println!("Climb rate: {} fpm", climb_rate);
                }
                if let Some(turn_rate) = pos.comment.turn_rate {
                    println!("Turn rate: {} rot", turn_rate);
                }
                if let Some(signal) = pos.comment.signal_quality {
                    println!("Signal quality: {} dB", signal);
                }
                if let Some(freq_offset) = pos.comment.frequency_offset {
                    println!("Frequency offset: {} kHz", freq_offset);
                }
                if let Some(gps_quality) = &pos.comment.gps_quality {
                    println!("GPS quality: {}", gps_quality);
                }
                if let Some(flight_level) = pos.comment.flight_level {
                    println!("Flight level: FL{}", flight_level);
                }

                println!("\n=== Aircraft Information ===");
                if let Some(reg) = &pos.comment.registration {
                    println!("Registration: {}", reg);
                }
                if let Some(model) = &pos.comment.model {
                    println!("Model: {}", model);
                }
                if let Some(call_sign) = &pos.comment.call_sign {
                    println!("Call sign: {}", call_sign);
                }
                if let Some(flight_number) = &pos.comment.flight_number {
                    println!("Flight number: {}", flight_number);
                }
            }
        }
        Err(e) => {
            println!("Error parsing packet: {}", e);
        }
    }
}
