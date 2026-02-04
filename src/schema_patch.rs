//! Manual Diesel schema extensions that survive `diesel print-schema` regeneration.
//!
//! `schema.rs` is auto-generated — any manual additions there will be lost.
//! Place join declarations, group-by allowances, and other manual schema
//! configuration here instead.

use crate::schema::{aircraft, spurious_flights};

// No foreign key constraint exists in the database for spurious_flights.aircraft_id,
// so diesel print-schema won't generate this automatically.
diesel::joinable!(spurious_flights -> aircraft (aircraft_id));

// Required for Diesel's type-level GROUP BY validation when columns from
// spurious_flights and aircraft appear together in a group_by clause.
diesel::allow_columns_to_appear_in_same_group_by_clause!(
    spurious_flights::aircraft_id,
    spurious_flights::device_address,
    aircraft::registration,
    aircraft::aircraft_model,
);
