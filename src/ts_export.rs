/// Module to trigger TypeScript type generation via ts-rs
/// Run `cargo test ts_export` to generate TypeScript types
#[cfg(test)]
mod tests {
    use ts_rs::TS;

    use crate::actions::club_tow_fees::TowFeeView;
    use crate::actions::views::{Aircraft, AircraftView, ClubView};
    use crate::aircraft_types::AircraftCategory;
    use crate::fixes::Fix;

    #[test]
    fn export_types() {
        // Calling export() generates the .ts files
        Fix::export().expect("Failed to export Fix type");
        AircraftView::export().expect("Failed to export AircraftView type");
        Aircraft::export().expect("Failed to export Aircraft type");
        AircraftCategory::export().expect("Failed to export AircraftCategory type");
        ClubView::export().expect("Failed to export ClubView type");
        TowFeeView::export().expect("Failed to export TowFeeView type");
    }
}
