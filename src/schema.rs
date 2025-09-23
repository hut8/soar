// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "address_type"))]
    pub struct AddressType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "adsb_emitter_category"))]
    pub struct AdsbEmitterCategory;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "aircraft_type"))]
    pub struct AircraftType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "airworthiness_class"))]
    pub struct AirworthinessClass;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "geography"))]
    pub struct Geography;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "point", schema = "pg_catalog"))]
    pub struct Point;
}

diesel::table! {
    aircraft_models (manufacturer_code, model_code, series_code) {
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
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    aircraft_other_names (registration_number, seq) {
        #[max_length = 5]
        registration_number -> Varchar,
        seq -> Int2,
        #[max_length = 50]
        other_name -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AirworthinessClass;

    aircraft_registrations (registration_number) {
        #[max_length = 6]
        registration_number -> Varchar,
        #[max_length = 30]
        serial_number -> Varchar,
        year_mfr -> Nullable<Int4>,
        #[max_length = 1]
        type_registration_code -> Nullable<Bpchar>,
        #[max_length = 50]
        registrant_name -> Nullable<Varchar>,
        last_action_date -> Nullable<Date>,
        certificate_issue_date -> Nullable<Date>,
        op_restricted_other -> Bool,
        op_restricted_ag_pest_control -> Bool,
        op_restricted_aerial_surveying -> Bool,
        op_restricted_aerial_advertising -> Bool,
        op_restricted_forest -> Bool,
        op_restricted_patrolling -> Bool,
        op_restricted_weather_control -> Bool,
        op_restricted_carriage_of_cargo -> Bool,
        op_experimental_show_compliance -> Bool,
        op_experimental_research_development -> Bool,
        op_experimental_amateur_built -> Bool,
        op_experimental_exhibition -> Bool,
        op_experimental_racing -> Bool,
        op_experimental_crew_training -> Bool,
        op_experimental_market_survey -> Bool,
        op_experimental_operating_kit_built -> Bool,
        op_experimental_light_sport_reg_prior_2008 -> Bool,
        op_experimental_light_sport_operating_kit_built -> Bool,
        op_experimental_light_sport_prev_21_190 -> Bool,
        op_experimental_uas_research_development -> Bool,
        op_experimental_uas_market_survey -> Bool,
        op_experimental_uas_crew_training -> Bool,
        op_experimental_uas_exhibition -> Bool,
        op_experimental_uas_compliance_with_cfr -> Bool,
        op_sfp_ferry_for_repairs_alterations_storage -> Bool,
        op_sfp_evacuate_impending_danger -> Bool,
        op_sfp_excess_of_max_certificated -> Bool,
        op_sfp_delivery_or_export -> Bool,
        op_sfp_production_flight_testing -> Bool,
        op_sfp_customer_demo -> Bool,
        #[max_length = 1]
        type_aircraft_code -> Nullable<Bpchar>,
        type_engine_code -> Nullable<Int2>,
        status_code -> Nullable<Text>,
        transponder_code -> Nullable<Int8>,
        fractional_owner -> Nullable<Bool>,
        airworthiness_date -> Nullable<Date>,
        expiration_date -> Nullable<Date>,
        #[max_length = 8]
        unique_id -> Nullable<Bpchar>,
        #[max_length = 30]
        kit_mfr_name -> Nullable<Varchar>,
        #[max_length = 20]
        kit_model_name -> Nullable<Varchar>,
        #[max_length = 9]
        approved_operations_raw -> Nullable<Varchar>,
        club_id -> Nullable<Uuid>,
        home_base_airport_id -> Nullable<Int4>,
        is_tow_plane -> Nullable<Bool>,
        location_id -> Nullable<Uuid>,
        airworthiness_class -> Nullable<AirworthinessClass>,
        device_id -> Nullable<Uuid>,
        #[max_length = 3]
        manufacturer_code -> Nullable<Varchar>,
        #[max_length = 2]
        model_code -> Nullable<Varchar>,
        #[max_length = 2]
        series_code -> Nullable<Varchar>,
        #[max_length = 3]
        engine_manufacturer_code -> Nullable<Varchar>,
        #[max_length = 2]
        engine_model_code -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;

    airports (id) {
        id -> Int4,
        #[max_length = 16]
        ident -> Varchar,
        #[sql_name = "type"]
        #[max_length = 50]
        type_ -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        latitude_deg -> Nullable<Numeric>,
        longitude_deg -> Nullable<Numeric>,
        location -> Nullable<Geography>,
        elevation_ft -> Nullable<Int4>,
        #[max_length = 2]
        continent -> Nullable<Varchar>,
        #[max_length = 2]
        iso_country -> Nullable<Varchar>,
        #[max_length = 7]
        iso_region -> Nullable<Varchar>,
        #[max_length = 255]
        municipality -> Nullable<Varchar>,
        scheduled_service -> Bool,
        #[max_length = 4]
        gps_code -> Nullable<Varchar>,
        #[max_length = 4]
        icao_code -> Nullable<Varchar>,
        #[max_length = 3]
        iata_code -> Nullable<Varchar>,
        #[max_length = 7]
        local_code -> Nullable<Varchar>,
        home_link -> Nullable<Text>,
        wikipedia_link -> Nullable<Text>,
        keywords -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    clubs (id) {
        id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        is_soaring -> Nullable<Bool>,
        home_base_airport_id -> Nullable<Int4>,
        location_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    countries (code) {
        #[max_length = 2]
        code -> Bpchar,
        name -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AddressType;

    devices (id) {
        address -> Int4,
        address_type -> AddressType,
        aircraft_model -> Text,
        registration -> Text,
        competition_number -> Text,
        tracked -> Bool,
        identified -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        id -> Uuid,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::AddressType;
    use super::sql_types::AircraftType;
    use super::sql_types::AdsbEmitterCategory;

    fixes (id) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        destination -> Varchar,
        via -> Nullable<Array<Nullable<Text>>>,
        raw_packet -> Text,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_feet -> Nullable<Int4>,
        #[max_length = 10]
        device_address -> Nullable<Varchar>,
        address_type -> Nullable<AddressType>,
        aircraft_type -> Nullable<AircraftType>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        emitter_category -> Nullable<AdsbEmitterCategory>,
        #[max_length = 10]
        registration -> Nullable<Varchar>,
        #[max_length = 50]
        model -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        snr_db -> Nullable<Float4>,
        bit_errors_corrected -> Nullable<Int4>,
        freq_offset_khz -> Nullable<Float4>,
        club_id -> Nullable<Uuid>,
        flight_id -> Nullable<Uuid>,
        unparsed_data -> Nullable<Varchar>,
        device_id -> Nullable<Uuid>,
        received_at -> Timestamptz,
        lag -> Nullable<Int4>,
    }
}

diesel::table! {
    flights (id) {
        id -> Uuid,
        #[max_length = 10]
        aircraft_id -> Varchar,
        takeoff_time -> Timestamptz,
        landing_time -> Nullable<Timestamptz>,
        #[max_length = 10]
        departure_airport -> Nullable<Varchar>,
        #[max_length = 10]
        arrival_airport -> Nullable<Varchar>,
        #[max_length = 5]
        tow_aircraft_id -> Nullable<Varchar>,
        tow_release_height_msl -> Nullable<Int4>,
        club_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Point;

    locations (id) {
        id -> Uuid,
        street1 -> Nullable<Text>,
        street2 -> Nullable<Text>,
        city -> Nullable<Text>,
        state -> Nullable<Text>,
        zip_code -> Nullable<Text>,
        region_code -> Nullable<Text>,
        county_mail_code -> Nullable<Text>,
        country_mail_code -> Nullable<Text>,
        geolocation -> Nullable<Point>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    receiver_statuses (id) {
        id -> Uuid,
        receiver_id -> Int4,
        received_at -> Timestamptz,
        version -> Nullable<Text>,
        platform -> Nullable<Text>,
        cpu_load -> Nullable<Numeric>,
        ram_free -> Nullable<Numeric>,
        ram_total -> Nullable<Numeric>,
        ntp_offset -> Nullable<Numeric>,
        ntp_correction -> Nullable<Numeric>,
        voltage -> Nullable<Numeric>,
        amperage -> Nullable<Numeric>,
        cpu_temperature -> Nullable<Numeric>,
        visible_senders -> Nullable<Int2>,
        latency -> Nullable<Numeric>,
        senders -> Nullable<Int2>,
        rf_correction_manual -> Nullable<Int2>,
        rf_correction_automatic -> Nullable<Numeric>,
        noise -> Nullable<Numeric>,
        senders_signal_quality -> Nullable<Numeric>,
        senders_messages -> Nullable<Int4>,
        good_senders_signal_quality -> Nullable<Numeric>,
        good_senders -> Nullable<Int2>,
        good_and_bad_senders -> Nullable<Int2>,
        geoid_offset -> Nullable<Int2>,
        name -> Nullable<Text>,
        demodulation_snr_db -> Nullable<Numeric>,
        ognr_pilotaware_version -> Nullable<Text>,
        unparsed_data -> Nullable<Text>,
        lag -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    receivers (id) {
        id -> Int4,
        callsign -> Text,
        description -> Nullable<Text>,
        contact -> Nullable<Text>,
        email -> Nullable<Text>,
        country -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    receivers_links (id) {
        id -> Int4,
        receiver_id -> Int4,
        rel -> Nullable<Text>,
        href -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    receivers_photos (id) {
        id -> Int4,
        receiver_id -> Int4,
        photo_url -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    regions (code) {
        #[max_length = 1]
        code -> Bpchar,
        description -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;

    runways (id) {
        id -> Int4,
        airport_ref -> Int4,
        #[max_length = 16]
        airport_ident -> Varchar,
        length_ft -> Nullable<Int4>,
        width_ft -> Nullable<Int4>,
        surface -> Nullable<Text>,
        lighted -> Bool,
        closed -> Bool,
        le_ident -> Nullable<Text>,
        le_latitude_deg -> Nullable<Numeric>,
        le_longitude_deg -> Nullable<Numeric>,
        le_location -> Nullable<Geography>,
        le_elevation_ft -> Nullable<Int4>,
        le_heading_degt -> Nullable<Numeric>,
        le_displaced_threshold_ft -> Nullable<Int4>,
        he_ident -> Nullable<Text>,
        he_latitude_deg -> Nullable<Numeric>,
        he_longitude_deg -> Nullable<Numeric>,
        he_location -> Nullable<Geography>,
        he_elevation_ft -> Nullable<Int4>,
        he_heading_degt -> Nullable<Numeric>,
        he_displaced_threshold_ft -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    server_messages (id) {
        id -> Uuid,
        software -> Text,
        server_timestamp -> Timestamptz,
        received_at -> Timestamptz,
        server_name -> Text,
        server_endpoint -> Text,
        lag -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    spatial_ref_sys (srid) {
        srid -> Int4,
        #[max_length = 256]
        auth_name -> Nullable<Varchar>,
        auth_srid -> Nullable<Int4>,
        #[max_length = 2048]
        srtext -> Nullable<Varchar>,
        #[max_length = 2048]
        proj4text -> Nullable<Varchar>,
    }
}

diesel::table! {
    states (code) {
        #[max_length = 2]
        code -> Bpchar,
        name -> Text,
    }
}

diesel::table! {
    status_codes (code) {
        code -> Text,
        description -> Text,
    }
}

diesel::table! {
    type_aircraft (code) {
        #[max_length = 1]
        code -> Bpchar,
        description -> Text,
    }
}

diesel::table! {
    type_engines (code) {
        code -> Int2,
        description -> Text,
    }
}

diesel::table! {
    type_registrations (code) {
        #[max_length = 1]
        code -> Bpchar,
        description -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 255]
        first_name -> Varchar,
        #[max_length = 255]
        last_name -> Varchar,
        #[max_length = 320]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        is_admin -> Bool,
        club_id -> Nullable<Uuid>,
        email_verified -> Bool,
        #[max_length = 255]
        password_reset_token -> Nullable<Varchar>,
        password_reset_expires_at -> Nullable<Timestamptz>,
        #[max_length = 255]
        email_verification_token -> Nullable<Varchar>,
        email_verification_expires_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(aircraft_other_names -> aircraft_registrations (registration_number));
diesel::joinable!(aircraft_registrations -> airports (home_base_airport_id));
diesel::joinable!(aircraft_registrations -> clubs (club_id));
diesel::joinable!(aircraft_registrations -> devices (device_id));
diesel::joinable!(aircraft_registrations -> locations (location_id));
diesel::joinable!(aircraft_registrations -> status_codes (status_code));
diesel::joinable!(aircraft_registrations -> type_aircraft (type_aircraft_code));
diesel::joinable!(aircraft_registrations -> type_engines (type_engine_code));
diesel::joinable!(aircraft_registrations -> type_registrations (type_registration_code));
diesel::joinable!(clubs -> airports (home_base_airport_id));
diesel::joinable!(clubs -> locations (location_id));
diesel::joinable!(fixes -> clubs (club_id));
diesel::joinable!(fixes -> devices (device_id));
diesel::joinable!(fixes -> flights (flight_id));
diesel::joinable!(flights -> aircraft_registrations (tow_aircraft_id));
diesel::joinable!(flights -> clubs (club_id));
diesel::joinable!(receiver_statuses -> receivers (receiver_id));
diesel::joinable!(receivers_links -> receivers (receiver_id));
diesel::joinable!(receivers_photos -> receivers (receiver_id));
diesel::joinable!(users -> clubs (club_id));

diesel::allow_tables_to_appear_in_same_query!(
    aircraft_models,
    aircraft_other_names,
    aircraft_registrations,
    airports,
    clubs,
    countries,
    devices,
    fixes,
    flights,
    locations,
    receiver_statuses,
    receivers,
    receivers_links,
    receivers_photos,
    regions,
    runways,
    server_messages,
    spatial_ref_sys,
    states,
    status_codes,
    type_aircraft,
    type_engines,
    type_registrations,
    users,
);
