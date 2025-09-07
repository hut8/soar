use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::faa::aircraft_registrations::{Aircraft, ApprovedOps, AirworthinessClass};
use crate::locations_repo::LocationsRepository;

pub struct AircraftRegistrationsRepository {
    pool: PgPool,
    locations_repo: LocationsRepository,
}

impl AircraftRegistrationsRepository {
    pub fn new(pool: PgPool) -> Self {
        let locations_repo = LocationsRepository::new(pool.clone());
        Self { pool, locations_repo }
    }

    /// Find or create a club by name, using the aircraft registration data to populate address and location
    /// for new clubs. Returns the club's UUID.
    async fn find_or_create_club(
        &self,
        club_name: &str,
        aircraft_reg: &Aircraft,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Uuid> {
        // First try to find existing club
        let existing_club = sqlx::query!("SELECT id FROM clubs WHERE name = $1", club_name)
            .fetch_optional(&mut **transaction)
            .await?;

        if let Some(club) = existing_club {
            return Ok(club.id);
        }

        // Club doesn't exist, create it using the first aircraft's address and location data
        let new_club_id = Uuid::new_v4();

        // Determine if this is likely a soaring club based on the name
        let is_soaring = Some(club_name.to_uppercase().contains("SOAR") ||
                             club_name.to_uppercase().contains("GLIDING") ||
                             club_name.to_uppercase().contains("SAILPLANE") ||
                             club_name.to_uppercase().contains("GLIDER"));

        // Create or find location for this club using the aircraft's address
        let location_geolocation = aircraft_reg.registered_location.as_ref().map(|loc| {
            crate::locations::Point::new(loc.latitude, loc.longitude)
        });

        let location = self.locations_repo.find_or_create(
            aircraft_reg.street1.clone(),
            aircraft_reg.street2.clone(),
            aircraft_reg.city.clone(),
            aircraft_reg.state.clone(),
            aircraft_reg.zip_code.clone(),
            aircraft_reg.region_code.clone(),
            aircraft_reg.county_mail_code.clone(),
            aircraft_reg.country_mail_code.clone(),
            location_geolocation,
        ).await?;

        sqlx::query!(
            r#"
            INSERT INTO clubs (
                id, name, is_soaring, location_id,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            "#,
            new_club_id,
            club_name,
            is_soaring,
            location.id
        )
        .execute(&mut **transaction)
        .await?;

        info!("Created new club: {} with location {} from aircraft {}", club_name, location.id, aircraft_reg.n_number);
        Ok(new_club_id)
    }

    /// Upsert aircraft registrations into the database
    /// This will insert new aircraft registrations or update existing ones based on the primary key (registration_number)
    pub async fn upsert_aircraft_registrations<I>(&self, aircraft: I) -> Result<usize>
    where
        I: IntoIterator<Item = Aircraft>,
    {
        let aircraft_vec: Vec<Aircraft> = aircraft.into_iter().collect();
        let mut upserted_count = 0;
        let mut failed_count = 0;

        for aircraft_reg in aircraft_vec {
            // Process each aircraft in its own transaction to avoid transaction abort issues
            let mut transaction = self.pool.begin().await?;

            // Convert year_mfr from u16 to i32 for database storage
            let year_mfr = aircraft_reg.year_mfr.map(|y| y as i32);

            // Convert transponder_code from u32 to i64 for database storage (BIGINT)
            let transponder_code = aircraft_reg.transponder_code.map(|t| t as i64);

            // Create or find location for this aircraft
            let location_geolocation = aircraft_reg.registered_location.as_ref().map(|loc| {
                crate::locations::Point::new(loc.latitude, loc.longitude)
            });

            let location = self.locations_repo.find_or_create(
                aircraft_reg.street1.clone(),
                aircraft_reg.street2.clone(),
                aircraft_reg.city.clone(),
                aircraft_reg.state.clone(),
                aircraft_reg.zip_code.clone(),
                aircraft_reg.region_code.clone(),
                aircraft_reg.county_mail_code.clone(),
                aircraft_reg.country_mail_code.clone(),
                location_geolocation,
            ).await?;

            // Handle club linking if aircraft has a valid club name
            let club_id = if let Some(club_name) = aircraft_reg.club_name() {
                // Check if aircraft already has a club_id set (never overwrite existing club_id)
                let existing_club_id = sqlx::query!(
                    "SELECT club_id FROM aircraft_registrations WHERE registration_number = $1",
                    aircraft_reg.n_number
                )
                .fetch_optional(&mut *transaction)
                .await?;

                if let Some(existing) = existing_club_id {
                    if existing.club_id.is_some() {
                        // Aircraft already has a club_id, don't change it
                        existing.club_id
                    } else {
                        // Aircraft exists but has no club_id, set it
                        Some(
                            self.find_or_create_club(&club_name, &aircraft_reg, &mut transaction)
                                .await?,
                        )
                    }
                } else {
                    // New aircraft, set club_id
                    Some(
                        self.find_or_create_club(&club_name, &aircraft_reg, &mut transaction)
                            .await?,
                    )
                }
            } else {
                None
            };

            // Use ON CONFLICT to handle upserts
            let result = sqlx::query!(
                r#"
                INSERT INTO aircraft_registrations (
                    registration_number,
                    serial_number,
                    mfr_mdl_code,
                    eng_mfr_mdl_code,
                    year_mfr,
                    type_registration_code,
                    registrant_name,
                    location_id,
                    last_action_date,
                    certificate_issue_date,
                    airworthiness_class,
                    approved_operations_raw,
                    op_restricted_other,
                    op_restricted_ag_pest_control,
                    op_restricted_aerial_surveying,
                    op_restricted_aerial_advertising,
                    op_restricted_forest,
                    op_restricted_patrolling,
                    op_restricted_weather_control,
                    op_restricted_carriage_of_cargo,
                    op_experimental_show_compliance,
                    op_experimental_research_development,
                    op_experimental_amateur_built,
                    op_experimental_exhibition,
                    op_experimental_racing,
                    op_experimental_crew_training,
                    op_experimental_market_survey,
                    op_experimental_operating_kit_built,
                    op_experimental_light_sport_reg_prior_2008,
                    op_experimental_light_sport_operating_kit_built,
                    op_experimental_light_sport_prev_21_190,
                    op_experimental_uas_research_development,
                    op_experimental_uas_market_survey,
                    op_experimental_uas_crew_training,
                    op_experimental_uas_exhibition,
                    op_experimental_uas_compliance_with_cfr,
                    op_sfp_ferry_for_repairs_alterations_storage,
                    op_sfp_evacuate_impending_danger,
                    op_sfp_excess_of_max_certificated,
                    op_sfp_delivery_or_export,
                    op_sfp_production_flight_testing,
                    op_sfp_customer_demo,
                    type_aircraft_code,
                    type_engine_code,
                    status_code,
                    transponder_code,
                    fractional_owner,
                    airworthiness_date,
                    expiration_date,
                    unique_id,
                    kit_mfr_name,
                    kit_model_name,
                    club_id
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
                    $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36,
                    $37, $38, $39, $40, $41, $42, $43, $44, $45, $46, $47, $48, $49, $50, $51, $52, $53
                )
                ON CONFLICT (registration_number)
                DO UPDATE SET
                    serial_number = EXCLUDED.serial_number,
                    mfr_mdl_code = EXCLUDED.mfr_mdl_code,
                    eng_mfr_mdl_code = EXCLUDED.eng_mfr_mdl_code,
                    year_mfr = EXCLUDED.year_mfr,
                    type_registration_code = EXCLUDED.type_registration_code,
                    registrant_name = EXCLUDED.registrant_name,
                    location_id = EXCLUDED.location_id,
                    last_action_date = EXCLUDED.last_action_date,
                    certificate_issue_date = EXCLUDED.certificate_issue_date,
                    airworthiness_class = EXCLUDED.airworthiness_class,
                    approved_operations_raw = EXCLUDED.approved_operations_raw,
                    op_restricted_other = EXCLUDED.op_restricted_other,
                    op_restricted_ag_pest_control = EXCLUDED.op_restricted_ag_pest_control,
                    op_restricted_aerial_surveying = EXCLUDED.op_restricted_aerial_surveying,
                    op_restricted_aerial_advertising = EXCLUDED.op_restricted_aerial_advertising,
                    op_restricted_forest = EXCLUDED.op_restricted_forest,
                    op_restricted_patrolling = EXCLUDED.op_restricted_patrolling,
                    op_restricted_weather_control = EXCLUDED.op_restricted_weather_control,
                    op_restricted_carriage_of_cargo = EXCLUDED.op_restricted_carriage_of_cargo,
                    op_experimental_show_compliance = EXCLUDED.op_experimental_show_compliance,
                    op_experimental_research_development = EXCLUDED.op_experimental_research_development,
                    op_experimental_amateur_built = EXCLUDED.op_experimental_amateur_built,
                    op_experimental_exhibition = EXCLUDED.op_experimental_exhibition,
                    op_experimental_racing = EXCLUDED.op_experimental_racing,
                    op_experimental_crew_training = EXCLUDED.op_experimental_crew_training,
                    op_experimental_market_survey = EXCLUDED.op_experimental_market_survey,
                    op_experimental_operating_kit_built = EXCLUDED.op_experimental_operating_kit_built,
                    op_experimental_light_sport_reg_prior_2008 = EXCLUDED.op_experimental_light_sport_reg_prior_2008,
                    op_experimental_light_sport_operating_kit_built = EXCLUDED.op_experimental_light_sport_operating_kit_built,
                    op_experimental_light_sport_prev_21_190 = EXCLUDED.op_experimental_light_sport_prev_21_190,
                    op_experimental_uas_research_development = EXCLUDED.op_experimental_uas_research_development,
                    op_experimental_uas_market_survey = EXCLUDED.op_experimental_uas_market_survey,
                    op_experimental_uas_crew_training = EXCLUDED.op_experimental_uas_crew_training,
                    op_experimental_uas_exhibition = EXCLUDED.op_experimental_uas_exhibition,
                    op_experimental_uas_compliance_with_cfr = EXCLUDED.op_experimental_uas_compliance_with_cfr,
                    op_sfp_ferry_for_repairs_alterations_storage = EXCLUDED.op_sfp_ferry_for_repairs_alterations_storage,
                    op_sfp_evacuate_impending_danger = EXCLUDED.op_sfp_evacuate_impending_danger,
                    op_sfp_excess_of_max_certificated = EXCLUDED.op_sfp_excess_of_max_certificated,
                    op_sfp_delivery_or_export = EXCLUDED.op_sfp_delivery_or_export,
                    op_sfp_production_flight_testing = EXCLUDED.op_sfp_production_flight_testing,
                    op_sfp_customer_demo = EXCLUDED.op_sfp_customer_demo,
                    type_aircraft_code = EXCLUDED.type_aircraft_code,
                    type_engine_code = EXCLUDED.type_engine_code,
                    status_code = EXCLUDED.status_code,
                    transponder_code = EXCLUDED.transponder_code,
                    fractional_owner = EXCLUDED.fractional_owner,
                    airworthiness_date = EXCLUDED.airworthiness_date,
                    expiration_date = EXCLUDED.expiration_date,
                    unique_id = EXCLUDED.unique_id,
                    kit_mfr_name = EXCLUDED.kit_mfr_name,
                    kit_model_name = EXCLUDED.kit_model_name,
                    club_id = CASE WHEN aircraft_registrations.club_id IS NOT NULL THEN aircraft_registrations.club_id ELSE EXCLUDED.club_id END
                "#,
                aircraft_reg.n_number,
                aircraft_reg.serial_number,
                aircraft_reg.mfr_mdl_code,
                aircraft_reg.eng_mfr_mdl_code,
                year_mfr,
                aircraft_reg.type_registration_code,
                aircraft_reg.registrant_name,
                location.id,
                aircraft_reg.last_action_date,
                aircraft_reg.certificate_issue_date,
                aircraft_reg.airworthiness_class as _,
                aircraft_reg.approved_operations_raw,
                aircraft_reg.approved_ops.restricted_other,
                aircraft_reg.approved_ops.restricted_ag_pest_control,
                aircraft_reg.approved_ops.restricted_aerial_surveying,
                aircraft_reg.approved_ops.restricted_aerial_advertising,
                aircraft_reg.approved_ops.restricted_forest,
                aircraft_reg.approved_ops.restricted_patrolling,
                aircraft_reg.approved_ops.restricted_weather_control,
                aircraft_reg.approved_ops.restricted_carriage_of_cargo,
                aircraft_reg.approved_ops.exp_show_compliance,
                aircraft_reg.approved_ops.exp_research_development,
                aircraft_reg.approved_ops.exp_amateur_built,
                aircraft_reg.approved_ops.exp_exhibition,
                aircraft_reg.approved_ops.exp_racing,
                aircraft_reg.approved_ops.exp_crew_training,
                aircraft_reg.approved_ops.exp_market_survey,
                aircraft_reg.approved_ops.exp_operating_kit_built,
                aircraft_reg.approved_ops.exp_lsa_reg_prior_2008,
                aircraft_reg.approved_ops.exp_lsa_operating_kit_built,
                aircraft_reg.approved_ops.exp_lsa_prev_21_190,
                aircraft_reg.approved_ops.exp_uas_research_development,
                aircraft_reg.approved_ops.exp_uas_market_survey,
                aircraft_reg.approved_ops.exp_uas_crew_training,
                aircraft_reg.approved_ops.exp_uas_exhibition,
                aircraft_reg.approved_ops.exp_uas_compliance_with_cfr,
                aircraft_reg.approved_ops.sfp_ferry_for_repairs_alterations_storage,
                aircraft_reg.approved_ops.sfp_evacuate_impending_danger,
                aircraft_reg.approved_ops.sfp_excess_of_max_certificated,
                aircraft_reg.approved_ops.sfp_delivery_or_export,
                aircraft_reg.approved_ops.sfp_production_flight_testing,
                aircraft_reg.approved_ops.sfp_customer_demo,
                aircraft_reg.type_aircraft_code,
                aircraft_reg.type_engine_code,
                aircraft_reg.status_code,
                transponder_code,
                aircraft_reg.fractional_owner,
                aircraft_reg.airworthiness_date,
                aircraft_reg.expiration_date,
                aircraft_reg.unique_id,
                aircraft_reg.kit_mfr_name,
                aircraft_reg.kit_model_name,
                club_id
            )
            .execute(&mut *transaction)
            .await;

            match result {
                Ok(_) => {
                    // Handle other names if present
                    if !aircraft_reg.other_names.is_empty() {
                        // First, delete existing other names for this aircraft
                        let delete_result = sqlx::query!(
                            "DELETE FROM aircraft_other_names WHERE registration_number = $1",
                            aircraft_reg.n_number
                        )
                        .execute(&mut *transaction)
                        .await;

                        if let Err(e) = delete_result {
                            warn!(
                                "Failed to delete other names for aircraft registration {}: {}",
                                aircraft_reg.n_number, e
                            );
                            transaction.rollback().await?;
                            failed_count += 1;
                            continue;
                        }

                        // Insert new other names
                        let mut other_names_success = true;
                        for (seq, other_name) in aircraft_reg.other_names.iter().enumerate() {
                            let seq_num = (seq + 1) as i16; // Convert to 1-based index
                            let insert_result = sqlx::query!(
                                "INSERT INTO aircraft_other_names (registration_number, seq, other_name) VALUES ($1, $2, $3)",
                                aircraft_reg.n_number,
                                seq_num,
                                other_name
                            )
                            .execute(&mut *transaction)
                            .await;

                            if let Err(e) = insert_result {
                                warn!(
                                    "Failed to insert other name for aircraft registration {}: {}",
                                    aircraft_reg.n_number, e
                                );
                                other_names_success = false;
                                break;
                            }
                        }

                        if !other_names_success {
                            transaction.rollback().await?;
                            failed_count += 1;
                            continue;
                        }
                    }

                    // Commit the transaction for this aircraft
                    match transaction.commit().await {
                        Ok(_) => {
                            upserted_count += 1;
                        }
                        Err(e) => {
                            warn!(
                                "Failed to commit transaction for aircraft registration {}: {}",
                                aircraft_reg.n_number, e
                            );
                            failed_count += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to upsert aircraft registration {}: {}\nAircraft data: {:#?}",
                        aircraft_reg.n_number, e, aircraft_reg
                    );
                    transaction.rollback().await?;
                    failed_count += 1;
                }
            }
        }

        if failed_count > 0 {
            warn!("Failed to upsert {} aircraft registrations", failed_count);
        }
        info!(
            "Successfully upserted {} aircraft registrations",
            upserted_count
        );

        Ok(upserted_count)
    }

    /// Get the total count of aircraft registrations in the database
    pub async fn get_aircraft_registration_count(&self) -> Result<i64> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM aircraft_registrations")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Get an aircraft registration by its N-number
    pub async fn get_aircraft_registration_by_n_number(
        &self,
        n_number: &str,
    ) -> Result<Option<Aircraft>> {
        let result = sqlx::query!(
            r#"
            SELECT a.registration_number, a.serial_number, a.mfr_mdl_code, a.eng_mfr_mdl_code, a.year_mfr,
                   a.type_registration_code, a.registrant_name, a.last_action_date, a.certificate_issue_date,
                   a.airworthiness_class as "airworthiness_class: AirworthinessClass",
                   a.approved_operations_raw, a.location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code,
                   l.region_code, l.county_mail_code, l.country_mail_code,
                   op_restricted_other, op_restricted_ag_pest_control, op_restricted_aerial_surveying,
                   op_restricted_aerial_advertising, op_restricted_forest, op_restricted_patrolling,
                   op_restricted_weather_control, op_restricted_carriage_of_cargo,
                   a.op_experimental_show_compliance, a.op_experimental_research_development, a.op_experimental_amateur_built,
                   a.op_experimental_exhibition, a.op_experimental_racing, a.op_experimental_crew_training,
                   a.op_experimental_market_survey, a.op_experimental_operating_kit_built,
                   a.op_experimental_light_sport_reg_prior_2008, a.op_experimental_light_sport_operating_kit_built,
                   a.op_experimental_light_sport_prev_21_190, a.op_experimental_uas_research_development,
                   a.op_experimental_uas_market_survey, a.op_experimental_uas_crew_training,
                   a.op_experimental_uas_exhibition, a.op_experimental_uas_compliance_with_cfr,
                   a.op_sfp_ferry_for_repairs_alterations_storage, a.op_sfp_evacuate_impending_danger,
                   a.op_sfp_excess_of_max_certificated, a.op_sfp_delivery_or_export,
                   a.op_sfp_production_flight_testing, a.op_sfp_customer_demo,
                   a.type_aircraft_code, a.type_engine_code, a.status_code, a.transponder_code,
                   a.fractional_owner, a.airworthiness_date, a.expiration_date, a.unique_id,
                   a.kit_mfr_name, a.kit_model_name
            FROM aircraft_registrations a
            LEFT JOIN locations l ON a.location_id = l.id
            WHERE a.registration_number = $1
            "#,
            n_number
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            // Get other names
            let other_names_result = sqlx::query!(
                "SELECT other_name FROM aircraft_other_names WHERE registration_number = $1 ORDER BY seq",
                n_number
            )
            .fetch_all(&self.pool)
            .await?;

            let other_names = other_names_result
                .into_iter()
                .filter_map(|r| r.other_name)
                .collect::<Vec<_>>();

            // Convert database types back to Rust types
            let year_mfr = row.year_mfr.map(|y| y as u16);
            let transponder_code = row.transponder_code.map(|t| t as u32);

            // Reconstruct ApprovedOps from database boolean fields
            let approved_ops = ApprovedOps {
                restricted_other: row.op_restricted_other.unwrap_or(false),
                restricted_ag_pest_control: row.op_restricted_ag_pest_control.unwrap_or(false),
                restricted_aerial_surveying: row.op_restricted_aerial_surveying.unwrap_or(false),
                restricted_aerial_advertising: row
                    .op_restricted_aerial_advertising
                    .unwrap_or(false),
                restricted_forest: row.op_restricted_forest.unwrap_or(false),
                restricted_patrolling: row.op_restricted_patrolling.unwrap_or(false),
                restricted_weather_control: row.op_restricted_weather_control.unwrap_or(false),
                restricted_carriage_of_cargo: row.op_restricted_carriage_of_cargo.unwrap_or(false),
                exp_show_compliance: row.op_experimental_show_compliance.unwrap_or(false),
                exp_research_development: row.op_experimental_research_development.unwrap_or(false),
                exp_amateur_built: row.op_experimental_amateur_built.unwrap_or(false),
                exp_exhibition: row.op_experimental_exhibition.unwrap_or(false),
                exp_racing: row.op_experimental_racing.unwrap_or(false),
                exp_crew_training: row.op_experimental_crew_training.unwrap_or(false),
                exp_market_survey: row.op_experimental_market_survey.unwrap_or(false),
                exp_operating_kit_built: row.op_experimental_operating_kit_built.unwrap_or(false),
                exp_lsa_reg_prior_2008: row
                    .op_experimental_light_sport_reg_prior_2008
                    .unwrap_or(false),
                exp_lsa_operating_kit_built: row
                    .op_experimental_light_sport_operating_kit_built
                    .unwrap_or(false),
                exp_lsa_prev_21_190: row.op_experimental_light_sport_prev_21_190.unwrap_or(false),
                exp_uas_research_development: row
                    .op_experimental_uas_research_development
                    .unwrap_or(false),
                exp_uas_market_survey: row.op_experimental_uas_market_survey.unwrap_or(false),
                exp_uas_crew_training: row.op_experimental_uas_crew_training.unwrap_or(false),
                exp_uas_exhibition: row.op_experimental_uas_exhibition.unwrap_or(false),
                exp_uas_compliance_with_cfr: row
                    .op_experimental_uas_compliance_with_cfr
                    .unwrap_or(false),
                sfp_ferry_for_repairs_alterations_storage: row
                    .op_sfp_ferry_for_repairs_alterations_storage
                    .unwrap_or(false),
                sfp_evacuate_impending_danger: row
                    .op_sfp_evacuate_impending_danger
                    .unwrap_or(false),
                sfp_excess_of_max_certificated: row
                    .op_sfp_excess_of_max_certificated
                    .unwrap_or(false),
                sfp_delivery_or_export: row.op_sfp_delivery_or_export.unwrap_or(false),
                sfp_production_flight_testing: row
                    .op_sfp_production_flight_testing
                    .unwrap_or(false),
                sfp_customer_demo: row.op_sfp_customer_demo.unwrap_or(false),
            };

            Ok(Some(Aircraft {
                n_number: row.registration_number,
                serial_number: row.serial_number,
                mfr_mdl_code: row.mfr_mdl_code,
                eng_mfr_mdl_code: row.eng_mfr_mdl_code,
                year_mfr,
                type_registration_code: row.type_registration_code,
                registrant_name: row.registrant_name,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                last_action_date: row.last_action_date,
                certificate_issue_date: row.certificate_issue_date,
                airworthiness_class: None, // TODO: Convert from database string to enum
                approved_operations_raw: row.approved_operations_raw,
                approved_ops,
                type_aircraft_code: row.type_aircraft_code,
                type_engine_code: row.type_engine_code,
                status_code: row.status_code,
                transponder_code,
                fractional_owner: row.fractional_owner,
                airworthiness_date: row.airworthiness_date,
                other_names,
                expiration_date: row.expiration_date,
                unique_id: row.unique_id,
                kit_mfr_name: row.kit_mfr_name,
                kit_model_name: row.kit_model_name,
                home_base_airport_id: None,
                location_id: row.location_id,
                registered_location: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Search aircraft registrations by registrant name (case-insensitive partial match)
    pub async fn search_by_registrant_name(&self, registrant_name: &str) -> Result<Vec<Aircraft>> {
        let results = sqlx::query!(
            r#"
            SELECT registration_number, serial_number, mfr_mdl_code, eng_mfr_mdl_code, year_mfr,
                   type_registration_code, registrant_name, last_action_date, certificate_issue_date,
                   airworthiness_class AS "airworthiness_class: AirworthinessClass", approved_operations_raw,
                   op_restricted_other, op_restricted_ag_pest_control, op_restricted_aerial_surveying,
                   op_restricted_aerial_advertising, op_restricted_forest, op_restricted_patrolling,
                   op_restricted_weather_control, op_restricted_carriage_of_cargo,
                   op_experimental_show_compliance, op_experimental_research_development, op_experimental_amateur_built,
                   op_experimental_exhibition, op_experimental_racing, op_experimental_crew_training,
                   op_experimental_market_survey, op_experimental_operating_kit_built,
                   op_experimental_light_sport_reg_prior_2008, op_experimental_light_sport_operating_kit_built,
                   op_experimental_light_sport_prev_21_190, op_experimental_uas_research_development,
                   op_experimental_uas_market_survey, op_experimental_uas_crew_training,
                   op_experimental_uas_exhibition, op_experimental_uas_compliance_with_cfr,
                   op_sfp_ferry_for_repairs_alterations_storage, op_sfp_evacuate_impending_danger,
                   op_sfp_excess_of_max_certificated, op_sfp_delivery_or_export,
                   op_sfp_production_flight_testing, op_sfp_customer_demo,
                   type_aircraft_code, type_engine_code, status_code, transponder_code,
                   fractional_owner, airworthiness_date, expiration_date, unique_id,
                   kit_mfr_name, kit_model_name, location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code,
                   l.region_code, l.county_mail_code, l.country_mail_code
            FROM aircraft_registrations
            LEFT JOIN locations l ON aircraft_registrations.location_id = l.id
            WHERE registrant_name ILIKE $1
            ORDER BY registrant_name, registration_number
            "#,
            format!("%{}%", registrant_name)
        )
        .fetch_all(&self.pool)
        .await?;

        let mut aircraft_list = Vec::new();
        for row in results {
            // Get other names for this aircraft
            let other_names_result = sqlx::query!(
                "SELECT other_name FROM aircraft_other_names WHERE registration_number = $1 ORDER BY seq",
                row.registration_number
            )
            .fetch_all(&self.pool)
            .await?;

            let other_names = other_names_result
                .into_iter()
                .filter_map(|r| r.other_name)
                .collect::<Vec<_>>();

            // Convert database types back to Rust types
            let year_mfr = row.year_mfr.map(|y| y as u16);
            let transponder_code = row.transponder_code.map(|t| t as u32);

            // Reconstruct ApprovedOps from database boolean fields
            let approved_ops = ApprovedOps {
                restricted_other: row.op_restricted_other.unwrap_or(false),
                restricted_ag_pest_control: row.op_restricted_ag_pest_control.unwrap_or(false),
                restricted_aerial_surveying: row.op_restricted_aerial_surveying.unwrap_or(false),
                restricted_aerial_advertising: row
                    .op_restricted_aerial_advertising
                    .unwrap_or(false),
                restricted_forest: row.op_restricted_forest.unwrap_or(false),
                restricted_patrolling: row.op_restricted_patrolling.unwrap_or(false),
                restricted_weather_control: row.op_restricted_weather_control.unwrap_or(false),
                restricted_carriage_of_cargo: row.op_restricted_carriage_of_cargo.unwrap_or(false),
                exp_show_compliance: row.op_experimental_show_compliance.unwrap_or(false),
                exp_research_development: row.op_experimental_research_development.unwrap_or(false),
                exp_amateur_built: row.op_experimental_amateur_built.unwrap_or(false),
                exp_exhibition: row.op_experimental_exhibition.unwrap_or(false),
                exp_racing: row.op_experimental_racing.unwrap_or(false),
                exp_crew_training: row.op_experimental_crew_training.unwrap_or(false),
                exp_market_survey: row.op_experimental_market_survey.unwrap_or(false),
                exp_operating_kit_built: row.op_experimental_operating_kit_built.unwrap_or(false),
                exp_lsa_reg_prior_2008: row
                    .op_experimental_light_sport_reg_prior_2008
                    .unwrap_or(false),
                exp_lsa_operating_kit_built: row
                    .op_experimental_light_sport_operating_kit_built
                    .unwrap_or(false),
                exp_lsa_prev_21_190: row.op_experimental_light_sport_prev_21_190.unwrap_or(false),
                exp_uas_research_development: row
                    .op_experimental_uas_research_development
                    .unwrap_or(false),
                exp_uas_market_survey: row.op_experimental_uas_market_survey.unwrap_or(false),
                exp_uas_crew_training: row.op_experimental_uas_crew_training.unwrap_or(false),
                exp_uas_exhibition: row.op_experimental_uas_exhibition.unwrap_or(false),
                exp_uas_compliance_with_cfr: row
                    .op_experimental_uas_compliance_with_cfr
                    .unwrap_or(false),
                sfp_ferry_for_repairs_alterations_storage: row
                    .op_sfp_ferry_for_repairs_alterations_storage
                    .unwrap_or(false),
                sfp_evacuate_impending_danger: row
                    .op_sfp_evacuate_impending_danger
                    .unwrap_or(false),
                sfp_excess_of_max_certificated: row
                    .op_sfp_excess_of_max_certificated
                    .unwrap_or(false),
                sfp_delivery_or_export: row.op_sfp_delivery_or_export.unwrap_or(false),
                sfp_production_flight_testing: row
                    .op_sfp_production_flight_testing
                    .unwrap_or(false),
                sfp_customer_demo: row.op_sfp_customer_demo.unwrap_or(false),
            };

            aircraft_list.push(Aircraft {
                n_number: row.registration_number,
                serial_number: row.serial_number,
                mfr_mdl_code: row.mfr_mdl_code,
                eng_mfr_mdl_code: row.eng_mfr_mdl_code,
                year_mfr,
                type_registration_code: row.type_registration_code,
                registrant_name: row.registrant_name,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                last_action_date: row.last_action_date,
                certificate_issue_date: row.certificate_issue_date,
                airworthiness_class: None, // TODO: Convert from database string to enum
                approved_operations_raw: row.approved_operations_raw,
                approved_ops,
                type_aircraft_code: row.type_aircraft_code,
                type_engine_code: row.type_engine_code,
                status_code: row.status_code,
                transponder_code,
                fractional_owner: row.fractional_owner,
                airworthiness_date: row.airworthiness_date,
                other_names,
                expiration_date: row.expiration_date,
                unique_id: row.unique_id,
                kit_mfr_name: row.kit_mfr_name,
                kit_model_name: row.kit_model_name,
                home_base_airport_id: None,
                location_id: row.location_id,
                registered_location: None,
            });
        }

        Ok(aircraft_list)
    }

    /// Search aircraft registrations by transponder code
    pub async fn search_by_transponder_code(&self, transponder_code: u32) -> Result<Vec<Aircraft>> {
        let transponder_code_i64 = transponder_code as i64;
        let results = sqlx::query!(
            r#"
            SELECT registration_number, serial_number, mfr_mdl_code, eng_mfr_mdl_code, year_mfr,
                   type_registration_code, registrant_name, last_action_date, certificate_issue_date,
                   airworthiness_class as "airworthiness_class: AirworthinessClass",
                   approved_operations_raw,
                   op_restricted_other, op_restricted_ag_pest_control, op_restricted_aerial_surveying,
                   op_restricted_aerial_advertising, op_restricted_forest, op_restricted_patrolling,
                   op_restricted_weather_control, op_restricted_carriage_of_cargo,
                   op_experimental_show_compliance, op_experimental_research_development, op_experimental_amateur_built,
                   op_experimental_exhibition, op_experimental_racing, op_experimental_crew_training,
                   op_experimental_market_survey, op_experimental_operating_kit_built,
                   op_experimental_light_sport_reg_prior_2008, op_experimental_light_sport_operating_kit_built,
                   op_experimental_light_sport_prev_21_190, op_experimental_uas_research_development,
                   op_experimental_uas_market_survey, op_experimental_uas_crew_training,
                   op_experimental_uas_exhibition, op_experimental_uas_compliance_with_cfr,
                   op_sfp_ferry_for_repairs_alterations_storage, op_sfp_evacuate_impending_danger,
                   op_sfp_excess_of_max_certificated, op_sfp_delivery_or_export,
                   op_sfp_production_flight_testing, op_sfp_customer_demo,
                   type_aircraft_code, type_engine_code, status_code, transponder_code,
                   fractional_owner, airworthiness_date, expiration_date, unique_id,
                   kit_mfr_name, kit_model_name, location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code,
                   l.region_code, l.county_mail_code, l.country_mail_code
            FROM aircraft_registrations
            LEFT JOIN locations l ON aircraft_registrations.location_id = l.id
            WHERE transponder_code = $1
            ORDER BY registration_number
            "#,
            transponder_code_i64
        )
        .fetch_all(&self.pool)
        .await?;

        let mut aircraft_list = Vec::new();
        for row in results {
            // Get other names for this aircraft
            let other_names_result = sqlx::query!(
                "SELECT other_name FROM aircraft_other_names WHERE registration_number = $1 ORDER BY seq",
                row.registration_number
            )
            .fetch_all(&self.pool)
            .await?;

            let other_names = other_names_result
                .into_iter()
                .filter_map(|r| r.other_name)
                .collect::<Vec<_>>();

            // Convert database types back to Rust types
            let year_mfr = row.year_mfr.map(|y| y as u16);
            let transponder_code = row.transponder_code.map(|t| t as u32);

            // Reconstruct ApprovedOps from database boolean fields
            let approved_ops = ApprovedOps {
                restricted_other: row.op_restricted_other.unwrap_or(false),
                restricted_ag_pest_control: row.op_restricted_ag_pest_control.unwrap_or(false),
                restricted_aerial_surveying: row.op_restricted_aerial_surveying.unwrap_or(false),
                restricted_aerial_advertising: row
                    .op_restricted_aerial_advertising
                    .unwrap_or(false),
                restricted_forest: row.op_restricted_forest.unwrap_or(false),
                restricted_patrolling: row.op_restricted_patrolling.unwrap_or(false),
                restricted_weather_control: row.op_restricted_weather_control.unwrap_or(false),
                restricted_carriage_of_cargo: row.op_restricted_carriage_of_cargo.unwrap_or(false),
                exp_show_compliance: row.op_experimental_show_compliance.unwrap_or(false),
                exp_research_development: row.op_experimental_research_development.unwrap_or(false),
                exp_amateur_built: row.op_experimental_amateur_built.unwrap_or(false),
                exp_exhibition: row.op_experimental_exhibition.unwrap_or(false),
                exp_racing: row.op_experimental_racing.unwrap_or(false),
                exp_crew_training: row.op_experimental_crew_training.unwrap_or(false),
                exp_market_survey: row.op_experimental_market_survey.unwrap_or(false),
                exp_operating_kit_built: row.op_experimental_operating_kit_built.unwrap_or(false),
                exp_lsa_reg_prior_2008: row
                    .op_experimental_light_sport_reg_prior_2008
                    .unwrap_or(false),
                exp_lsa_operating_kit_built: row
                    .op_experimental_light_sport_operating_kit_built
                    .unwrap_or(false),
                exp_lsa_prev_21_190: row.op_experimental_light_sport_prev_21_190.unwrap_or(false),
                exp_uas_research_development: row
                    .op_experimental_uas_research_development
                    .unwrap_or(false),
                exp_uas_market_survey: row.op_experimental_uas_market_survey.unwrap_or(false),
                exp_uas_crew_training: row.op_experimental_uas_crew_training.unwrap_or(false),
                exp_uas_exhibition: row.op_experimental_uas_exhibition.unwrap_or(false),
                exp_uas_compliance_with_cfr: row
                    .op_experimental_uas_compliance_with_cfr
                    .unwrap_or(false),
                sfp_ferry_for_repairs_alterations_storage: row
                    .op_sfp_ferry_for_repairs_alterations_storage
                    .unwrap_or(false),
                sfp_evacuate_impending_danger: row
                    .op_sfp_evacuate_impending_danger
                    .unwrap_or(false),
                sfp_excess_of_max_certificated: row
                    .op_sfp_excess_of_max_certificated
                    .unwrap_or(false),
                sfp_delivery_or_export: row.op_sfp_delivery_or_export.unwrap_or(false),
                sfp_production_flight_testing: row
                    .op_sfp_production_flight_testing
                    .unwrap_or(false),
                sfp_customer_demo: row.op_sfp_customer_demo.unwrap_or(false),
            };

            aircraft_list.push(Aircraft {
                n_number: row.registration_number,
                serial_number: row.serial_number,
                mfr_mdl_code: row.mfr_mdl_code,
                eng_mfr_mdl_code: row.eng_mfr_mdl_code,
                year_mfr,
                type_registration_code: row.type_registration_code,
                registrant_name: row.registrant_name,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                last_action_date: row.last_action_date,
                certificate_issue_date: row.certificate_issue_date,
                airworthiness_class: None, // TODO: Convert from database string to enum
                approved_operations_raw: row.approved_operations_raw,
                approved_ops,
                type_aircraft_code: row.type_aircraft_code,
                type_engine_code: row.type_engine_code,
                status_code: row.status_code,
                transponder_code,
                fractional_owner: row.fractional_owner,
                airworthiness_date: row.airworthiness_date,
                other_names,
                expiration_date: row.expiration_date,
                unique_id: row.unique_id,
                kit_mfr_name: row.kit_mfr_name,
                kit_model_name: row.kit_model_name,
                home_base_airport_id: None,
                location_id: row.location_id,
                registered_location: None,
            });
        }

        Ok(aircraft_list)
    }

    /// Search aircraft registrations by state
    pub async fn search_by_state(&self, state: &str) -> Result<Vec<Aircraft>> {
        let results = sqlx::query!(
            r#"
            SELECT a.registration_number, a.serial_number, a.mfr_mdl_code, a.eng_mfr_mdl_code, a.year_mfr,
                   a.type_registration_code, a.registrant_name, a.last_action_date, a.certificate_issue_date,
                   a.airworthiness_class as "airworthiness_class: AirworthinessClass", a.approved_operations_raw, a.location_id,
                   l.street1, l.street2, l.city, l.state, l.zip_code,
                   l.region_code, l.county_mail_code, l.country_mail_code,
                   a.op_restricted_other, a.op_restricted_ag_pest_control, a.op_restricted_aerial_surveying,
                   a.op_restricted_aerial_advertising, a.op_restricted_forest, a.op_restricted_patrolling,
                   a.op_restricted_weather_control, a.op_restricted_carriage_of_cargo,
                   a.op_experimental_show_compliance, a.op_experimental_research_development, a.op_experimental_amateur_built,
                   a.op_experimental_exhibition, a.op_experimental_racing, a.op_experimental_crew_training,
                   a.op_experimental_market_survey, a.op_experimental_operating_kit_built,
                   a.op_experimental_light_sport_reg_prior_2008, a.op_experimental_light_sport_operating_kit_built,
                   a.op_experimental_light_sport_prev_21_190, a.op_experimental_uas_research_development,
                   a.op_experimental_uas_market_survey, a.op_experimental_uas_crew_training,
                   a.op_experimental_uas_exhibition, a.op_experimental_uas_compliance_with_cfr,
                   a.op_sfp_ferry_for_repairs_alterations_storage, a.op_sfp_evacuate_impending_danger,
                   a.op_sfp_excess_of_max_certificated, a.op_sfp_delivery_or_export,
                   a.op_sfp_production_flight_testing, a.op_sfp_customer_demo,
                   a.type_aircraft_code, a.type_engine_code, a.status_code, a.transponder_code,
                   a.fractional_owner, a.airworthiness_date, a.expiration_date, a.unique_id,
                   a.kit_mfr_name, a.kit_model_name
            FROM aircraft_registrations a
            LEFT JOIN locations l ON a.location_id = l.id
            WHERE l.state = $1
            ORDER BY registrant_name, registration_number
            "#,
            state
        )
        .fetch_all(&self.pool)
        .await?;

        let mut aircraft_list = Vec::new();
        for row in results {
            // Get other names for this aircraft
            let other_names_result = sqlx::query!(
                "SELECT other_name FROM aircraft_other_names WHERE registration_number = $1 ORDER BY seq",
                row.registration_number
            )
            .fetch_all(&self.pool)
            .await?;

            let other_names = other_names_result
                .into_iter()
                .filter_map(|r| r.other_name)
                .collect::<Vec<_>>();

            // Convert database types back to Rust types
            let year_mfr = row.year_mfr.map(|y| y as u16);
            let transponder_code = row.transponder_code.map(|t| t as u32);

            // Reconstruct ApprovedOps from database boolean fields
            let approved_ops = ApprovedOps {
                restricted_other: row.op_restricted_other.unwrap_or(false),
                restricted_ag_pest_control: row.op_restricted_ag_pest_control.unwrap_or(false),
                restricted_aerial_surveying: row.op_restricted_aerial_surveying.unwrap_or(false),
                restricted_aerial_advertising: row
                    .op_restricted_aerial_advertising
                    .unwrap_or(false),
                restricted_forest: row.op_restricted_forest.unwrap_or(false),
                restricted_patrolling: row.op_restricted_patrolling.unwrap_or(false),
                restricted_weather_control: row.op_restricted_weather_control.unwrap_or(false),
                restricted_carriage_of_cargo: row.op_restricted_carriage_of_cargo.unwrap_or(false),
                exp_show_compliance: row.op_experimental_show_compliance.unwrap_or(false),
                exp_research_development: row.op_experimental_research_development.unwrap_or(false),
                exp_amateur_built: row.op_experimental_amateur_built.unwrap_or(false),
                exp_exhibition: row.op_experimental_exhibition.unwrap_or(false),
                exp_racing: row.op_experimental_racing.unwrap_or(false),
                exp_crew_training: row.op_experimental_crew_training.unwrap_or(false),
                exp_market_survey: row.op_experimental_market_survey.unwrap_or(false),
                exp_operating_kit_built: row.op_experimental_operating_kit_built.unwrap_or(false),
                exp_lsa_reg_prior_2008: row
                    .op_experimental_light_sport_reg_prior_2008
                    .unwrap_or(false),
                exp_lsa_operating_kit_built: row
                    .op_experimental_light_sport_operating_kit_built
                    .unwrap_or(false),
                exp_lsa_prev_21_190: row.op_experimental_light_sport_prev_21_190.unwrap_or(false),
                exp_uas_research_development: row
                    .op_experimental_uas_research_development
                    .unwrap_or(false),
                exp_uas_market_survey: row.op_experimental_uas_market_survey.unwrap_or(false),
                exp_uas_crew_training: row.op_experimental_uas_crew_training.unwrap_or(false),
                exp_uas_exhibition: row.op_experimental_uas_exhibition.unwrap_or(false),
                exp_uas_compliance_with_cfr: row
                    .op_experimental_uas_compliance_with_cfr
                    .unwrap_or(false),
                sfp_ferry_for_repairs_alterations_storage: row
                    .op_sfp_ferry_for_repairs_alterations_storage
                    .unwrap_or(false),
                sfp_evacuate_impending_danger: row
                    .op_sfp_evacuate_impending_danger
                    .unwrap_or(false),
                sfp_excess_of_max_certificated: row
                    .op_sfp_excess_of_max_certificated
                    .unwrap_or(false),
                sfp_delivery_or_export: row.op_sfp_delivery_or_export.unwrap_or(false),
                sfp_production_flight_testing: row
                    .op_sfp_production_flight_testing
                    .unwrap_or(false),
                sfp_customer_demo: row.op_sfp_customer_demo.unwrap_or(false),
            };

            aircraft_list.push(Aircraft {
                n_number: row.registration_number,
                serial_number: row.serial_number,
                mfr_mdl_code: row.mfr_mdl_code,
                eng_mfr_mdl_code: row.eng_mfr_mdl_code,
                year_mfr,
                type_registration_code: row.type_registration_code,
                registrant_name: row.registrant_name,
                street1: row.street1,
                street2: row.street2,
                city: row.city,
                state: row.state,
                zip_code: row.zip_code,
                region_code: row.region_code,
                county_mail_code: row.county_mail_code,
                country_mail_code: row.country_mail_code,
                last_action_date: row.last_action_date,
                certificate_issue_date: row.certificate_issue_date,
                airworthiness_class: None, // TODO: Convert from database string to enum
                approved_operations_raw: row.approved_operations_raw,
                approved_ops,
                type_aircraft_code: row.type_aircraft_code,
                type_engine_code: row.type_engine_code,
                status_code: row.status_code,
                transponder_code,
                fractional_owner: row.fractional_owner,
                airworthiness_date: row.airworthiness_date,
                other_names,
                expiration_date: row.expiration_date,
                unique_id: row.unique_id,
                kit_mfr_name: row.kit_mfr_name,
                kit_model_name: row.kit_model_name,
                home_base_airport_id: None,
                location_id: row.location_id,
                registered_location: None,
            });
        }

        Ok(aircraft_list)
    }

}

#[cfg(test)]
mod tests {
    use crate::faa::aircraft_registrations::{Aircraft, AirworthinessClass, ApprovedOps};

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_aircraft() -> Aircraft {
        Aircraft {
            n_number: "N123AB".to_string(),
            serial_number: "12345".to_string(),
            mfr_mdl_code: Some("1660225".to_string()),
            eng_mfr_mdl_code: Some("12345".to_string()),
            year_mfr: Some(2020),
            type_registration_code: Some("1".to_string()),
            registrant_name: Some("Test Owner".to_string()),
            street1: Some("123 Test St".to_string()),
            street2: None,
            city: Some("Test City".to_string()),
            state: Some("CA".to_string()),
            zip_code: Some("12345".to_string()),
            region_code: Some("4".to_string()),
            county_mail_code: Some("037".to_string()),
            country_mail_code: Some("US".to_string()),
            last_action_date: None,
            certificate_issue_date: None,
            airworthiness_class: Some(AirworthinessClass::Standard),
            approved_operations_raw: None,
            approved_ops: ApprovedOps::default(),
            type_aircraft_code: Some("4".to_string()),
            type_engine_code: Some(1),
            status_code: Some("V".to_string()),
            transponder_code: Some(0xA12345),
            fractional_owner: Some(false),
            airworthiness_date: None,
            other_names: vec![],
            expiration_date: None,
            unique_id: Some("12345678".to_string()),
            kit_mfr_name: None,
            kit_model_name: None,
            home_base_airport_id: None,
            registered_location: None,
            location_id: None,
        }
    }

    #[test]
    fn test_aircraft_creation() {
        let aircraft = create_test_aircraft();
        assert_eq!(aircraft.n_number, "N123AB");
        assert_eq!(aircraft.serial_number, "12345");
        assert_eq!(aircraft.mfr_mdl_code, Some("1660225".to_string()));
        assert_eq!(aircraft.year_mfr, Some(2020));
        assert_eq!(aircraft.registrant_name, Some("Test Owner".to_string()));
        assert_eq!(aircraft.state, Some("CA".to_string()));
        assert_eq!(aircraft.status_code, Some("V".to_string()));
        assert_eq!(aircraft.transponder_code, Some(0xA12345));
    }
}
