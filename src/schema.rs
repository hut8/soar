// @generated automatically by Diesel CLI.

diesel::table! {
    aircraft_registrations (registration_number) {
        registration_number -> Varchar,
        serial_number -> Varchar,
        mfr_mdl_code -> Nullable<Varchar>,
        eng_mfr_mdl_code -> Nullable<Varchar>,
        year_mfr -> Nullable<Int4>,
        type_registration_code -> Nullable<Bpchar>,
        registrant_name -> Nullable<Varchar>,
        location_id -> Nullable<Uuid>,
        last_action_date -> Nullable<Date>,
        certificate_issue_date -> Nullable<Date>,
        airworthiness_class -> Nullable<Text>,
        approved_operations_raw -> Nullable<Varchar>,
        op_restricted_other -> Nullable<Bool>,
        op_restricted_ag_pest_control -> Nullable<Bool>,
        op_restricted_aerial_surveying -> Nullable<Bool>,
        op_restricted_aerial_advertising -> Nullable<Bool>,
        op_restricted_forest -> Nullable<Bool>,
        op_restricted_patrolling -> Nullable<Bool>,
        op_restricted_weather_control -> Nullable<Bool>,
        op_restricted_carriage_of_cargo -> Nullable<Bool>,
        op_experimental_show_compliance -> Nullable<Bool>,
        op_experimental_research_development -> Nullable<Bool>,
        op_experimental_amateur_built -> Nullable<Bool>,
        op_experimental_exhibition -> Nullable<Bool>,
        op_experimental_racing -> Nullable<Bool>,
        op_experimental_crew_training -> Nullable<Bool>,
        op_experimental_market_survey -> Nullable<Bool>,
        op_experimental_operating_kit_built -> Nullable<Bool>,
        op_experimental_light_sport_reg_prior_2008 -> Nullable<Bool>,
        op_experimental_light_sport_operating_kit_built -> Nullable<Bool>,
        op_experimental_light_sport_prev_21_190 -> Nullable<Bool>,
        op_experimental_uas_research_development -> Nullable<Bool>,
        op_experimental_uas_market_survey -> Nullable<Bool>,
        op_experimental_uas_crew_training -> Nullable<Bool>,
        op_experimental_uas_exhibition -> Nullable<Bool>,
        op_experimental_uas_compliance_with_cfr -> Nullable<Bool>,
        op_sfp_ferry_for_repairs_alterations_storage -> Nullable<Bool>,
        op_sfp_evacuate_impending_danger -> Nullable<Bool>,
        op_sfp_excess_of_max_certificated -> Nullable<Bool>,
        op_sfp_delivery_or_export -> Nullable<Bool>,
        op_sfp_production_flight_testing -> Nullable<Bool>,
        op_sfp_customer_demo -> Nullable<Bool>,
        type_aircraft_code -> Nullable<Bpchar>,
        type_engine_code -> Nullable<Int2>,
        status_code -> Nullable<Text>,
        transponder_code -> Nullable<Int8>,
        fractional_owner -> Nullable<Bool>,
        airworthiness_date -> Nullable<Date>,
        expiration_date -> Nullable<Date>,
        unique_id -> Nullable<Bpchar>,
        kit_mfr_name -> Nullable<Varchar>,
        kit_model_name -> Nullable<Varchar>,
        club_id -> Nullable<Uuid>,
        device_id -> Nullable<Int4>,
    }
}

diesel::table! {
    airports (id) {
        id -> Int4,
        ident -> Varchar,
        #[sql_name = "type"]
        airport_type -> Varchar,
        name -> Varchar,
        latitude_deg -> Nullable<Float8>,
        longitude_deg -> Nullable<Float8>,
        elevation_ft -> Nullable<Int4>,
        continent -> Nullable<Varchar>,
        iso_country -> Nullable<Varchar>,
        iso_region -> Nullable<Varchar>,
        municipality -> Nullable<Varchar>,
        scheduled_service -> Bool,
        icao_code -> Nullable<Varchar>,
        iata_code -> Nullable<Varchar>,
        gps_code -> Nullable<Varchar>,
        local_code -> Nullable<Varchar>,
        home_link -> Nullable<Text>,
        wikipedia_link -> Nullable<Text>,
        keywords -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        location -> Nullable<diesel::sql_types::Text>,
    }
}

diesel::table! {
    devices (device_id) {
        device_id -> Int4,
        device_type -> Text,
        aircraft_model -> Text,
        registration -> Text,
        competition_number -> Text,
        tracked -> Bool,
        identified -> Bool,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        user_id -> Nullable<Uuid>,
    }
}

// TODO: Add joinable and allow_tables_to_appear_in_same_query macros when needed
// diesel::joinable!(aircraft_registrations -> devices (device_id));
// 
// diesel::allow_tables_to_appear_in_same_query!(
//     aircraft_registrations,
//     devices,
// );