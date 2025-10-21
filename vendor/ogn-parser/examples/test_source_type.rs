use ogn_parser::AprsPacket;

fn main() {
    // Test receiver packet (contains TCPIP* and qAC)
    let receiver_packet = "AVX1053>OGNSDR,TCPIP*,qAC,GLIDERN3:/190916h6022.40NI00512.27E&/A=000049 AVIONIX ENGINEERING ADS-B/OGN receiver";

    // Test aircraft packet (contains qAS)
    let aircraft_packet = "ICA3D17F2>APRS,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054";

    // Test unknown packet (no special via indicators)
    let unknown_packet =
        "TEST123>APRS,WIDE1-1,WIDE2-1:/074849h4821.61N\\01224.49E^322/103/A=003054";

    // Test non-position packet
    let status_packet = "TEST123>APRS,WIDE1-1:>Status message";

    println!("Testing receiver packet:");
    println!("Packet: {receiver_packet}");
    test_packet(receiver_packet);

    println!("\nTesting aircraft packet:");
    println!("Packet: {aircraft_packet}");
    test_packet(aircraft_packet);

    println!("\nTesting unknown packet:");
    println!("Packet: {unknown_packet}");
    test_packet(unknown_packet);

    println!("\nTesting non-position packet:");
    println!("Packet: {status_packet}");
    test_packet(status_packet);
}

fn test_packet(packet_str: &str) {
    match packet_str.parse::<AprsPacket>() {
        Ok(packet) => {
            println!("  Via: {:?}", packet.via);
            println!("  Source type: {:?}", packet.position_source_type());
        }
        Err(e) => {
            println!("  Error parsing: {e}");
        }
    }
    println!();
}
