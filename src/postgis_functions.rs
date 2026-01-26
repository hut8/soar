//! PostGIS spatial functions for use with Diesel queries
//!
//! This module defines PostGIS spatial functions that aren't provided by postgis_diesel
//! so they can be used in type-safe Diesel query builder expressions.

use diesel::define_sql_function;

// Re-export functions from postgis_diesel that we use
pub use postgis_diesel::functions::st_intersects;

// Define PostGIS functions not provided by postgis_diesel

define_sql_function! {
    /// Creates a rectangular Polygon from the minimum and maximum values for X and Y coordinates.
    /// Input values must be in the spatial reference system specified by the SRID.
    ///
    /// # Arguments
    /// * `xmin` - Minimum X coordinate (longitude for SRID 4326)
    /// * `ymin` - Minimum Y coordinate (latitude for SRID 4326)
    /// * `xmax` - Maximum X coordinate (longitude for SRID 4326)
    /// * `ymax` - Maximum Y coordinate (latitude for SRID 4326)
    /// * `srid` - Spatial reference system identifier (e.g., 4326 for WGS 84)
    ///
    /// # Returns
    /// A geometry representing the bounding box envelope
    #[sql_name = "ST_MakeEnvelope"]
    fn st_make_envelope(
        xmin: diesel::sql_types::Double,
        ymin: diesel::sql_types::Double,
        xmax: diesel::sql_types::Double,
        ymax: diesel::sql_types::Double,
        srid: diesel::sql_types::Integer
    ) -> diesel::sql_types::Nullable<postgis_diesel::sql_types::Geometry>;
}

define_sql_function! {
    /// Creates a Point geometry from X and Y coordinates.
    ///
    /// # Arguments
    /// * `x` - X coordinate (longitude for geographic coordinates)
    /// * `y` - Y coordinate (latitude for geographic coordinates)
    ///
    /// # Returns
    /// A Point geometry
    #[sql_name = "ST_MakePoint"]
    fn st_make_point(
        x: diesel::sql_types::Double,
        y: diesel::sql_types::Double
    ) -> postgis_diesel::sql_types::Geometry;
}

define_sql_function! {
    /// Sets the SRID (Spatial Reference System Identifier) on a geometry.
    ///
    /// # Arguments
    /// * `geom` - The geometry to set the SRID on
    /// * `srid` - The SRID value to set
    ///
    /// # Returns
    /// The geometry with the specified SRID
    #[sql_name = "ST_SetSRID"]
    fn st_set_srid(
        geom: postgis_diesel::sql_types::Geometry,
        srid: diesel::sql_types::Integer
    ) -> postgis_diesel::sql_types::Geometry;
}

define_sql_function! {
    /// Returns the X coordinate (longitude) of a Point geometry.
    ///
    /// # Arguments
    /// * `geom` - A Point geometry
    ///
    /// # Returns
    /// The X coordinate as a double precision value
    #[sql_name = "ST_X"]
    fn st_x(geom: postgis_diesel::sql_types::Geometry) -> diesel::sql_types::Double;
}

define_sql_function! {
    /// Returns the Y coordinate (latitude) of a Point geometry.
    ///
    /// # Arguments
    /// * `geom` - A Point geometry
    ///
    /// # Returns
    /// The Y coordinate as a double precision value
    #[sql_name = "ST_Y"]
    fn st_y(geom: postgis_diesel::sql_types::Geometry) -> diesel::sql_types::Double;
}
