use anyhow::Result;
use soar::airports::read_airports_csv_sample;

fn main() -> Result<()> {
    println!("Testing airports CSV parsing with sample data...");

    let airports = read_airports_csv_sample("/home/liam/aviation-data/ourairports-data/airports.csv", 5)?;

    println!("Successfully parsed {} airports:", airports.len());

    for (i, airport) in airports.iter().enumerate() {
        println!("\n--- Airport {} ---", i + 1);
        println!("ID: {}", airport.id);
        println!("Ident: {}", airport.ident);
        println!("Type: {}", airport.airport_type);
        println!("Name: {}", airport.name);
        println!("Location: {:?}, {:?}", airport.latitude_deg, airport.longitude_deg);
        println!("Elevation: {:?} ft", airport.elevation_ft);
        println!("Country: {:?}", airport.iso_country);
        println!("Region: {:?}", airport.iso_region);
        println!("Municipality: {:?}", airport.municipality);
        println!("Scheduled Service: {}", airport.scheduled_service);
        println!("ICAO: {:?}", airport.icao_code);
        println!("IATA: {:?}", airport.iata_code);
        println!("GPS Code: {:?}", airport.gps_code);
        println!("Local Code: {:?}", airport.local_code);
    }

    Ok(())
}
