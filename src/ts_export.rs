/// Module to trigger TypeScript type generation via ts-rs
/// Run `cargo test ts_export` to generate TypeScript types
#[cfg(test)]
mod tests {
    use ts_rs::{Config, TS};

    use crate::actions::club_tow_fees::TowFeeView;
    use crate::actions::geocoding::ReverseGeocodeResponse;
    use crate::actions::payments::{CheckoutResponse, CreateChargeRequest, PaymentView};
    use crate::actions::stripe_connect::{
        StripeConnectStatusView, StripeDashboardLinkResponse, StripeOnboardingResponse,
    };
    use crate::actions::views::{
        Aircraft, AircraftModelView, AircraftRegistrationView, AircraftView, AirportView, ClubView,
        FlightView, ModelDataView, ReceiverView, RunwayEnd, RunwayView, UserView,
    };
    use crate::aircraft::AddressType;
    use crate::aircraft_registrations::{AirworthinessClass, LightSportType, RegistrantType};
    use crate::aircraft_types::AircraftCategory;
    use crate::fixes::Fix;
    use crate::flights::FlightState;
    use crate::geofence::{
        AircraftGeofence, CreateGeofenceRequest, Geofence, GeofenceDetailResponse,
        GeofenceExitEvent, GeofenceExitEventsResponse, GeofenceLayer, GeofenceListResponse,
        GeofenceSubscriber, GeofenceWithCounts, UpdateGeofenceRequest,
    };
    use crate::ingest_config::{DataStream, StreamFormat};
    use crate::payments::{PaymentStatus, PaymentType};

    #[test]
    fn export_types() {
        // In ts-rs 12.0+, export() requires a Config parameter
        // Config::from_env() reads configuration from environment variables
        let cfg = &Config::from_env();

        // Calling export() generates the .ts files
        Fix::export(cfg).expect("Failed to export Fix type");
        AircraftView::export(cfg).expect("Failed to export AircraftView type");
        ModelDataView::export(cfg).expect("Failed to export ModelDataView type");
        Aircraft::export(cfg).expect("Failed to export Aircraft type");
        AircraftCategory::export(cfg).expect("Failed to export AircraftCategory type");
        AircraftRegistrationView::export(cfg)
            .expect("Failed to export AircraftRegistrationView type");
        AircraftModelView::export(cfg).expect("Failed to export AircraftModelView type");
        AirworthinessClass::export(cfg).expect("Failed to export AirworthinessClass type");
        LightSportType::export(cfg).expect("Failed to export LightSportType type");
        RegistrantType::export(cfg).expect("Failed to export RegistrantType type");
        ClubView::export(cfg).expect("Failed to export ClubView type");
        TowFeeView::export(cfg).expect("Failed to export TowFeeView type");
        FlightView::export(cfg).expect("Failed to export FlightView type");
        FlightState::export(cfg).expect("Failed to export FlightState type");
        AddressType::export(cfg).expect("Failed to export AddressType type");
        UserView::export(cfg).expect("Failed to export UserView type");
        ReceiverView::export(cfg).expect("Failed to export ReceiverView type");
        AirportView::export(cfg).expect("Failed to export AirportView type");
        RunwayView::export(cfg).expect("Failed to export RunwayView type");
        RunwayEnd::export(cfg).expect("Failed to export RunwayEnd type");

        // Geocoding types
        ReverseGeocodeResponse::export(cfg).expect("Failed to export ReverseGeocodeResponse type");

        // Geofence types
        GeofenceLayer::export(cfg).expect("Failed to export GeofenceLayer type");
        Geofence::export(cfg).expect("Failed to export Geofence type");
        CreateGeofenceRequest::export(cfg).expect("Failed to export CreateGeofenceRequest type");
        UpdateGeofenceRequest::export(cfg).expect("Failed to export UpdateGeofenceRequest type");
        GeofenceSubscriber::export(cfg).expect("Failed to export GeofenceSubscriber type");
        AircraftGeofence::export(cfg).expect("Failed to export AircraftGeofence type");
        GeofenceExitEvent::export(cfg).expect("Failed to export GeofenceExitEvent type");
        GeofenceWithCounts::export(cfg).expect("Failed to export GeofenceWithCounts type");
        GeofenceListResponse::export(cfg).expect("Failed to export GeofenceListResponse type");
        GeofenceDetailResponse::export(cfg).expect("Failed to export GeofenceDetailResponse type");
        GeofenceExitEventsResponse::export(cfg)
            .expect("Failed to export GeofenceExitEventsResponse type");

        // Data stream types
        DataStream::export(cfg).expect("Failed to export DataStream type");
        StreamFormat::export(cfg).expect("Failed to export StreamFormat type");

        // Payment types
        PaymentType::export(cfg).expect("Failed to export PaymentType type");
        PaymentStatus::export(cfg).expect("Failed to export PaymentStatus type");
        PaymentView::export(cfg).expect("Failed to export PaymentView type");
        CreateChargeRequest::export(cfg).expect("Failed to export CreateChargeRequest type");
        CheckoutResponse::export(cfg).expect("Failed to export CheckoutResponse type");

        // Stripe Connect types
        StripeOnboardingResponse::export(cfg)
            .expect("Failed to export StripeOnboardingResponse type");
        StripeConnectStatusView::export(cfg)
            .expect("Failed to export StripeConnectStatusView type");
        StripeDashboardLinkResponse::export(cfg)
            .expect("Failed to export StripeDashboardLinkResponse type");
    }
}
