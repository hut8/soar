/// Module to trigger TypeScript type generation via ts-rs
/// Run `cargo test ts_export` to generate TypeScript types
#[cfg(test)]
mod tests {
    use ts_rs::TS;

    use crate::actions::views::{Aircraft, AircraftView};
    use crate::fixes::Fix;
    use crate::ogn_aprs_aircraft::AircraftType;

    #[test]
    fn export_types() {
        // Calling export() generates the .ts files
        Fix::export().expect("Failed to export Fix type");
        AircraftView::export().expect("Failed to export AircraftView type");
        Aircraft::export().expect("Failed to export Aircraft type");
        AircraftType::export().expect("Failed to export AircraftType type");
    }
}
