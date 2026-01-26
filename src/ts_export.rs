/// Module to trigger TypeScript type generation via ts-rs
/// Run `cargo test ts_export` to generate TypeScript types
#[cfg(test)]
mod tests {
    use ts_rs::TS;

    use crate::actions::club_tow_fees::TowFeeView;
    use crate::actions::views::{Aircraft, AircraftView, ClubView};
    use crate::aircraft_types::AircraftCategory;
    use crate::fixes::Fix;
    use crate::geofence::{
        AircraftGeofence, CreateGeofenceRequest, Geofence, GeofenceDetailResponse,
        GeofenceExitEvent, GeofenceExitEventsResponse, GeofenceLayer, GeofenceListResponse,
        GeofenceSubscriber, GeofenceWithCounts, UpdateGeofenceRequest,
    };

    #[test]
    fn export_types() {
        // Calling export() generates the .ts files
        Fix::export().expect("Failed to export Fix type");
        AircraftView::export().expect("Failed to export AircraftView type");
        Aircraft::export().expect("Failed to export Aircraft type");
        AircraftCategory::export().expect("Failed to export AircraftCategory type");
        ClubView::export().expect("Failed to export ClubView type");
        TowFeeView::export().expect("Failed to export TowFeeView type");

        // Geofence types
        GeofenceLayer::export().expect("Failed to export GeofenceLayer type");
        Geofence::export().expect("Failed to export Geofence type");
        CreateGeofenceRequest::export().expect("Failed to export CreateGeofenceRequest type");
        UpdateGeofenceRequest::export().expect("Failed to export UpdateGeofenceRequest type");
        GeofenceSubscriber::export().expect("Failed to export GeofenceSubscriber type");
        AircraftGeofence::export().expect("Failed to export AircraftGeofence type");
        GeofenceExitEvent::export().expect("Failed to export GeofenceExitEvent type");
        GeofenceWithCounts::export().expect("Failed to export GeofenceWithCounts type");
        GeofenceListResponse::export().expect("Failed to export GeofenceListResponse type");
        GeofenceDetailResponse::export().expect("Failed to export GeofenceDetailResponse type");
        GeofenceExitEventsResponse::export()
            .expect("Failed to export GeofenceExitEventsResponse type");
    }
}
