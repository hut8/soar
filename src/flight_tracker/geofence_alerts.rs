//! Geofence alert processing
//!
//! Handles detection of geofence exits and sending email alerts to subscribers.

use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::Fix;
use crate::email::{AircraftEmailData, EmailService, GeofenceExitEmailData};
use crate::geofence::{Geofence, GeofenceExitEvent, GeofenceLayer};
use crate::geofence_repo::GeofenceRepository;
use crate::users_repo::UsersRepository;

use super::aircraft_state::GeofenceStatus;
use super::geofence_detector::{check_fix_against_geofence, has_exited_geofence, is_inside};

/// Check a fix against all geofences for an aircraft and detect exits
///
/// Returns a list of exit events that should be recorded and alerted.
pub async fn check_geofences_for_aircraft(
    fix: &Fix,
    previous_status: &GeofenceStatus,
    geofence_repo: &GeofenceRepository,
) -> Result<(Vec<(Geofence, GeofenceLayer)>, GeofenceStatus)> {
    // Get all geofences linked to this aircraft
    let geofences = geofence_repo
        .get_geofences_for_aircraft(fix.aircraft_id)
        .await?;

    if geofences.is_empty() {
        return Ok((vec![], GeofenceStatus::new()));
    }

    let mut exits = Vec::new();
    let mut new_status = GeofenceStatus::new();

    for geofence in geofences {
        let result = check_fix_against_geofence(fix, &geofence);
        let currently_inside = is_inside(&result);

        // Check for exit transition
        let was_inside = previous_status.get(&geofence.id).copied().unwrap_or(false);

        if let Some(exited_layer) = has_exited_geofence(was_inside, &result) {
            info!(
                "Aircraft {} exited geofence '{}' (layer: {}-{} ft, {} nm)",
                fix.aircraft_id,
                geofence.name,
                exited_layer.floor_ft,
                exited_layer.ceiling_ft,
                exited_layer.radius_nm
            );
            exits.push((geofence.clone(), exited_layer));
        }

        // Update status for next check
        new_status.insert(geofence.id, currently_inside);
    }

    Ok((exits, new_status))
}

/// Process geofence exits: create events and send alerts
///
/// This is called when exits are detected. It:
/// 1. Creates exit event records in the database
/// 2. Sends email alerts to all subscribers
#[allow(clippy::too_many_arguments)]
pub async fn process_geofence_exits(
    fix: &Fix,
    exits: Vec<(Geofence, GeofenceLayer)>,
    geofence_repo: &GeofenceRepository,
    users_repo: &UsersRepository,
    aircraft_registration: Option<String>,
    aircraft_model: String,
    hex_address: String,
) {
    let flight_id = match fix.flight_id {
        Some(id) => id,
        None => {
            warn!(
                "Cannot process geofence exit for aircraft {} - no flight_id on fix",
                fix.aircraft_id
            );
            return;
        }
    };

    for (geofence, exited_layer) in exits {
        // Create exit event record
        let event = match geofence_repo
            .create_exit_event(
                geofence.id,
                flight_id,
                fix.aircraft_id,
                fix.received_at,
                fix.latitude,
                fix.longitude,
                fix.altitude_msl_feet,
                &exited_layer,
            )
            .await
        {
            Ok(event) => {
                metrics::counter!("geofence.exit_events_created_total").increment(1);
                event
            }
            Err(e) => {
                error!(
                    "Failed to create geofence exit event for geofence {}: {}",
                    geofence.id, e
                );
                continue;
            }
        };

        // Send email alerts in background
        let geofence_repo_clone = geofence_repo.clone();
        let users_repo_clone = users_repo.clone();
        let aircraft_registration = aircraft_registration.clone();
        let aircraft_model = aircraft_model.clone();
        let hex_address = hex_address.clone();
        let aircraft_id = fix.aircraft_id;

        tokio::spawn(async move {
            send_geofence_exit_alerts(
                event,
                geofence,
                exited_layer,
                geofence_repo_clone,
                users_repo_clone,
                aircraft_id,
                aircraft_registration,
                aircraft_model,
                hex_address,
            )
            .await;
        });
    }
}

/// Send email alerts for a geofence exit event
#[allow(clippy::too_many_arguments)]
async fn send_geofence_exit_alerts(
    event: GeofenceExitEvent,
    geofence: Geofence,
    exited_layer: GeofenceLayer,
    geofence_repo: GeofenceRepository,
    users_repo: UsersRepository,
    aircraft_id: Uuid,
    aircraft_registration: Option<String>,
    aircraft_model: String,
    hex_address: String,
) {
    // Get subscribers who want email
    let subscriber_ids = match geofence_repo.get_subscribers_for_email(geofence.id).await {
        Ok(ids) => ids,
        Err(e) => {
            error!(
                "Failed to get subscribers for geofence {}: {}",
                geofence.id, e
            );
            return;
        }
    };

    if subscriber_ids.is_empty() {
        return;
    }

    // Create email service
    let email_service = match EmailService::new() {
        Ok(service) => Arc::new(service),
        Err(e) => {
            error!("Failed to create email service for geofence alerts: {}", e);
            return;
        }
    };

    // Build email data
    let exit_data = GeofenceExitEmailData {
        geofence_name: geofence.name.clone(),
        geofence_id: geofence.id,
        flight_id: event.flight_id,
        aircraft: AircraftEmailData {
            id: aircraft_id,
            registration: aircraft_registration,
            aircraft_model,
            hex_address,
        },
        exit_time: event.exit_time,
        exit_latitude: event.exit_latitude,
        exit_longitude: event.exit_longitude,
        exit_altitude_msl_ft: event.exit_altitude_msl_ft,
        exit_layer_floor_ft: exited_layer.floor_ft,
        exit_layer_ceiling_ft: exited_layer.ceiling_ft,
        exit_layer_radius_nm: exited_layer.radius_nm,
    };

    let mut emails_sent = 0;

    // Send email to each subscriber
    for user_id in subscriber_ids {
        let user = match users_repo.get_by_id(user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                warn!("User {} not found for geofence email", user_id);
                continue;
            }
            Err(e) => {
                error!("Failed to get user {} for geofence email: {}", user_id, e);
                continue;
            }
        };

        // Skip if user has no email or not verified
        let email = match &user.email {
            Some(e) if user.email_verified => e,
            _ => continue,
        };

        let to_name = format!("{} {}", user.first_name, user.last_name);

        match email_service
            .send_geofence_exit_email(email, &to_name, &exit_data)
            .await
        {
            Ok(_) => {
                emails_sent += 1;
                metrics::counter!("geofence.alert_emails_sent_total").increment(1);
            }
            Err(e) => {
                error!("Failed to send geofence exit email to {}: {}", email, e);
                metrics::counter!("geofence.alert_emails_failed_total").increment(1);
            }
        }
    }

    // Update exit event with count of emails sent
    if emails_sent > 0 {
        if let Err(e) = geofence_repo
            .update_exit_event_email_count(event.id, emails_sent)
            .await
        {
            error!(
                "Failed to update email count for exit event {}: {}",
                event.id, e
            );
        }

        info!(
            "Sent {} geofence exit alert emails for geofence '{}', aircraft {}",
            emails_sent, geofence.name, aircraft_id
        );
    }
}
