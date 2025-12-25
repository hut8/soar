// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "address_type"))]
    pub struct AddressType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "adsb_emitter_category"))]
    pub struct AdsbEmitterCategory;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "aircraft_category"))]
    pub struct AircraftCategory;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "aircraft_category_adsb"))]
    pub struct AircraftCategoryAdsb;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "aircraft_type"))]
    pub struct AircraftType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "aircraft_type_ogn"))]
    pub struct AircraftTypeOgn;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "airspace_class"))]
    pub struct AirspaceClass;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "airspace_type"))]
    pub struct AirspaceType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "airworthiness_class"))]
    pub struct AirworthinessClass;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "altitude_reference"))]
    pub struct AltitudeReference;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "engine_type"))]
    pub struct EngineType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "engine_type_adsb"))]
    pub struct EngineTypeAdsb;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "geography"))]
    pub struct Geography;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "geometry"))]
    pub struct Geometry;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "light_sport_type"))]
    pub struct LightSportType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "message_source"))]
    pub struct MessageSource;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "point", schema = "pg_catalog"))]
    pub struct Point;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "registrant_type"))]
    pub struct RegistrantType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "timeout_phase"))]
    pub struct TimeoutPhase;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AddressType;
    use super::sql_types::AircraftTypeOgn;
    use super::sql_types::AdsbEmitterCategory;
    use super::sql_types::Geometry;
    use super::sql_types::Geography;
    use super::sql_types::AircraftCategoryAdsb;
    use super::sql_types::EngineTypeAdsb;
    use super::sql_types::AircraftCategory;
    use super::sql_types::EngineType;

    aircraft (id) {
        address -> Int4,
        address_type -> AddressType,
        aircraft_model -> Text,
        registration -> Nullable<Text>,
        competition_number -> Text,
        tracked -> Bool,
        identified -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        id -> Uuid,
        from_ogn_ddb -> Bool,
        frequency_mhz -> Nullable<Numeric>,
        pilot_name -> Nullable<Text>,
        home_base_airport_ident -> Nullable<Text>,
        aircraft_type_ogn -> Nullable<AircraftTypeOgn>,
        last_fix_at -> Nullable<Timestamptz>,
        club_id -> Nullable<Uuid>,
        #[max_length = 4]
        icao_model_code -> Nullable<Varchar>,
        adsb_emitter_category -> Nullable<AdsbEmitterCategory>,
        tracker_device_type -> Nullable<Text>,
        #[max_length = 2]
        country_code -> Nullable<Bpchar>,
        latitude -> Nullable<Float8>,
        longitude -> Nullable<Float8>,
        location_geom -> Nullable<Geometry>,
        location_geog -> Nullable<Geography>,
        aircraft_category_adsb -> Nullable<AircraftCategoryAdsb>,
        num_engines -> Nullable<Int2>,
        engine_type_adsb -> Nullable<EngineTypeAdsb>,
        aircraft_category -> Nullable<AircraftCategory>,
        engine_count -> Nullable<Int2>,
        engine_type -> Nullable<EngineType>,
        faa_pia -> Nullable<Bool>,
        faa_ladd -> Nullable<Bool>,
        owner_operator -> Nullable<Text>,
        from_adsbx_ddb -> Bool,
        year -> Nullable<Int2>,
        is_military -> Nullable<Bool>,
        current_fix -> Nullable<Jsonb>,
    }
}

diesel::table! {
    aircraft_analytics (aircraft_id) {
        aircraft_id -> Uuid,
        registration -> Nullable<Varchar>,
        aircraft_model -> Nullable<Varchar>,
        flight_count_total -> Int4,
        flight_count_30d -> Int4,
        flight_count_7d -> Int4,
        last_flight_at -> Nullable<Timestamptz>,
        avg_flight_duration_seconds -> Int4,
        total_distance_meters -> Int8,
        z_score_30d -> Nullable<Numeric>,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    aircraft_approved_operations (id) {
        id -> Uuid,
        #[max_length = 6]
        aircraft_registration_id -> Varchar,
        operation -> Varchar,
        created_at -> Timestamptz,
    }
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
        #[max_length = 7]
        registration_number -> Varchar,
        seq -> Int2,
        other_name -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AirworthinessClass;
    use super::sql_types::RegistrantType;
    use super::sql_types::LightSportType;
    use super::sql_types::AircraftType;

    aircraft_registrations (registration_number) {
        #[max_length = 6]
        registration_number -> Varchar,
        #[max_length = 30]
        serial_number -> Varchar,
        year_mfr -> Nullable<Int4>,
        #[max_length = 50]
        registrant_name -> Nullable<Varchar>,
        last_action_date -> Nullable<Date>,
        certificate_issue_date -> Nullable<Date>,
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
        home_base_airport_id -> Nullable<Int4>,
        location_id -> Nullable<Uuid>,
        airworthiness_class -> Nullable<AirworthinessClass>,
        aircraft_id -> Nullable<Uuid>,
        #[max_length = 3]
        manufacturer_code -> Varchar,
        #[max_length = 2]
        model_code -> Varchar,
        #[max_length = 2]
        series_code -> Varchar,
        #[max_length = 3]
        engine_manufacturer_code -> Nullable<Varchar>,
        #[max_length = 2]
        engine_model_code -> Nullable<Varchar>,
        registrant_type_code -> Nullable<RegistrantType>,
        light_sport_type -> Nullable<LightSportType>,
        aircraft_type -> Nullable<AircraftType>,
        club_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    aircraft_types (icao_code) {
        icao_code -> Text,
        iata_code -> Nullable<Text>,
        description -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    airport_analytics_daily (airport_id, date) {
        airport_id -> Int4,
        date -> Date,
        airport_ident -> Nullable<Varchar>,
        airport_name -> Nullable<Varchar>,
        departure_count -> Int4,
        arrival_count -> Int4,
        updated_at -> Timestamptz,
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
        location_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    airspace_sync_log (id) {
        id -> Uuid,
        started_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
        success -> Nullable<Bool>,
        airspaces_fetched -> Nullable<Int4>,
        airspaces_inserted -> Nullable<Int4>,
        airspaces_updated -> Nullable<Int4>,
        error_message -> Nullable<Text>,
        countries_filter -> Nullable<Array<Nullable<Text>>>,
        updated_after -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AirspaceClass;
    use super::sql_types::AirspaceType;
    use super::sql_types::AltitudeReference;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    airspaces (id) {
        id -> Uuid,
        openaip_id -> Text,
        name -> Text,
        airspace_class -> Nullable<AirspaceClass>,
        airspace_type -> AirspaceType,
        #[max_length = 2]
        country_code -> Nullable<Bpchar>,
        lower_value -> Nullable<Int4>,
        lower_unit -> Nullable<Text>,
        lower_reference -> Nullable<AltitudeReference>,
        upper_value -> Nullable<Int4>,
        upper_unit -> Nullable<Text>,
        upper_reference -> Nullable<AltitudeReference>,
        geometry -> Geography,
        geometry_geom -> Nullable<Geometry>,
        remarks -> Nullable<Text>,
        activity_type -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        openaip_updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    club_analytics_daily (club_id, date) {
        club_id -> Uuid,
        date -> Date,
        club_name -> Nullable<Varchar>,
        flight_count -> Int4,
        active_devices -> Int4,
        total_airtime_seconds -> Int8,
        tow_count -> Int4,
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
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_default (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_old (id) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        snr_db -> Nullable<Float4>,
        bit_errors_corrected -> Nullable<Int4>,
        freq_offset_khz -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        gnss_horizontal_resolution -> Nullable<Int2>,
        gnss_vertical_resolution -> Nullable<Int2>,
        aprs_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_p20251211 (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_p20251212 (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_p20251213 (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_p20251214 (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_p20251215 (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_p20251216 (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;
    use super::sql_types::Geometry;

    fixes_p20251217 (id, received_at) {
        id -> Uuid,
        #[max_length = 9]
        source -> Varchar,
        #[max_length = 9]
        aprs_type -> Varchar,
        via -> Array<Nullable<Text>>,
        timestamp -> Timestamptz,
        latitude -> Float8,
        longitude -> Float8,
        location -> Nullable<Geography>,
        altitude_msl_feet -> Nullable<Int4>,
        #[max_length = 20]
        flight_number -> Nullable<Varchar>,
        #[max_length = 4]
        squawk -> Nullable<Varchar>,
        ground_speed_knots -> Nullable<Float4>,
        track_degrees -> Nullable<Float4>,
        climb_fpm -> Nullable<Int4>,
        turn_rate_rot -> Nullable<Float4>,
        flight_id -> Nullable<Uuid>,
        aircraft_id -> Uuid,
        received_at -> Timestamptz,
        is_active -> Bool,
        altitude_agl_feet -> Nullable<Int4>,
        receiver_id -> Uuid,
        raw_message_id -> Uuid,
        altitude_agl_valid -> Bool,
        location_geom -> Nullable<Geometry>,
        time_gap_seconds -> Nullable<Int4>,
        source_metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    flight_analytics_daily (date) {
        date -> Date,
        flight_count -> Int4,
        total_duration_seconds -> Int8,
        avg_duration_seconds -> Int4,
        total_distance_meters -> Int8,
        tow_flight_count -> Int4,
        cross_country_count -> Int4,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    flight_analytics_hourly (hour) {
        hour -> Timestamptz,
        flight_count -> Int4,
        active_devices -> Int4,
        active_clubs -> Int4,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    flight_duration_buckets (bucket_name) {
        #[max_length = 20]
        bucket_name -> Varchar,
        bucket_order -> Int4,
        min_minutes -> Int4,
        max_minutes -> Nullable<Int4>,
        flight_count -> Int4,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    flight_pilots (id) {
        id -> Uuid,
        flight_id -> Uuid,
        user_id -> Uuid,
        is_tow_pilot -> Bool,
        is_student -> Bool,
        is_instructor -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AddressType;
    use super::sql_types::TimeoutPhase;

    flights (id) {
        id -> Uuid,
        #[max_length = 20]
        device_address -> Varchar,
        takeoff_time -> Nullable<Timestamptz>,
        landing_time -> Nullable<Timestamptz>,
        club_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        device_address_type -> AddressType,
        aircraft_id -> Nullable<Uuid>,
        takeoff_altitude_offset_ft -> Nullable<Int4>,
        landing_altitude_offset_ft -> Nullable<Int4>,
        takeoff_runway_ident -> Nullable<Text>,
        landing_runway_ident -> Nullable<Text>,
        total_distance_meters -> Nullable<Float8>,
        maximum_displacement_meters -> Nullable<Float8>,
        departure_airport_id -> Nullable<Int4>,
        arrival_airport_id -> Nullable<Int4>,
        towed_by_aircraft_id -> Nullable<Uuid>,
        towed_by_flight_id -> Nullable<Uuid>,
        tow_release_altitude_msl_ft -> Nullable<Int4>,
        tow_release_time -> Nullable<Timestamptz>,
        runways_inferred -> Nullable<Bool>,
        takeoff_location_id -> Nullable<Uuid>,
        landing_location_id -> Nullable<Uuid>,
        timed_out_at -> Nullable<Timestamptz>,
        last_fix_at -> Timestamptz,
        callsign -> Nullable<Text>,
        tow_release_height_delta_ft -> Nullable<Int4>,
        min_latitude -> Nullable<Float8>,
        max_latitude -> Nullable<Float8>,
        min_longitude -> Nullable<Float8>,
        max_longitude -> Nullable<Float8>,
        timeout_phase -> Nullable<TimeoutPhase>,
        start_location_id -> Nullable<Uuid>,
        end_location_id -> Nullable<Uuid>,
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
        country_code -> Nullable<Text>,
        geolocation -> Nullable<Point>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages_default (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages_p20251211 (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages_p20251212 (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages_p20251213 (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages_p20251214 (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages_p20251215 (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages_p20251216 (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::MessageSource;

    raw_messages_p20251217 (id, received_at) {
        id -> Uuid,
        received_at -> Timestamptz,
        receiver_id -> Uuid,
        unparsed -> Nullable<Text>,
        raw_message_hash -> Bytea,
        raw_message -> Bytea,
        source -> MessageSource,
    }
}

diesel::table! {
    receiver_statuses (id) {
        id -> Uuid,
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
        receiver_id -> Uuid,
        raw_message_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Geography;

    receivers (id) {
        callsign -> Text,
        description -> Nullable<Text>,
        contact -> Nullable<Text>,
        email -> Nullable<Text>,
        ogn_db_country -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        id -> Uuid,
        latest_packet_at -> Nullable<Timestamptz>,
        from_ogn_db -> Bool,
        location -> Nullable<Geography>,
        latitude -> Nullable<Float8>,
        longitude -> Nullable<Float8>,
        street_address -> Nullable<Text>,
        city -> Nullable<Text>,
        region -> Nullable<Text>,
        country -> Nullable<Text>,
        postal_code -> Nullable<Text>,
        geocoded -> Bool,
    }
}

diesel::table! {
    receivers_links (id) {
        id -> Int4,
        rel -> Nullable<Text>,
        href -> Text,
        created_at -> Timestamptz,
        receiver_id -> Uuid,
    }
}

diesel::table! {
    receivers_photos (id) {
        id -> Int4,
        photo_url -> Text,
        created_at -> Timestamptz,
        receiver_id -> Uuid,
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
    use diesel::sql_types::*;
    use super::sql_types::Geometry;
    use super::sql_types::Geography;

    user_fixes (id) {
        id -> Uuid,
        user_id -> Uuid,
        latitude -> Float8,
        longitude -> Float8,
        heading -> Nullable<Float8>,
        location_geom -> Nullable<Geometry>,
        location_geog -> Nullable<Geography>,
        raw -> Nullable<Jsonb>,
        timestamp -> Timestamptz,
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
        email -> Nullable<Varchar>,
        #[max_length = 255]
        password_hash -> Nullable<Varchar>,
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
        settings -> Jsonb,
        is_licensed -> Bool,
        is_instructor -> Bool,
        is_tow_pilot -> Bool,
        is_examiner -> Bool,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    watchlist (user_id, aircraft_id) {
        user_id -> Uuid,
        aircraft_id -> Uuid,
        send_email -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(aircraft -> clubs (club_id));
diesel::joinable!(aircraft_approved_operations -> aircraft_registrations (aircraft_registration_id));
diesel::joinable!(aircraft_other_names -> aircraft_registrations (registration_number));
diesel::joinable!(aircraft_registrations -> aircraft (aircraft_id));
diesel::joinable!(aircraft_registrations -> airports (home_base_airport_id));
diesel::joinable!(aircraft_registrations -> clubs (club_id));
diesel::joinable!(aircraft_registrations -> locations (location_id));
diesel::joinable!(aircraft_registrations -> status_codes (status_code));
diesel::joinable!(aircraft_registrations -> type_engines (type_engine_code));
diesel::joinable!(airports -> locations (location_id));
diesel::joinable!(clubs -> airports (home_base_airport_id));
diesel::joinable!(clubs -> locations (location_id));
diesel::joinable!(fixes -> aircraft (aircraft_id));
diesel::joinable!(fixes -> flights (flight_id));
diesel::joinable!(fixes -> receivers (receiver_id));
diesel::joinable!(fixes_default -> aircraft (aircraft_id));
diesel::joinable!(fixes_default -> flights (flight_id));
diesel::joinable!(fixes_default -> receivers (receiver_id));
diesel::joinable!(fixes_old -> aircraft (aircraft_id));
diesel::joinable!(fixes_old -> flights (flight_id));
diesel::joinable!(fixes_old -> receivers (receiver_id));
diesel::joinable!(fixes_p20251211 -> aircraft (aircraft_id));
diesel::joinable!(fixes_p20251211 -> flights (flight_id));
diesel::joinable!(fixes_p20251211 -> receivers (receiver_id));
diesel::joinable!(fixes_p20251212 -> aircraft (aircraft_id));
diesel::joinable!(fixes_p20251212 -> flights (flight_id));
diesel::joinable!(fixes_p20251212 -> receivers (receiver_id));
diesel::joinable!(fixes_p20251213 -> aircraft (aircraft_id));
diesel::joinable!(fixes_p20251213 -> flights (flight_id));
diesel::joinable!(fixes_p20251213 -> receivers (receiver_id));
diesel::joinable!(fixes_p20251214 -> aircraft (aircraft_id));
diesel::joinable!(fixes_p20251214 -> flights (flight_id));
diesel::joinable!(fixes_p20251214 -> receivers (receiver_id));
diesel::joinable!(fixes_p20251215 -> aircraft (aircraft_id));
diesel::joinable!(fixes_p20251215 -> flights (flight_id));
diesel::joinable!(fixes_p20251215 -> receivers (receiver_id));
diesel::joinable!(fixes_p20251216 -> aircraft (aircraft_id));
diesel::joinable!(fixes_p20251216 -> flights (flight_id));
diesel::joinable!(fixes_p20251216 -> receivers (receiver_id));
diesel::joinable!(fixes_p20251217 -> aircraft (aircraft_id));
diesel::joinable!(fixes_p20251217 -> flights (flight_id));
diesel::joinable!(fixes_p20251217 -> receivers (receiver_id));
diesel::joinable!(flight_pilots -> flights (flight_id));
diesel::joinable!(flight_pilots -> users (user_id));
diesel::joinable!(flights -> clubs (club_id));
diesel::joinable!(raw_messages -> receivers (receiver_id));
diesel::joinable!(raw_messages_default -> receivers (receiver_id));
diesel::joinable!(raw_messages_p20251211 -> receivers (receiver_id));
diesel::joinable!(raw_messages_p20251212 -> receivers (receiver_id));
diesel::joinable!(raw_messages_p20251213 -> receivers (receiver_id));
diesel::joinable!(raw_messages_p20251214 -> receivers (receiver_id));
diesel::joinable!(raw_messages_p20251215 -> receivers (receiver_id));
diesel::joinable!(raw_messages_p20251216 -> receivers (receiver_id));
diesel::joinable!(raw_messages_p20251217 -> receivers (receiver_id));
diesel::joinable!(receiver_statuses -> receivers (receiver_id));
diesel::joinable!(receivers_links -> receivers (receiver_id));
diesel::joinable!(receivers_photos -> receivers (receiver_id));
diesel::joinable!(user_fixes -> users (user_id));
diesel::joinable!(users -> clubs (club_id));
diesel::joinable!(watchlist -> aircraft (aircraft_id));
diesel::joinable!(watchlist -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    aircraft,
    aircraft_analytics,
    aircraft_approved_operations,
    aircraft_models,
    aircraft_other_names,
    aircraft_registrations,
    aircraft_types,
    airport_analytics_daily,
    airports,
    airspace_sync_log,
    airspaces,
    club_analytics_daily,
    clubs,
    countries,
    fixes,
    fixes_default,
    fixes_old,
    fixes_p20251211,
    fixes_p20251212,
    fixes_p20251213,
    fixes_p20251214,
    fixes_p20251215,
    fixes_p20251216,
    fixes_p20251217,
    flight_analytics_daily,
    flight_analytics_hourly,
    flight_duration_buckets,
    flight_pilots,
    flights,
    locations,
    raw_messages,
    raw_messages_default,
    raw_messages_p20251211,
    raw_messages_p20251212,
    raw_messages_p20251213,
    raw_messages_p20251214,
    raw_messages_p20251215,
    raw_messages_p20251216,
    raw_messages_p20251217,
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
    user_fixes,
    users,
    watchlist,
);
