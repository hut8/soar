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
    clubs (id) {
        id -> Uuid,
        name -> Varchar,
        is_soaring -> Nullable<Bool>,
        home_base_airport_id -> Nullable<Int4>,
        location_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
    }
}

diesel::table! {
    flights (id) {
        id -> Uuid,
        aircraft_id -> Varchar,
        takeoff_time -> Timestamptz,
        landing_time -> Nullable<Timestamptz>,
        departure_airport -> Nullable<Varchar>,
        arrival_airport -> Nullable<Varchar>,
        tow_aircraft_id -> Nullable<Varchar>,
        tow_release_height_msl -> Nullable<Int4>,
        club_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    locations (id) {
        id -> Uuid,
        street1 -> Nullable<Varchar>,
        street2 -> Nullable<Varchar>,
        city -> Nullable<Varchar>,
        state -> Nullable<Varchar>,
        zip_code -> Nullable<Varchar>,
        region_code -> Nullable<Varchar>,
        county_mail_code -> Nullable<Varchar>,
        country_mail_code -> Nullable<Varchar>,
        geolocation -> Nullable<diesel::sql_types::Text>, // Using text for PostGIS compatibility
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    receivers (id) {
        id -> Int4,
        callsign -> Varchar,
        description -> Nullable<Text>,
        contact -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        country -> Nullable<Varchar>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    receivers_photos (id) {
        id -> Int4,
        receiver_id -> Int4,
        photo_url -> Varchar,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    receivers_links (id) {
        id -> Int4,
        receiver_id -> Int4,
        rel -> Nullable<Varchar>,
        href -> Varchar,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    fixes (id) {
        id -> Uuid,
        source -> Varchar,
        destination -> Varchar,
        via -> Array<Text>,
        raw_packet -> Text,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        altitude_feet -> Nullable<Int4>,
        aircraft_id -> Nullable<Varchar>,
        device_id -> Nullable<Int4>,
        device_type -> Nullable<Text>,
        aircraft_type -> Nullable<Text>,
        flight_number -> Nullable<Varchar>,
        emitter_category -> Nullable<Text>,
        registration -> Nullable<Varchar>,
        model -> Nullable<Varchar>,
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        snr_db -> Nullable<Float4>,
        bit_errors_corrected -> Nullable<Int4>,
        freq_offset_khz -> Nullable<Float4>,
        club_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        first_name -> Varchar,
        last_name -> Varchar,
        email -> Varchar,
        password_hash -> Varchar,
        access_level -> Text, // Will map to enum
        club_id -> Nullable<Uuid>,
        email_verified -> Nullable<Bool>,
        password_reset_token -> Nullable<Varchar>,
        password_reset_expires_at -> Nullable<Timestamptz>,
        email_verification_token -> Nullable<Varchar>,
        email_verification_expires_at -> Nullable<Timestamptz>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    runways (id) {
        id -> Int4,
        airport_ref -> Int4,
        airport_ident -> Varchar,
        length_ft -> Nullable<Int4>,
        width_ft -> Nullable<Int4>,
        surface -> Nullable<Varchar>,
        lighted -> Bool,
        closed -> Bool,
        le_ident -> Nullable<Varchar>,
        le_latitude_deg -> Nullable<Numeric>,
        le_longitude_deg -> Nullable<Numeric>,
        le_elevation_ft -> Nullable<Int4>,
        le_heading_degt -> Nullable<Numeric>,
        le_displaced_threshold_ft -> Nullable<Int4>,
        he_ident -> Nullable<Varchar>,
        he_latitude_deg -> Nullable<Numeric>,
        he_longitude_deg -> Nullable<Numeric>,
        he_elevation_ft -> Nullable<Int4>,
        he_heading_degt -> Nullable<Numeric>,
        he_displaced_threshold_ft -> Nullable<Int4>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    aircraft_model (manufacturer_code, model_code, series_code) {
        manufacturer_code -> Text,
        model_code -> Text,
        series_code -> Text,
        manufacturer_name -> Text,
        model_name -> Text,
        aircraft_type -> Nullable<Text>,
        engine_type -> Nullable<Text>,
        aircraft_category -> Nullable<Text>,
        builder_certification -> Nullable<Text>,
        number_of_engines -> Nullable<Int2>,
        number_of_seats -> Nullable<Int2>,
        weight_class -> Nullable<Text>,
        cruising_speed -> Nullable<Int2>,
        type_certificate_data_sheet -> Nullable<Text>,
        type_certificate_data_holder -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

// TODO: Add joinable and allow_tables_to_appear_in_same_query macros when needed
// diesel::joinable!(aircraft_registrations -> devices (device_id));
// 
// diesel::allow_tables_to_appear_in_same_query!(
//     aircraft_registrations,
//     devices,
// );