use anyhow::Result;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::airports_repo::AirportsRepository;
use crate::fixes_repo::FixesRepository;
use crate::flights_repo::FlightsRepository;
use crate::geocoding::Geocoder;
use crate::locations::Location;
use crate::locations_repo::LocationsRepository;

/// TEMPORARY: Reverse geocoding is disabled for flight takeoffs and landings
/// When this is false, the FlightLocationProcessor will not perform reverse geocoding
/// This is a temporary measure to avoid unnecessary geocoding API calls
const GEOCODING_ENABLED_FOR_FLIGHTS: bool = false;

/// Background processor that adds location data to completed flights
/// This runs periodically and processes flights that don't have location data yet
pub struct FlightLocationProcessor {
    flights_repo: FlightsRepository,
    fixes_repo: FixesRepository,
    airports_repo: AirportsRepository,
    locations_repo: LocationsRepository,
    geocoder: Geocoder,
}

impl FlightLocationProcessor {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            flights_repo: FlightsRepository::new(pool.clone()),
            fixes_repo: FixesRepository::new(pool.clone()),
            airports_repo: AirportsRepository::new(pool.clone()),
            locations_repo: LocationsRepository::new(pool.clone()),
            geocoder: Geocoder::new(),
        }
    }

    /// Start the background processor that runs periodically
    pub fn start(pool: Pool<ConnectionManager<PgConnection>>, interval_secs: u64) {
        tokio::spawn(async move {
            let processor = Self::new(pool);
            let mut ticker = interval(Duration::from_secs(interval_secs));

            info!(
                "Started flight location processor (running every {} seconds)",
                interval_secs
            );

            loop {
                ticker.tick().await;

                if let Err(e) = processor.process_flights_needing_locations().await {
                    error!("Error processing flight locations: {}", e);
                }
            }
        });
    }

    /// Process a batch of flights that need location data
    async fn process_flights_needing_locations(&self) -> Result<()> {
        // GEOCODING DISABLED: Skip processing if geocoding is disabled
        if !GEOCODING_ENABLED_FOR_FLIGHTS {
            debug!("Flight location geocoding is temporarily disabled, skipping");
            return Ok(());
        }

        // Get completed flights without location data (limit to 10 per batch to be nice to Nominatim)
        let flights = self
            .flights_repo
            .get_completed_flights_without_locations(10)
            .await?;

        if flights.is_empty() {
            debug!("No flights need location data");
            return Ok(());
        }

        info!(
            "Processing {} flights that need location data",
            flights.len()
        );

        for flight in flights {
            if let Err(e) = self.process_single_flight(&flight.id).await {
                warn!(
                    "Failed to process location data for flight {}: {}",
                    flight.id, e
                );
            }

            // Rate limiting: Nominatim allows max 1 request per second
            // We do 2 requests per flight (takeoff + landing), so wait 2.5 seconds between flights
            tokio::time::sleep(Duration::from_millis(2500)).await;
        }

        Ok(())
    }

    /// Process a single flight to add location data
    async fn process_single_flight(&self, flight_id: &Uuid) -> Result<()> {
        let flight = self
            .flights_repo
            .get_flight_by_id(*flight_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Flight not found"))?;

        // Skip if locations are already set
        if flight.takeoff_location_id.is_some() && flight.landing_location_id.is_some() {
            return Ok(());
        }

        // Get takeoff coordinates from the first fix
        let takeoff_location_id = if flight.takeoff_location_id.is_none() {
            self.get_or_create_takeoff_location(&flight).await?
        } else {
            flight.takeoff_location_id
        };

        // Get landing coordinates from the last fix
        let landing_location_id = if flight.landing_location_id.is_none() {
            self.get_or_create_landing_location(&flight).await?
        } else {
            flight.landing_location_id
        };

        // Update the flight with location IDs
        if takeoff_location_id.is_some() || landing_location_id.is_some() {
            self.flights_repo
                .update_flight_locations(*flight_id, takeoff_location_id, landing_location_id)
                .await?;

            info!(
                "Updated flight {} with location data (takeoff: {:?}, landing: {:?})",
                flight_id, takeoff_location_id, landing_location_id
            );
        }

        Ok(())
    }

    /// Get or create location for takeoff
    async fn get_or_create_takeoff_location(
        &self,
        flight: &crate::flights::Flight,
    ) -> Result<Option<Uuid>> {
        // Get first fix for this flight to find takeoff coordinates
        let fixes = self
            .fixes_repo
            .get_fixes_for_flight(flight.id, Some(1))
            .await?;

        if let Some(first_fix) = fixes.first() {
            let latitude = first_fix.latitude;
            let longitude = first_fix.longitude;

            return self
                .create_location_from_coordinates(latitude, longitude)
                .await;
        }

        Ok(None)
    }

    /// Get or create location for landing
    async fn get_or_create_landing_location(
        &self,
        flight: &crate::flights::Flight,
    ) -> Result<Option<Uuid>> {
        // Get last fix for this flight to find landing coordinates
        // We need to get all fixes and take the last one since there's no direct "last fix" query
        let fixes = self
            .fixes_repo
            .get_fixes_for_flight(flight.id, None)
            .await?;

        if let Some(last_fix) = fixes.last() {
            let latitude = last_fix.latitude;
            let longitude = last_fix.longitude;

            return self
                .create_location_from_coordinates(latitude, longitude)
                .await;
        }

        Ok(None)
    }

    /// Create a location from coordinates using reverse geocoding
    /// Note: This method is not called when GEOCODING_ENABLED_FOR_FLIGHTS is false
    async fn create_location_from_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Option<Uuid>> {
        debug!(
            "Performing reverse geocoding for coordinates ({}, {})",
            latitude, longitude
        );

        match self.geocoder.reverse_geocode(latitude, longitude).await {
            Ok(result) => {
                debug!(
                    "Reverse geocoded ({}, {}) to: {}",
                    latitude, longitude, result.display_name
                );

                // Create a location with the reverse geocoded address
                let location = Location::new(
                    result.street1,
                    None, // street2
                    result.city,
                    result.state,
                    result.zip_code,
                    None, // region_code
                    result.country.map(|c| {
                        // Take first 2 characters for country code
                        c.chars().take(2).collect()
                    }),
                    Some(crate::locations::Point::new(latitude, longitude)),
                );

                // Use find_or_create to avoid duplicate locations
                match self
                    .locations_repo
                    .find_or_create(
                        location.street1.clone(),
                        location.street2.clone(),
                        location.city.clone(),
                        location.state.clone(),
                        location.zip_code.clone(),
                        location.region_code.clone(),
                        location.country_mail_code.clone(),
                        location.geolocation.clone(),
                    )
                    .await
                {
                    Ok(created_location) => {
                        info!(
                            "Created/found location {} for coordinates ({}, {}): {}",
                            created_location.id, latitude, longitude, result.display_name
                        );
                        Ok(Some(created_location.id))
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create location for coordinates ({}, {}): {}",
                            latitude, longitude, e
                        );
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Reverse geocoding failed for coordinates ({}, {}): {}",
                    latitude, longitude, e
                );
                Ok(None)
            }
        }
    }
}
