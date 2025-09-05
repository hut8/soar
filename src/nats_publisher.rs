use anyhow::Result;
use async_nats::Client;
use ogn_parser::AprsPacket;
use serde_json;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{debug, error, info, trace};

use crate::aprs_client::MessageProcessor;
use crate::device_repo::DeviceRepository;
use crate::ogn_aprs_aircraft::OgnAprsParameters;

/// Get registration number for a given device address
/// This maps OGN/FLARM addresses to aircraft registration numbers
async fn get_registration_for_address(device_repo: &DeviceRepository, address: u32, address_type: &str) -> Option<String> {
    // Convert address to device ID format expected by the device repository
    let device_id = format!("{:06X}", address);
    
    match device_repo.get_device_by_id(&device_id).await {
        Ok(Some(device)) => {
            if !device.registration.is_empty() {
                Some(device.registration)
            } else {
                debug!("Device {} found but has no registration", device_id);
                None
            }
        }
        Ok(None) => {
            debug!("No device found for address {} ({})", device_id, address_type);
            None
        }
        Err(e) => {
            error!("Failed to lookup device {}: {}", device_id, e);
            None
        }
    }
}

/// Publish APRS message to NATS
async fn publish_to_nats(nats_client: &Client, registration: &str, message: &AprsPacket) -> Result<()> {
    let subject = format!("aprs.aircraft.{}", registration);
    
    // Serialize the APRS message to JSON
    let payload = serde_json::to_vec(message)?;
    
    nats_client.publish(subject, payload.into()).await?;
    debug!("Published APRS message for {} to NATS", registration);
    
    Ok(())
}


/// NATS publisher for APRS messages keyed by aircraft registration number
pub struct NatsAprsPublisher {
    nats_client: Arc<Client>,
    device_repo: DeviceRepository,
}

impl NatsAprsPublisher {
    /// Create a new NATS publisher for APRS messages
    pub async fn new(nats_url: &str, db_pool: PgPool) -> Result<Self> {
        info!("Connecting to NATS server at {}", nats_url);
        let nats_client = async_nats::connect(nats_url).await?;
        
        Ok(Self {
            nats_client: Arc::new(nats_client),
            device_repo: DeviceRepository::new(db_pool),
        })
    }
}

impl MessageProcessor for NatsAprsPublisher {
    fn process_message(&self, message: AprsPacket) {
        // Clone the client and device repo for the async task
        let nats_client = Arc::clone(&self.nats_client);
        let device_repo = self.device_repo.clone();
        
        tokio::spawn(async move {
            // Try to extract aircraft identifier from the APRS message
            let aircraft_id = match &message.data {
                ogn_parser::AprsData::Position(pos_packet) => {
                    // First check if there's already a parsed ID field
                    if let Some(ref id) = pos_packet.comment.id {
                        let address_type = match id.address_type {
                            0 => "Random",
                            1 => "ICAO", 
                            2 => "FLARM",
                            3 => "OGN",
                            _ => "Unknown",
                        };
                        
                        // First try to look up US registration
                        if let Some(registration) = get_registration_for_address(&device_repo, id.address, address_type).await {
                            Some(registration)
                        } else {
                            // If no US registration, use the aircraft address as identifier
                            let aircraft_address = format!("{:06X}", id.address);
                            debug!("No US registration found for {}, using aircraft ID: {}", address_type, aircraft_address);
                            Some(format!("{}-{}", address_type, aircraft_address))
                        }
                    }
                    // Fallback: Try to parse OGN parameters from the comment unparsed field
                    else if let Some(ref comment) = pos_packet.comment.unparsed {
                        match comment.parse::<OgnAprsParameters>() {
                            Ok(ogn_params) => {
                                let address_type = match ogn_params.address_type {
                                    crate::ogn_aprs_aircraft::AddressType::Icao => "ICAO",
                                    crate::ogn_aprs_aircraft::AddressType::Flarm => "FLARM", 
                                    crate::ogn_aprs_aircraft::AddressType::OgnTracker => "OGN",
                                    crate::ogn_aprs_aircraft::AddressType::Unknown => "Unknown",
                                };
                                
                                // First try to look up US registration
                                if let Some(registration) = get_registration_for_address(&device_repo, ogn_params.address, address_type).await {
                                    Some(registration)
                                } else {
                                    // If no US registration, use the aircraft address as identifier
                                    let aircraft_address = format!("{:06X}", ogn_params.address);
                                    debug!("No US registration found for {}, using aircraft ID: {}", address_type, aircraft_address);
                                    Some(format!("{}-{}", address_type, aircraft_address))
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse OGN parameters from position comment '{}': {:?}", comment, e);
                                None
                            }
                        }
                    } else {
                        debug!("Position packet has no ID field or comment to parse");
                        None
                    }
                }
                ogn_parser::AprsData::Status(status_packet) => {
                    // For status messages, try to parse from comment unparsed field as well
                    if let Some(ref comment) = status_packet.comment.unparsed {
                        match comment.parse::<OgnAprsParameters>() {
                            Ok(ogn_params) => {
                                let address_type = match ogn_params.address_type {
                                    crate::ogn_aprs_aircraft::AddressType::Icao => "ICAO",
                                    crate::ogn_aprs_aircraft::AddressType::Flarm => "FLARM",
                                    crate::ogn_aprs_aircraft::AddressType::OgnTracker => "OGN", 
                                    crate::ogn_aprs_aircraft::AddressType::Unknown => "Unknown",
                                };
                                
                                // First try to look up US registration
                                if let Some(registration) = get_registration_for_address(&device_repo, ogn_params.address, address_type).await {
                                    Some(registration)
                                } else {
                                    // If no US registration, use the aircraft address as identifier
                                    let aircraft_address = format!("{:06X}", ogn_params.address);
                                    debug!("No US registration found for {}, using aircraft ID: {}", address_type, aircraft_address);
                                    Some(format!("{}-{}", address_type, aircraft_address))
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse OGN parameters from status comment '{}': {:?}", comment, e);
                                None
                            }
                        }
                    } else {
                        trace!("Status packet has no comment to parse");
                        None
                    }
                }
                _ => {
                    trace!("Unsupported APRS packet type for registration lookup");
                    None
                }
            };

            if let Some(aircraft_id) = aircraft_id {
                if let Err(e) = publish_to_nats(&nats_client, &aircraft_id, &message).await {
                    error!("Failed to publish APRS message for {}: {}", aircraft_id, e);
                } else {
                    info!("Published APRS message for aircraft {}", aircraft_id);
                }
            } else {
                debug!("No aircraft identifier found in message, skipping NATS publish");
            }
        });
    }
}