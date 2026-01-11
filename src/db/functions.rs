//! PostGIS SQL function definitions for use with Diesel's query builder.
//!
//! These allow type-safe spatial queries instead of raw SQL.
//!
//! Note: We use `#[allow(non_snake_case)]` because PostGIS function names
//! use PascalCase (e.g., `ST_MakeEnvelope`) and we want to match them exactly.

#![allow(non_snake_case)]

use diesel::sql_types::{Double, Integer, Nullable};
use postgis_diesel::sql_types::Geometry;

diesel::define_sql_function! {
    /// Creates a rectangular Polygon from minimum and maximum coordinates.
    /// The SRID should typically be 4326 for WGS84 lat/lon coordinates.
    fn ST_MakeEnvelope(
        xmin: Double,
        ymin: Double,
        xmax: Double,
        ymax: Double,
        srid: Integer
    ) -> Geometry;
}

diesel::define_sql_function! {
    /// Returns true if two geometries intersect (share any portion of space).
    fn ST_Intersects(a: Geometry, b: Geometry) -> Bool;
}

diesel::define_sql_function! {
    /// Returns true if geometries are within the specified distance of one another.
    /// For geometry types, distance is in the units of the spatial reference system.
    fn ST_DWithin(a: Geometry, b: Geometry, distance: Double) -> Bool;
}

diesel::define_sql_function! {
    /// Returns the 2D Cartesian distance between two geometries.
    /// For geometry types, distance is in the units of the spatial reference system.
    fn ST_Distance(a: Geometry, b: Geometry) -> Double;
}

diesel::define_sql_function! {
    /// Creates a Point geometry from X and Y coordinates.
    fn ST_MakePoint(x: Double, y: Double) -> Geometry;
}

diesel::define_sql_function! {
    /// Sets the SRID on a geometry to a particular integer value.
    fn ST_SetSRID(geom: Geometry, srid: Integer) -> Geometry;
}

// Nullable variants for use with nullable geometry columns
diesel::define_sql_function! {
    /// ST_Intersects variant that accepts nullable geometry.
    #[sql_name = "ST_Intersects"]
    fn ST_Intersects_nullable(a: Nullable<Geometry>, b: Geometry) -> Nullable<Bool>;
}
