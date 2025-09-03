use anyhow::Result;
use soar::runways::read_runways_csv_sample;

fn main() -> Result<()> {
    println!("Testing runways CSV parsing with sample data...");

    let runways = read_runways_csv_sample("/home/liam/aviation-data/ourairports-data/runways.csv", 5)?;

    println!("Successfully parsed {} runways:", runways.len());

    for (i, runway) in runways.iter().enumerate() {
        println!("\n--- Runway {} ---", i + 1);
        println!("ID: {}", runway.id);
        println!("Airport Ref: {}", runway.airport_ref);
        println!("Airport Ident: {}", runway.airport_ident);
        println!("Dimensions: {:?} x {:?} ft", runway.length_ft, runway.width_ft);
        println!("Surface: {:?}", runway.surface);
        println!("Lighted: {}", runway.lighted);
        println!("Closed: {}", runway.closed);

        println!("Low End:");
        println!("  Ident: {:?}", runway.le_ident);
        println!("  Location: {:?}, {:?}", runway.le_latitude_deg, runway.le_longitude_deg);
        println!("  Elevation: {:?} ft", runway.le_elevation_ft);
        println!("  Heading: {:?}°T", runway.le_heading_degt);
        println!("  Displaced Threshold: {:?} ft", runway.le_displaced_threshold_ft);

        println!("High End:");
        println!("  Ident: {:?}", runway.he_ident);
        println!("  Location: {:?}, {:?}", runway.he_latitude_deg, runway.he_longitude_deg);
        println!("  Elevation: {:?} ft", runway.he_elevation_ft);
        println!("  Heading: {:?}°T", runway.he_heading_degt);
        println!("  Displaced Threshold: {:?} ft", runway.he_displaced_threshold_ft);
    }

    Ok(())
}
