use anyhow::{Context, Result};
use dashmap::DashMap;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::upsert::excluded;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::aircraft::{AddressType, Aircraft, AircraftModel, NewAircraft};
use crate::aircraft_types::AircraftCategory;
use crate::ogn_aprs_aircraft::AdsbEmitterCategory;
use crate::schema::aircraft;
use chrono::{DateTime, Utc};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Statistics from merging duplicate aircraft by pending_registration.
#[derive(Debug, Clone, Default)]
pub struct MergeStats {
    pub duplicates_found: usize,
    pub aircraft_merged: usize,
    pub aircraft_deleted: usize,
    pub fixes_reassigned: usize,
    pub flights_reassigned: usize,
    pub registrations_claimed: usize,
    pub errors: Vec<String>,
}

/// Fields extracted from packet for device creation/update
#[derive(Debug, Clone)]
pub struct AircraftPacketFields {
    pub aircraft_category: Option<AircraftCategory>,
    pub aircraft_model: Option<String>,
    pub icao_model_code: Option<String>,
    pub adsb_emitter_category: Option<AdsbEmitterCategory>,
    pub tracker_device_type: Option<String>,
    pub registration: Option<String>,
}

#[derive(Clone)]
pub struct AircraftRepository {
    pool: PgPool,
}

impl AircraftRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn get_connection(&self) -> Result<PgPooledConnection> {
        self.pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))
    }

    /// Upsert aircraft into the database
    /// This will insert new aircraft or update existing ones based on typed address columns.
    /// Each DDB record has a single address type, so we branch on which typed address column
    /// is populated to determine the ON CONFLICT target.
    pub async fn upsert_aircraft<I>(&self, aircraft_iter: I) -> Result<usize>
    where
        I: IntoIterator<Item = Aircraft>,
    {
        let mut conn = self.get_connection()?;
        let mut upserted_count = 0;

        // Convert aircraft to NewAircraft structs for insertion
        let new_aircraft: Vec<NewAircraft> = aircraft_iter.into_iter().map(|d| d.into()).collect();

        // Common DO UPDATE set fields used by all address type branches
        macro_rules! ddb_upsert_set {
            () => {(
                // Update fields from DDB, but preserve existing values if DDB value is empty
                // Use COALESCE(NULLIF(new, ''), old) to keep existing data when DDB has empty strings
                aircraft::aircraft_model.eq(diesel::dsl::sql(
                    "COALESCE(NULLIF(EXCLUDED.aircraft_model, ''), aircraft.aircraft_model)"
                )),
                aircraft::registration.eq(diesel::dsl::sql(
                    "COALESCE(NULLIF(EXCLUDED.registration, ''), aircraft.registration)"
                )),
                aircraft::competition_number.eq(diesel::dsl::sql(
                    "COALESCE(NULLIF(EXCLUDED.competition_number, ''), aircraft.competition_number)"
                )),
                aircraft::tracked.eq(excluded(aircraft::tracked)),
                aircraft::identified.eq(excluded(aircraft::identified)),
                aircraft::from_ogn_ddb.eq(excluded(aircraft::from_ogn_ddb)),
                // For Option fields, use COALESCE to prefer new value over NULL, but keep old if new is NULL
                aircraft::frequency_mhz.eq(diesel::dsl::sql(
                    "COALESCE(EXCLUDED.frequency_mhz, aircraft.frequency_mhz)"
                )),
                aircraft::pilot_name.eq(diesel::dsl::sql(
                    "COALESCE(EXCLUDED.pilot_name, aircraft.pilot_name)"
                )),
                aircraft::home_base_airport_ident.eq(diesel::dsl::sql(
                    "COALESCE(EXCLUDED.home_base_airport_ident, aircraft.home_base_airport_ident)"
                )),
                aircraft::country_code.eq(diesel::dsl::sql(
                    "COALESCE(EXCLUDED.country_code, aircraft.country_code)"
                )),
                aircraft::updated_at.eq(diesel::dsl::now),
                // NOTE: We do NOT update the following fields because they come from real-time packets:
                // - aircraft_category (from OGN packets)
                // - icao_model_code (from ADSB packets)
                // - adsb_emitter_category (from ADSB packets)
                // - tracker_device_type (from tracker packets)
                // - last_fix_at (managed by fix processing)
                // - club_id (managed by club assignment logic)
            )}
        }

        for new_aircraft_entry in new_aircraft {
            // Branch on which typed address column is populated to determine ON CONFLICT target.
            // Diesel requires the conflict column to be known at compile time.
            let result = if new_aircraft_entry.icao_address.is_some() {
                diesel::insert_into(aircraft::table)
                    .values(&new_aircraft_entry)
                    .on_conflict(aircraft::icao_address)
                    .do_update()
                    .set(ddb_upsert_set!())
                    .execute(&mut conn)
            } else if new_aircraft_entry.flarm_address.is_some() {
                diesel::insert_into(aircraft::table)
                    .values(&new_aircraft_entry)
                    .on_conflict(aircraft::flarm_address)
                    .do_update()
                    .set(ddb_upsert_set!())
                    .execute(&mut conn)
            } else if new_aircraft_entry.ogn_address.is_some() {
                diesel::insert_into(aircraft::table)
                    .values(&new_aircraft_entry)
                    .on_conflict(aircraft::ogn_address)
                    .do_update()
                    .set(ddb_upsert_set!())
                    .execute(&mut conn)
            } else if new_aircraft_entry.other_address.is_some() {
                diesel::insert_into(aircraft::table)
                    .values(&new_aircraft_entry)
                    .on_conflict(aircraft::other_address)
                    .do_update()
                    .set(ddb_upsert_set!())
                    .execute(&mut conn)
            } else {
                warn!("Skipping aircraft with no address columns set");
                continue;
            };

            match result {
                Ok(_) => {
                    upserted_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Failed to upsert aircraft {}: {}",
                        new_aircraft_entry.address_hex(),
                        e
                    );
                    // Continue with other aircraft rather than failing the entire batch
                }
            }
        }

        info!("Successfully upserted {} aircraft", upserted_count);
        Ok(upserted_count)
    }

    /// Get the total count of aircraft in the database
    pub async fn get_aircraft_count(&self) -> Result<i64> {
        let mut conn = self.get_connection()?;
        let count = aircraft::table.count().get_result::<i64>(&mut conn)?;
        Ok(count)
    }

    /// Get an aircraft by its address and type.
    /// Uses the specific typed address column for lookup.
    pub async fn get_aircraft_by_address(
        &self,
        address: u32,
        address_type: AddressType,
    ) -> Result<Option<Aircraft>> {
        let mut conn = self.get_connection()?;
        let addr = address as i32;
        let model = match address_type {
            AddressType::Icao => aircraft::table
                .filter(aircraft::icao_address.eq(addr))
                .select(AircraftModel::as_select())
                .first(&mut conn)
                .optional()?,
            AddressType::Flarm => aircraft::table
                .filter(aircraft::flarm_address.eq(addr))
                .select(AircraftModel::as_select())
                .first(&mut conn)
                .optional()?,
            AddressType::Ogn => aircraft::table
                .filter(aircraft::ogn_address.eq(addr))
                .select(AircraftModel::as_select())
                .first(&mut conn)
                .optional()?,
            AddressType::Unknown => aircraft::table
                .filter(aircraft::other_address.eq(addr))
                .select(AircraftModel::as_select())
                .first(&mut conn)
                .optional()?,
        };

        Ok(model.map(|model| model.into()))
    }

    /// Get an aircraft model (with UUID) by address and type.
    /// Uses the specific typed address column for lookup.
    pub async fn get_aircraft_model_by_address(
        &self,
        address: i32,
        address_type: AddressType,
    ) -> Result<Option<AircraftModel>> {
        let mut conn = self.get_connection()?;
        let model = match address_type {
            AddressType::Icao => aircraft::table
                .filter(aircraft::icao_address.eq(address))
                .select(AircraftModel::as_select())
                .first(&mut conn)
                .optional()?,
            AddressType::Flarm => aircraft::table
                .filter(aircraft::flarm_address.eq(address))
                .select(AircraftModel::as_select())
                .first(&mut conn)
                .optional()?,
            AddressType::Ogn => aircraft::table
                .filter(aircraft::ogn_address.eq(address))
                .select(AircraftModel::as_select())
                .first(&mut conn)
                .optional()?,
            AddressType::Unknown => aircraft::table
                .filter(aircraft::other_address.eq(address))
                .select(AircraftModel::as_select())
                .first(&mut conn)
                .optional()?,
        };
        Ok(model)
    }

    /// Get or insert an aircraft by address
    /// If the aircraft doesn't exist, it will be created with from_ogn_ddb=false, tracked=true, identified=true
    /// Uses INSERT ... ON CONFLICT to handle race conditions atomically.
    /// If a registration is computed from the ICAO address, it is only applied when no other
    /// aircraft already owns that registration (prevents unique constraint violations).
    #[tracing::instrument(skip(self), fields(%address, ?address_type))]
    pub async fn get_or_insert_aircraft_by_address(
        &self,
        address: i32,
        address_type: AddressType,
    ) -> Result<AircraftModel> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

            // Extract country code from ICAO address if applicable
            let country_code =
                Aircraft::extract_country_code_from_icao(address as u32, address_type);

            // Extract tail number from ICAO address if it's a US aircraft
            let registration =
                Aircraft::extract_tail_number_from_icao(address as u32, address_type)
                    .unwrap_or_default();

            // Route address to the correct typed column
            let (icao_address, flarm_address, ogn_address, other_address) = match address_type {
                AddressType::Icao => (Some(address), None, None, None),
                AddressType::Flarm => (None, Some(address), None, None),
                AddressType::Ogn => (None, None, Some(address), None),
                AddressType::Unknown => (None, None, None, Some(address)),
            };

            // If we have a computed registration, try to merge into an existing aircraft
            // that already has this registration but is missing our address type.
            if !registration.is_empty()
                && let Some(model) =
                    Self::merge_by_registration(&registration, address, address_type, &mut conn)?
            {
                return Ok(model);
            }

            let new_aircraft = NewAircraft {
                icao_address,
                flarm_address,
                ogn_address,
                other_address,
                aircraft_model: String::new(),
                // Always insert with NULL registration to avoid violating the
                // idx_aircraft_registration_unique constraint. The DO UPDATE clause
                // will safely set the registration (with a duplicate check) on the
                // next fix for this aircraft.
                registration: None,
                competition_number: String::new(),
                tracked: true,
                identified: true,
                from_ogn_ddb: false,
                from_adsbx_ddb: false,
                frequency_mhz: None,
                pilot_name: None,
                home_base_airport_ident: None,
                last_fix_at: None,
                club_id: None,
                icao_model_code: None,
                adsb_emitter_category: None,
                tracker_device_type: None,
                country_code,
                latitude: None,
                longitude: None,
                owner_operator: None,
                aircraft_category: None,
                engine_count: None,
                engine_type: None,
                faa_pia: None,
                faa_ladd: None,
                year: None,
                is_military: None,
                current_fix: None,
                images: None,
                pending_registration: None,
            };

            // Use INSERT ... ON CONFLICT ... DO UPDATE RETURNING to atomically handle race conditions
            // This ensures we always get an aircraft_id, even if concurrent inserts happen
            // The DO UPDATE with a no-op ensures RETURNING gives us the existing row on conflict.
            // Branch on address_type because Diesel requires static ON CONFLICT target columns.
            let mut model = match address_type {
                AddressType::Icao => diesel::insert_into(aircraft::table)
                    .values(&new_aircraft)
                    .on_conflict(aircraft::icao_address)
                    .do_update()
                    .set(aircraft::icao_address.eq(excluded(aircraft::icao_address)))
                    .returning(AircraftModel::as_returning())
                    .get_result(&mut conn),
                AddressType::Flarm => diesel::insert_into(aircraft::table)
                    .values(&new_aircraft)
                    .on_conflict(aircraft::flarm_address)
                    .do_update()
                    .set(aircraft::flarm_address.eq(excluded(aircraft::flarm_address)))
                    .returning(AircraftModel::as_returning())
                    .get_result(&mut conn),
                AddressType::Ogn => diesel::insert_into(aircraft::table)
                    .values(&new_aircraft)
                    .on_conflict(aircraft::ogn_address)
                    .do_update()
                    .set(aircraft::ogn_address.eq(excluded(aircraft::ogn_address)))
                    .returning(AircraftModel::as_returning())
                    .get_result(&mut conn),
                AddressType::Unknown => diesel::insert_into(aircraft::table)
                    .values(&new_aircraft)
                    .on_conflict(aircraft::other_address)
                    .do_update()
                    .set(aircraft::other_address.eq(excluded(aircraft::other_address)))
                    .returning(AircraftModel::as_returning())
                    .get_result(&mut conn),
            }
            .map_err(|e| {
                error!(
                    "get_or_insert_aircraft_by_address upsert failed: address={:06X}, \
                     address_type={:?}, computed_registration={:?}, error={}",
                    address, address_type, registration, e
                );
                e
            })?;

            // If we have a computed registration and the aircraft doesn't already have one,
            // try to set it — but only if no other aircraft already owns that registration.
            if !registration.is_empty() && model.registration.as_deref().unwrap_or("").is_empty() {
                let duplicate_exists = diesel::dsl::select(diesel::dsl::exists(
                    aircraft::table
                        .filter(aircraft::registration.eq(&registration))
                        .filter(aircraft::id.ne(model.id)),
                ))
                .get_result::<bool>(&mut conn)?;

                if duplicate_exists {
                    // Store the conflicting registration for async merge
                    if model.pending_registration.as_deref() != Some(&registration) {
                        diesel::update(aircraft::table.filter(aircraft::id.eq(model.id)))
                            .set(aircraft::pending_registration.eq(&registration))
                            .execute(&mut conn)?;
                        model.pending_registration = Some(registration.clone());
                    }
                    warn!(
                        "Duplicate registration: address={:06X}, address_type={:?}, \
                         computed_registration={:?}, kept_registration={:?} \
                         (set pending_registration for async merge)",
                        address,
                        address_type,
                        registration,
                        model.registration.as_deref().unwrap_or(""),
                    );
                } else {
                    diesel::update(aircraft::table.filter(aircraft::id.eq(model.id)))
                        .set(aircraft::registration.eq(&registration))
                        .execute(&mut conn)?;
                    model.registration = Some(registration);
                }
            }

            Ok::<AircraftModel, anyhow::Error>(model)
        })
        .await?
    }

    /// Get or insert an aircraft for fix processing
    /// This method is optimized for the high-frequency fix processing path:
    /// - If aircraft doesn't exist, creates it with all available fields from the packet
    /// - If aircraft exists, atomically updates APRS-specific metadata fields:
    ///   - aircraft_category, icao_model_code, adsb_emitter_category, tracker_device_type, registration
    /// - Always returns the aircraft in one atomic operation
    ///
    /// Note: latitude, longitude, and last_fix_at are updated in fixes_repo.insert()
    /// which is the common endpoint for all data sources (APRS, Beast, SBS).
    #[tracing::instrument(skip(self, packet_fields), fields(%address, ?address_type))]
    pub async fn aircraft_for_fix(
        &self,
        address: i32,
        address_type: AddressType,
        packet_fields: AircraftPacketFields,
    ) -> Result<AircraftModel> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Extract country code from ICAO address if applicable
            let country_code =
                Aircraft::extract_country_code_from_icao(address as u32, address_type);

            // Extract tail number from ICAO address if it's a US aircraft
            // Use packet registration if available and non-empty, otherwise try to extract from ICAO
            let registration = packet_fields
                .registration
                .as_ref()
                .filter(|r| !r.is_empty())
                .cloned()
                .or_else(|| Aircraft::extract_tail_number_from_icao(address as u32, address_type))
                .unwrap_or_default();

            // Route address to the correct typed column
            let (icao_address, flarm_address, ogn_address, other_address) = match address_type {
                AddressType::Icao => (Some(address), None, None, None),
                AddressType::Flarm => (None, Some(address), None, None),
                AddressType::Ogn => (None, None, Some(address), None),
                AddressType::Unknown => (None, None, None, Some(address)),
            };

            // If we have a registration, try to merge into an existing aircraft that
            // already has this registration but is missing our address type.
            // This handles the case where an aircraft was created from OGN DDB (with
            // flarm_address) and later appears on an ADSB receiver (with icao_address).
            if !registration.is_empty()
                && let Some(model) =
                    Self::merge_by_registration(&registration, address, address_type, &mut conn)?
            {
                return Ok(model);
            }

            let new_aircraft = NewAircraft {
                icao_address,
                flarm_address,
                ogn_address,
                other_address,
                aircraft_model: packet_fields.aircraft_model.clone().unwrap_or_default(),
                // Always insert with NULL registration to avoid violating the
                // idx_aircraft_registration_unique constraint. The DO UPDATE clause
                // will safely set the registration (with a duplicate check) on the
                // next fix for this aircraft.
                registration: None,
                competition_number: String::new(),
                tracked: true,
                identified: true,
                from_ogn_ddb: false,
                from_adsbx_ddb: false,
                frequency_mhz: None,
                pilot_name: None,
                home_base_airport_ident: None,
                last_fix_at: None, // Updated in fixes_repo.insert()
                club_id: None,
                icao_model_code: packet_fields.icao_model_code.clone(),
                adsb_emitter_category: packet_fields.adsb_emitter_category,
                tracker_device_type: packet_fields.tracker_device_type.clone(),
                country_code: country_code.clone(),
                latitude: None,
                longitude: None,
                owner_operator: None,
                aircraft_category: packet_fields.aircraft_category,
                engine_count: None,
                engine_type: None,
                faa_pia: None,
                faa_ladd: None,
                year: None,
                is_military: None,
                current_fix: None, // Will be populated when fix is inserted
                images: None,
                pending_registration: None,
            };

            // Use INSERT ... ON CONFLICT ... DO UPDATE RETURNING to atomically handle race conditions
            // On conflict, update all packet-derived fields atomically in one operation
            // This eliminates the need for separate async update tasks

            // Prepare registration SQL expression
            // Only set registration if no OTHER aircraft already has this registration.
            // This prevents unique constraint violations from bad packet data (e.g., ADS-B
            // packets reporting the wrong registration for an aircraft).
            let registration_sql = if !registration.is_empty() {
                let escaped = registration.replace('\'', "''");
                format!(
                    "CASE WHEN NOT EXISTS (\
                        SELECT 1 FROM aircraft a2 \
                        WHERE a2.registration = '{escaped}' \
                        AND a2.id != aircraft.id\
                    ) THEN '{escaped}'::text \
                    ELSE aircraft.registration END"
                )
            } else {
                "aircraft.registration".to_string()
            };

            // Note: latitude, longitude, and last_fix_at are updated in fixes_repo.insert()
            // which is called at the end of the pipeline for all data sources (APRS, Beast, SBS).
            // This function only updates APRS-specific metadata fields.
            //
            // Common DO UPDATE set for all address type branches
            macro_rules! fix_upsert_set {
                () => {(
                    aircraft::aircraft_category.eq(packet_fields.aircraft_category),
                    // Only update icao_model_code if current value is NULL (preserve data from authoritative sources)
                    aircraft::icao_model_code.eq(diesel::dsl::sql("COALESCE(aircraft.icao_model_code, excluded.icao_model_code)")),
                    // Only update adsb_emitter_category if current value is NULL (preserve data from authoritative sources)
                    aircraft::adsb_emitter_category.eq(diesel::dsl::sql("COALESCE(aircraft.adsb_emitter_category, excluded.adsb_emitter_category)")),
                    aircraft::tracker_device_type.eq(&packet_fields.tracker_device_type),
                    // Only update aircraft_model if current value is NULL or empty string
                    aircraft::aircraft_model.eq(diesel::dsl::sql(
                        "CASE WHEN (aircraft.aircraft_model IS NULL OR aircraft.aircraft_model = '') \
                         THEN excluded.aircraft_model \
                         ELSE aircraft.aircraft_model END"
                    )),
                    aircraft::registration.eq(diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Text>>(&registration_sql)),
                    aircraft::country_code.eq(&country_code),
                )}
            }

            // Branch on address_type because Diesel requires static ON CONFLICT target columns
            let mut model = match address_type {
                AddressType::Icao => diesel::insert_into(aircraft::table)
                    .values(&new_aircraft)
                    .on_conflict(aircraft::icao_address)
                    .do_update()
                    .set(fix_upsert_set!())
                    .returning(AircraftModel::as_returning())
                    .get_result(&mut conn),
                AddressType::Flarm => diesel::insert_into(aircraft::table)
                    .values(&new_aircraft)
                    .on_conflict(aircraft::flarm_address)
                    .do_update()
                    .set(fix_upsert_set!())
                    .returning(AircraftModel::as_returning())
                    .get_result(&mut conn),
                AddressType::Ogn => diesel::insert_into(aircraft::table)
                    .values(&new_aircraft)
                    .on_conflict(aircraft::ogn_address)
                    .do_update()
                    .set(fix_upsert_set!())
                    .returning(AircraftModel::as_returning())
                    .get_result(&mut conn),
                AddressType::Unknown => diesel::insert_into(aircraft::table)
                    .values(&new_aircraft)
                    .on_conflict(aircraft::other_address)
                    .do_update()
                    .set(fix_upsert_set!())
                    .returning(AircraftModel::as_returning())
                    .get_result(&mut conn),
            }
            .map_err(|e| {
                error!(
                    "aircraft_for_fix upsert failed: address={:06X}, address_type={:?}, \
                     packet_registration={:?}, computed_registration={:?}, error={}",
                    address, address_type, packet_fields.registration, registration, e
                );
                e
            })?;

            // If the registration from the packet was not applied because another aircraft
            // already has it, store it as pending_registration for async merge
            if !registration.is_empty() {
                let model_reg = model.registration.as_deref().unwrap_or("");
                if model_reg != registration {
                    // Store the conflicting registration so a background task can merge later
                    if model.pending_registration.as_deref() != Some(&registration) {
                        diesel::update(aircraft::table.filter(aircraft::id.eq(model.id)))
                            .set(aircraft::pending_registration.eq(&registration))
                            .execute(&mut conn)?;
                        model.pending_registration = Some(registration.clone());
                    }
                    warn!(
                        "Duplicate registration: address={:06X}, address_type={:?}, \
                         packet_registration={:?}, kept_registration={:?} \
                         (set pending_registration for async merge)",
                        address, address_type, registration, model_reg
                    );
                }
            }

            Ok::<AircraftModel, anyhow::Error>(model)
        })
        .await?
    }

    /// Try to merge a new address into an existing aircraft that already has the given registration.
    ///
    /// When an aircraft appears via a different address type (e.g., ICAO) than the one it was
    /// originally created with (e.g., FLARM from OGN DDB), this method adds the new address
    /// to the existing record instead of creating a duplicate.
    ///
    /// If another aircraft already holds the incoming address (a pre-existing duplicate),
    /// this method returns `None` and the caller should fall through to the normal
    /// INSERT...ON CONFLICT path. A background task will be spawned to clean up the duplicate
    /// (reassigning fixes/flights is too expensive for the hot path).
    ///
    /// Returns `Some(model)` if merge succeeded, `None` if no matching aircraft found,
    /// the target already has the address type, or a conflicting aircraft holds the address.
    fn merge_by_registration(
        registration: &str,
        address: i32,
        address_type: AddressType,
        conn: &mut PgConnection,
    ) -> Result<Option<AircraftModel>> {
        // First, find the aircraft that owns this registration
        let target = aircraft::table
            .filter(aircraft::registration.eq(registration))
            .select(AircraftModel::as_select())
            .first(conn)
            .optional()?;

        let target = match target {
            Some(t) => t,
            None => return Ok(None),
        };

        // Check if the target already has the incoming address type populated
        let target_has_address = match address_type {
            AddressType::Icao => target.icao_address.is_some(),
            AddressType::Flarm => target.flarm_address.is_some(),
            AddressType::Ogn => target.ogn_address.is_some(),
            AddressType::Unknown => target.other_address.is_some(),
        };

        if target_has_address {
            return Ok(None);
        }

        // Check if another aircraft currently holds the incoming address
        // (a pre-existing duplicate created before this merge logic existed)
        let duplicate_exists = match address_type {
            AddressType::Icao => aircraft::table
                .filter(aircraft::icao_address.eq(address))
                .filter(aircraft::id.ne(target.id))
                .select(aircraft::id)
                .first::<Uuid>(conn)
                .optional()?,
            AddressType::Flarm => aircraft::table
                .filter(aircraft::flarm_address.eq(address))
                .filter(aircraft::id.ne(target.id))
                .select(aircraft::id)
                .first::<Uuid>(conn)
                .optional()?,
            AddressType::Ogn => aircraft::table
                .filter(aircraft::ogn_address.eq(address))
                .filter(aircraft::id.ne(target.id))
                .select(aircraft::id)
                .first::<Uuid>(conn)
                .optional()?,
            AddressType::Unknown => aircraft::table
                .filter(aircraft::other_address.eq(address))
                .filter(aircraft::id.ne(target.id))
                .select(aircraft::id)
                .first::<Uuid>(conn)
                .optional()?,
        };

        if duplicate_exists.is_some() {
            // Another aircraft holds this address — can't merge synchronously because
            // reassigning fixes across hypertable chunks is too expensive for the hot path.
            // Fall through to normal INSERT...ON CONFLICT and let the duplicate persist
            // until cleaned up separately.
            warn!(
                "Cannot merge address {:06X} ({:?}) into aircraft {} (registration={}): \
                 another aircraft already holds this address. Duplicate cleanup required.",
                address as u32, address_type, target.id, registration
            );
            return Ok(None);
        }

        // No conflict detected in check above — try to set the address on the target aircraft.
        // However, due to race conditions, another thread may have inserted an aircraft with this
        // address between our check and this UPDATE. If that happens, the UPDATE will fail with
        // a unique constraint violation. We catch that error and return None, which causes the
        // caller to fall through to the safe INSERT...ON CONFLICT path.
        let model = match address_type {
            AddressType::Icao => diesel::update(aircraft::table.filter(aircraft::id.eq(target.id)))
                .set(aircraft::icao_address.eq(address))
                .returning(AircraftModel::as_returning())
                .get_result(conn),
            AddressType::Flarm => {
                diesel::update(aircraft::table.filter(aircraft::id.eq(target.id)))
                    .set(aircraft::flarm_address.eq(address))
                    .returning(AircraftModel::as_returning())
                    .get_result(conn)
            }
            AddressType::Ogn => diesel::update(aircraft::table.filter(aircraft::id.eq(target.id)))
                .set(aircraft::ogn_address.eq(address))
                .returning(AircraftModel::as_returning())
                .get_result(conn),
            AddressType::Unknown => {
                diesel::update(aircraft::table.filter(aircraft::id.eq(target.id)))
                    .set(aircraft::other_address.eq(address))
                    .returning(AircraftModel::as_returning())
                    .get_result(conn)
            }
        };

        // Handle unique constraint violation from race condition
        let model = match model {
            Ok(m) => m,
            Err(e) => {
                // Check if this is a unique constraint violation using structured error matching
                if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) = e {
                    warn!(
                        "Race condition in merge_by_registration: address {:06X} ({:?}) was \
                         inserted by another thread before UPDATE completed. Falling back to \
                         INSERT...ON CONFLICT path.",
                        address as u32, address_type
                    );
                    return Ok(None);
                }
                // Other errors are unexpected, propagate them with context
                return Err(e).with_context(|| {
                    format!(
                        "Failed to update aircraft {} with address {:06X} ({:?})",
                        target.id, address as u32, address_type
                    )
                });
            }
        };

        info!(
            "Merged address {:06X} ({:?}) into existing aircraft {} (registration={})",
            address as u32, address_type, model.id, registration
        );

        Ok(Some(model))
    }

    /// Merge duplicate aircraft that have a pending_registration set.
    ///
    /// For each aircraft with `pending_registration IS NOT NULL`:
    /// 1. Find the "target" aircraft that owns the registration
    /// 2. Determine which address type(s) the duplicate has that the target lacks
    /// 3. Reassign fixes and flights from the duplicate to the target
    /// 4. Copy the address(es) to the target
    /// 5. Delete the duplicate
    /// 6. Clear pending_registration
    ///
    /// Returns detailed merge statistics.
    pub async fn merge_pending_registrations(&self) -> Result<MergeStats> {
        use crate::schema::fixes;
        use crate::schema::flights;
        use crate::schema::spurious_flights;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            use diesel::Connection;

            let mut conn = pool
                .get()
                .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

            // Find all aircraft with a pending registration
            let duplicates: Vec<AircraftModel> = aircraft::table
                .filter(aircraft::pending_registration.is_not_null())
                .select(AircraftModel::as_select())
                .load(&mut conn)?;

            let mut stats = MergeStats {
                duplicates_found: duplicates.len(),
                ..MergeStats::default()
            };

            if stats.duplicates_found == 0 {
                return Ok(stats);
            }

            info!(
                "Found {} aircraft with pending registrations to merge",
                stats.duplicates_found
            );

            for dup in duplicates {
                let pending_reg = match dup.pending_registration.as_deref() {
                    Some(r) => r.to_string(),
                    None => continue,
                };

                // Each merge is wrapped in a transaction so partial merges don't occur,
                // and errors on one aircraft don't abort the entire batch.
                // Returns None for "claimed" (no target found), Some(...) for merged.
                let result: Result<Option<(usize, usize)>, anyhow::Error> =
                    conn.transaction(|conn| {
                        // Find the target aircraft that owns this registration
                        let target = match aircraft::table
                            .filter(aircraft::registration.eq(&pending_reg))
                            .filter(aircraft::id.ne(dup.id))
                            .select(AircraftModel::as_select())
                            .first(conn)
                            .optional()?
                        {
                            Some(t) => t,
                            None => {
                                // No aircraft owns this registration anymore — clear pending
                                // and set the registration directly on this aircraft
                                diesel::update(
                                    aircraft::table.filter(aircraft::id.eq(dup.id)),
                                )
                                .set((
                                    aircraft::registration.eq(&pending_reg),
                                    aircraft::pending_registration.eq(None::<String>),
                                ))
                                .execute(conn)?;
                                info!(
                                    "No owner for registration {:?}, assigned directly to aircraft {}",
                                    pending_reg, dup.id
                                );
                                return Ok(None);
                            }
                        };

                        // Reassign fixes from the duplicate to the target
                        let fixes_updated = diesel::update(
                            fixes::table.filter(fixes::aircraft_id.eq(dup.id)),
                        )
                        .set(fixes::aircraft_id.eq(target.id))
                        .execute(conn)
                        .context("reassigning fixes")?;

                        // Reassign flights from the duplicate to the target
                        let flights_updated = diesel::update(
                            flights::table.filter(flights::aircraft_id.eq(dup.id)),
                        )
                        .set(flights::aircraft_id.eq(target.id))
                        .execute(conn)
                        .context("reassigning flights")?;

                        // Reassign spurious flights from the duplicate to the target
                        diesel::update(
                            spurious_flights::table
                                .filter(spurious_flights::aircraft_id.eq(dup.id)),
                        )
                        .set(spurious_flights::aircraft_id.eq(target.id))
                        .execute(conn)
                        .context("reassigning spurious flights")?;

                        // Delete the duplicate first, then copy its addresses to the target.
                        // This avoids two problems:
                        // 1. Clearing an address on the dup could violate chk_at_least_one_address
                        //    when the address being transferred is the dup's only address.
                        // 2. Setting an address on the target while the dup still holds it would
                        //    violate the unique index.
                        // Deleting the dup removes it from unique indexes, and cascade handles
                        // geofences, watchlist, etc.
                        let deleted = diesel::delete(
                            aircraft::table.filter(aircraft::id.eq(dup.id)),
                        )
                        .execute(conn)
                        .context("deleting duplicate aircraft")?;
                        anyhow::ensure!(
                            deleted == 1,
                            "expected to delete 1 duplicate aircraft row, but deleted {}",
                            deleted
                        );

                        // Copy addresses from the duplicate to the target in a single
                        // UPDATE. Safe now that the duplicate row is gone.
                        let new_icao = if dup.icao_address.is_some() && target.icao_address.is_none() {
                            dup.icao_address
                        } else {
                            target.icao_address
                        };
                        let new_flarm = if dup.flarm_address.is_some() && target.flarm_address.is_none() {
                            dup.flarm_address
                        } else {
                            target.flarm_address
                        };
                        let new_ogn = if dup.ogn_address.is_some() && target.ogn_address.is_none() {
                            dup.ogn_address
                        } else {
                            target.ogn_address
                        };
                        let new_other = if dup.other_address.is_some() && target.other_address.is_none() {
                            dup.other_address
                        } else {
                            target.other_address
                        };
                        diesel::update(
                            aircraft::table.filter(aircraft::id.eq(target.id)),
                        )
                        .set((
                            aircraft::icao_address.eq(new_icao),
                            aircraft::flarm_address.eq(new_flarm),
                            aircraft::ogn_address.eq(new_ogn),
                            aircraft::other_address.eq(new_other),
                        ))
                        .execute(conn)
                        .context("transferring addresses to target")?;

                        info!(
                            "Merged aircraft {} into {} (registration={}): \
                             reassigned {} fixes, {} flights",
                            dup.id, target.id, pending_reg, fixes_updated, flights_updated
                        );

                        Ok(Some((fixes_updated, flights_updated)))
                    });

                match result {
                    Ok(None) => {
                        stats.registrations_claimed += 1;
                    }
                    Ok(Some((fixes_updated, flights_updated))) => {
                        stats.fixes_reassigned += fixes_updated;
                        stats.flights_reassigned += flights_updated;
                        stats.aircraft_deleted += 1;
                        stats.aircraft_merged += 1;
                    }
                    Err(e) => {
                        error!(
                            "Failed to merge aircraft {} (pending_registration={:?}): {:#}",
                            dup.id, pending_reg, e
                        );
                        stats.errors.push(format!(
                            "aircraft {} (pending_registration={}): {:#}",
                            dup.id, pending_reg, e
                        ));
                    }
                }
            }

            info!(
                "Completed pending registration merge: {}/{} aircraft merged, \
                 {} fixes reassigned, {} flights reassigned, {} deleted, {} errors",
                stats.aircraft_merged,
                stats.duplicates_found,
                stats.fixes_reassigned,
                stats.flights_reassigned,
                stats.aircraft_deleted,
                stats.errors.len()
            );

            Ok(stats)
        })
        .await?
    }

    /// Get an aircraft by its ID
    pub async fn get_aircraft_by_id(&self, aircraft_id: Uuid) -> Result<Option<Aircraft>> {
        let mut conn = self.get_connection()?;
        let model = aircraft::table
            .filter(aircraft::id.eq(aircraft_id))
            .select(AircraftModel::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(model.map(|model| model.into()))
    }

    /// Search for all aircraft assigned to a specific club
    pub async fn search_by_club_id(&self, club_id: Uuid) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;

        let models = aircraft::table
            .filter(aircraft::club_id.eq(club_id))
            .order_by(aircraft::registration)
            .select(AircraftModel::as_select())
            .load(&mut conn)?;

        Ok(models.into_iter().map(|model| model.into()).collect())
    }

    /// Search aircraft by address across all typed address columns.
    /// Returns the first match (an address value should only exist in one column).
    pub async fn search_by_address(&self, address: u32) -> Result<Option<Aircraft>> {
        let mut conn = self.get_connection()?;
        let addr = address as i32;
        let model = aircraft::table
            .filter(
                aircraft::icao_address
                    .eq(addr)
                    .or(aircraft::flarm_address.eq(addr))
                    .or(aircraft::ogn_address.eq(addr))
                    .or(aircraft::other_address.eq(addr)),
            )
            .select(AircraftModel::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(model.map(|model| model.into()))
    }

    /// Search aircraft by registration
    pub async fn search_by_registration(&self, registration: &str) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;
        let search_pattern = format!("%{}%", registration);
        let models = aircraft::table
            .filter(aircraft::registration.ilike(&search_pattern))
            .select(AircraftModel::as_select())
            .load(&mut conn)?;

        Ok(models.into_iter().map(|model| model.into()).collect())
    }

    /// Get recent aircraft with a limit, ordered by last_fix_at (most recently heard from)
    /// Optionally filter by aircraft types
    pub async fn get_recent_aircraft(
        &self,
        limit: i64,
        aircraft_types: Option<Vec<String>>,
    ) -> Result<Vec<Aircraft>> {
        use diesel::ExpressionMethods;

        let mut conn = self.get_connection()?;

        let mut query = aircraft::table
            .filter(aircraft::last_fix_at.is_not_null())
            .into_boxed();

        // Apply aircraft category filter if provided
        if let Some(types) = aircraft_types
            && !types.is_empty()
        {
            // Convert string aircraft types to AircraftCategory enum values
            let aircraft_category_enums: Vec<AircraftCategory> = types
                .iter()
                .filter_map(|t| match t.as_str() {
                    "glider" => Some(AircraftCategory::Glider),
                    "tow_tug" => Some(AircraftCategory::TowTug),
                    "gyroplane" => Some(AircraftCategory::Gyroplane),
                    "skydiver_parachute" => Some(AircraftCategory::SkydiverParachute),
                    "hang_glider" => Some(AircraftCategory::HangGlider),
                    "paraglider" => Some(AircraftCategory::Paraglider),
                    "landplane" => Some(AircraftCategory::Landplane),
                    "unknown" => Some(AircraftCategory::Unknown),
                    "balloon" => Some(AircraftCategory::Balloon),
                    "airship" => Some(AircraftCategory::Airship),
                    "drone" => Some(AircraftCategory::Drone),
                    "static_obstacle" => Some(AircraftCategory::StaticObstacle),
                    "helicopter" => Some(AircraftCategory::Helicopter),
                    "amphibian" => Some(AircraftCategory::Amphibian),
                    "powered_parachute" => Some(AircraftCategory::PoweredParachute),
                    "rotorcraft" => Some(AircraftCategory::Rotorcraft),
                    "seaplane" => Some(AircraftCategory::Seaplane),
                    "tiltrotor" => Some(AircraftCategory::Tiltrotor),
                    "vtol" => Some(AircraftCategory::Vtol),
                    "electric" => Some(AircraftCategory::Electric),
                    _ => None,
                })
                .collect();

            if !aircraft_category_enums.is_empty() {
                query = query.filter(aircraft::aircraft_category.eq_any(aircraft_category_enums));
            }
        }

        let models = query
            .order(aircraft::last_fix_at.desc())
            .limit(limit)
            .select(AircraftModel::as_select())
            .load(&mut conn)?;

        Ok(models.into_iter().map(|model| model.into()).collect())
    }

    /// Find all aircraft within a bounding box that have recent fixes
    /// Returns aircraft models with all fields populated from the database
    pub async fn find_aircraft_in_bounding_box(
        &self,
        north: f64,
        west: f64,
        south: f64,
        east: f64,
        cutoff_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<AircraftModel>> {
        use diesel::sql_types::{BigInt, Double, Timestamptz};

        let mut conn = self.get_connection()?;

        // Build the SQL query with optional LIMIT clause
        let limit_clause = if limit.is_some() { "LIMIT $6" } else { "" };

        let aircraft_sql = format!(
            r#"
            WITH params AS (
                SELECT
                    $1::double precision AS left_lng,
                    $2::double precision AS bottom_lat,
                    $3::double precision AS right_lng,
                    $4::double precision AS top_lat,
                    $5::timestamptz AS cutoff_time
            ),
            parts AS (
                SELECT
                    CASE WHEN left_lng <= right_lng THEN
                        ARRAY[
                            ST_MakeEnvelope(left_lng, bottom_lat, right_lng, top_lat, 4326)::geometry
                        ]
                    ELSE
                        ARRAY[
                            ST_MakeEnvelope(left_lng, bottom_lat, 180, top_lat, 4326)::geometry,
                            ST_MakeEnvelope(-180, bottom_lat, right_lng, top_lat, 4326)::geometry
                        ]
                    END AS boxes,
                    cutoff_time
                FROM params
            )
            SELECT d.*
            FROM aircraft d, parts
            WHERE d.last_fix_at >= parts.cutoff_time
              AND d.location_geom IS NOT NULL
              AND (
                  d.location_geom && parts.boxes[1]
                  OR (array_length(parts.boxes, 1) = 2 AND d.location_geom && parts.boxes[2])
              )
            {}
        "#,
            limit_clause
        );

        let query = diesel::sql_query(aircraft_sql)
            .bind::<Double, _>(west)
            .bind::<Double, _>(south)
            .bind::<Double, _>(east)
            .bind::<Double, _>(north)
            .bind::<Timestamptz, _>(cutoff_time);

        // Bind limit if provided
        let aircraft_models = if let Some(lim) = limit {
            query
                .bind::<BigInt, _>(lim)
                .load::<AircraftModel>(&mut conn)?
        } else {
            query.load::<AircraftModel>(&mut conn)?
        };

        Ok(aircraft_models)
    }

    /// Update the club assignment for an aircraft
    pub async fn update_club_id(&self, aircraft_id: Uuid, club_id: Option<Uuid>) -> Result<bool> {
        let mut conn = self.get_connection()?;

        let rows_updated = diesel::update(aircraft::table.filter(aircraft::id.eq(aircraft_id)))
            .set(aircraft::club_id.eq(club_id))
            .execute(&mut conn)?;

        if rows_updated > 0 {
            info!(
                "Updated aircraft {} club assignment to {:?}",
                aircraft_id, club_id
            );
            Ok(true)
        } else {
            warn!("No aircraft found with ID {}", aircraft_id);
            Ok(false)
        }
    }

    /// Get aircraft by ID (returns AircraftModel with all fields including images)
    pub async fn get_aircraft_model_by_id(
        &self,
        aircraft_id: Uuid,
    ) -> Result<Option<AircraftModel>> {
        let mut conn = self.get_connection()?;

        let model = aircraft::table
            .filter(aircraft::id.eq(aircraft_id))
            .select(AircraftModel::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(model)
    }

    /// Update the images cache for an aircraft
    pub async fn update_images(
        &self,
        aircraft_id: Uuid,
        images_json: serde_json::Value,
    ) -> Result<bool> {
        let mut conn = self.get_connection()?;

        let rows_updated = diesel::update(aircraft::table.filter(aircraft::id.eq(aircraft_id)))
            .set(aircraft::images.eq(images_json))
            .execute(&mut conn)?;

        if rows_updated > 0 {
            info!("Updated aircraft {} images cache", aircraft_id);
            Ok(true)
        } else {
            warn!("No aircraft found with ID {}", aircraft_id);
            Ok(false)
        }
    }
}

/// In-memory aircraft cache to eliminate DB round trips on the fix processing hot path.
///
/// On startup, loads all aircraft with a fix in the last 7 days. On cache hit,
/// returns the cached Aircraft directly (no DB call). On miss, falls through to
/// the DB upsert and caches the result.
///
/// Keyed by both `(AddressType, address)` (for fix processing lookups) and
/// `Uuid` (for state transition lookups by aircraft_id).
#[derive(Clone)]
pub struct AircraftCache {
    by_address: Arc<DashMap<(AddressType, i32), Aircraft>>,
    by_id: Arc<DashMap<Uuid, Aircraft>>,
    repo: AircraftRepository,
}

impl AircraftCache {
    pub fn new(pool: PgPool) -> Self {
        Self {
            by_address: Arc::new(DashMap::new()),
            by_id: Arc::new(DashMap::new()),
            repo: AircraftRepository::new(pool),
        }
    }

    /// Preload all aircraft that have had a fix in the last 7 days.
    pub async fn preload(&self) -> Result<usize> {
        let start = std::time::Instant::now();
        let pool = self.repo.pool.clone();

        let models: Vec<AircraftModel> = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let cutoff = Utc::now() - chrono::Duration::days(7);
            aircraft::table
                .filter(aircraft::last_fix_at.gt(cutoff))
                .select(AircraftModel::as_select())
                .load(&mut conn)
                .map_err(|e| anyhow::anyhow!("Failed to preload aircraft cache: {}", e))
        })
        .await??;

        let count = models.len();
        for model in models {
            let a: Aircraft = model.into();
            let id = a.id.expect("aircraft from DB must have id");
            self.by_address
                .insert((a.address_type, a.address as i32), a.clone());
            self.by_id.insert(id, a);
        }

        let elapsed_ms = start.elapsed().as_millis();
        info!(
            "Aircraft cache preloaded: {} aircraft in {}ms",
            count, elapsed_ms
        );
        metrics::histogram!("aircraft_cache.preload_ms").record(elapsed_ms as f64);
        metrics::gauge!("aircraft_cache.size").set(count as f64);

        Ok(count)
    }

    /// Get or upsert an aircraft for fix processing.
    ///
    /// On cache hit: returns immediately, spawns async metadata DB update.
    /// On cache miss: falls through to DB upsert, caches result.
    pub async fn get_or_upsert(
        &self,
        address: i32,
        address_type: AddressType,
        packet_fields: AircraftPacketFields,
    ) -> Result<Aircraft> {
        let key = (address_type, address);

        // Fast path: cache hit
        if let Some(mut entry) = self.by_address.get_mut(&key) {
            metrics::counter!("aircraft_cache.hit_total", "lookup" => "by_address").increment(1);

            // Apply metadata updates to cached entry in-place
            let aircraft = entry.value_mut();
            let mut changed = false;

            if packet_fields.aircraft_category.is_some()
                && aircraft.aircraft_category != packet_fields.aircraft_category
            {
                aircraft.aircraft_category = packet_fields.aircraft_category;
                changed = true;
            }
            if packet_fields.tracker_device_type.is_some()
                && aircraft.tracker_device_type != packet_fields.tracker_device_type
            {
                aircraft.tracker_device_type = packet_fields.tracker_device_type.clone();
                changed = true;
            }
            if packet_fields.icao_model_code.is_some() && aircraft.icao_model_code.is_none() {
                aircraft.icao_model_code = packet_fields.icao_model_code.clone();
                changed = true;
            }
            if packet_fields.adsb_emitter_category.is_some()
                && aircraft.adsb_emitter_category.is_none()
            {
                aircraft.adsb_emitter_category = packet_fields.adsb_emitter_category;
                changed = true;
            }
            if packet_fields
                .aircraft_model
                .as_ref()
                .is_some_and(|m| !m.trim().is_empty())
                && aircraft.aircraft_model.trim().is_empty()
            {
                aircraft.aircraft_model = packet_fields.aircraft_model.clone().unwrap_or_default();
                changed = true;
            }
            // Registration: only update if we have one and aircraft doesn't
            // (the full duplicate-check logic runs on cache miss via DB upsert)
            if packet_fields
                .registration
                .as_ref()
                .is_some_and(|r| !r.is_empty())
                && aircraft.registration.is_none()
            {
                aircraft.registration = packet_fields.registration.clone();
                changed = true;
            }

            let result = aircraft.clone();

            // Update by_id map too
            if changed && let Some(id) = result.id {
                self.by_id.insert(id, result.clone());
            }

            // Drop the DashMap guard before spawning async work
            drop(entry);

            // Fire-and-forget async DB metadata update if anything changed.
            // Reuse the existing aircraft_for_fix() upsert which handles all the
            // complex SQL (registration duplicate checking, COALESCE logic, etc.)
            if changed {
                let repo = self.repo.clone();
                tokio::spawn(async move {
                    if let Err(e) = repo
                        .aircraft_for_fix(address, address_type, packet_fields)
                        .await
                    {
                        warn!("Failed to update aircraft metadata: {:#}", e);
                    }
                });
            }

            return Ok(result);
        }

        // Slow path: cache miss — fall through to DB upsert
        metrics::counter!("aircraft_cache.miss_total", "lookup" => "by_address").increment(1);

        let model = self
            .repo
            .aircraft_for_fix(address, address_type, packet_fields)
            .await?;
        let aircraft: Aircraft = model.into();
        let id = aircraft.id.expect("aircraft from DB must have id");

        self.by_address.insert(key, aircraft.clone());
        self.by_id.insert(id, aircraft.clone());

        self.update_size_gauge();

        Ok(aircraft)
    }

    /// Get an aircraft by ID from cache, falling back to DB on miss.
    pub async fn get_by_id(&self, aircraft_id: Uuid) -> Result<Option<Aircraft>> {
        if let Some(entry) = self.by_id.get(&aircraft_id) {
            metrics::counter!("aircraft_cache.hit_total", "lookup" => "by_id").increment(1);
            return Ok(Some(entry.value().clone()));
        }

        metrics::counter!("aircraft_cache.miss_total", "lookup" => "by_id").increment(1);

        let result = self.repo.get_aircraft_by_id(aircraft_id).await?;
        if let Some(ref aircraft) = result {
            let id = aircraft.id.expect("aircraft from DB must have id");
            self.by_id.insert(id, aircraft.clone());
            self.by_address.insert(
                (aircraft.address_type, aircraft.address as i32),
                aircraft.clone(),
            );
            self.update_size_gauge();
        }

        Ok(result)
    }

    fn update_size_gauge(&self) {
        metrics::gauge!("aircraft_cache.size").set(self.by_id.len() as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::r2d2::ConnectionManager;

    // Helper function to create a test database pool (for integration tests)
    fn create_test_pool() -> Result<PgPool> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/soar_test".to_string());

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder().build(manager)?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_aircraft_repository_creation() {
        // Just test that we can create the repository
        if let Ok(pool) = create_test_pool() {
            let _repo = AircraftRepository::new(pool);
        } else {
            // Skip test if we can't connect to test database
            println!("Skipping test - no test database connection");
        }
    }
}
