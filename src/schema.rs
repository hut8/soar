// @generated automatically by Diesel CLI.

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