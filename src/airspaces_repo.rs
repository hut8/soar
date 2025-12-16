use anyhow::Result;
use diesel::prelude::*;
use diesel::sql_types;
use tracing::{debug, info};
use uuid::Uuid;

use crate::airspace::{AirspaceGeoJson, AirspaceProperties, NewAirspace};
use crate::web::PgPool;

#[derive(Clone)]
pub struct AirspacesRepository {
    pool: PgPool,
}

impl AirspacesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert airspaces in batches
    /// Uses raw SQL because Diesel doesn't support ST_GeomFromGeoJSON directly
    /// Pattern similar to airports_repo.rs for geometry handling
    pub async fn upsert_airspaces(
        &self,
        airspaces_list: Vec<(NewAirspace, serde_json::Value)>,
    ) -> Result<usize> {
        let pool = self.pool.clone();
        let total = airspaces_list.len();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut inserted = 0;

            // Process each airspace individually to handle geometry
            for (airspace, geojson) in airspaces_list {
                let geojson_str = serde_json::to_string(&geojson)?;

                // Use raw SQL for geometry insertion
                diesel::sql_query(
                    r#"
                    INSERT INTO airspaces (
                        openaip_id, name, airspace_class, airspace_type, country_code,
                        lower_value, lower_unit, lower_reference,
                        upper_value, upper_unit, upper_reference,
                        remarks, activity_type, openaip_updated_at, geometry
                    ) VALUES (
                        $1, $2, $3, $4, $5,
                        $6, $7, $8,
                        $9, $10, $11,
                        $12, $13, $14,
                        ST_GeomFromGeoJSON($15)::geography
                    )
                    ON CONFLICT (openaip_id) DO UPDATE SET
                        name = EXCLUDED.name,
                        airspace_class = EXCLUDED.airspace_class,
                        airspace_type = EXCLUDED.airspace_type,
                        country_code = EXCLUDED.country_code,
                        lower_value = EXCLUDED.lower_value,
                        lower_unit = EXCLUDED.lower_unit,
                        lower_reference = EXCLUDED.lower_reference,
                        upper_value = EXCLUDED.upper_value,
                        upper_unit = EXCLUDED.upper_unit,
                        upper_reference = EXCLUDED.upper_reference,
                        remarks = EXCLUDED.remarks,
                        activity_type = EXCLUDED.activity_type,
                        openaip_updated_at = EXCLUDED.openaip_updated_at,
                        geometry = EXCLUDED.geometry,
                        updated_at = NOW()
                    "#,
                )
                .bind::<sql_types::Text, _>(&airspace.openaip_id)
                .bind::<sql_types::Text, _>(&airspace.name)
                .bind::<sql_types::Nullable<crate::schema::sql_types::AirspaceClass>, _>(
                    airspace.airspace_class,
                )
                .bind::<crate::schema::sql_types::AirspaceType, _>(airspace.airspace_type)
                .bind::<sql_types::Nullable<sql_types::Text>, _>(airspace.country_code.as_deref())
                .bind::<sql_types::Nullable<sql_types::Integer>, _>(airspace.lower_value)
                .bind::<sql_types::Nullable<sql_types::Text>, _>(airspace.lower_unit.as_deref())
                .bind::<sql_types::Nullable<crate::schema::sql_types::AltitudeReference>, _>(
                    airspace.lower_reference,
                )
                .bind::<sql_types::Nullable<sql_types::Integer>, _>(airspace.upper_value)
                .bind::<sql_types::Nullable<sql_types::Text>, _>(airspace.upper_unit.as_deref())
                .bind::<sql_types::Nullable<crate::schema::sql_types::AltitudeReference>, _>(
                    airspace.upper_reference,
                )
                .bind::<sql_types::Nullable<sql_types::Text>, _>(airspace.remarks.as_deref())
                .bind::<sql_types::Nullable<sql_types::Text>, _>(airspace.activity_type.as_deref())
                .bind::<sql_types::Nullable<sql_types::Timestamptz>, _>(airspace.openaip_updated_at)
                .bind::<sql_types::Text, _>(&geojson_str)
                .execute(&mut conn)?;

                inserted += 1;

                // Log progress every 100 airspaces
                if inserted % 100 == 0 {
                    debug!("Upserted {} of {} airspaces", inserted, total);
                }
            }

            info!("Successfully upserted {} airspaces", inserted);
            Ok::<usize, anyhow::Error>(inserted)
        })
        .await?
    }

    /// Get airspaces within a bounding box for frontend display
    /// Returns GeoJSON Feature objects
    /// Uses geometry_geom with && operator for fast spatial filtering
    pub async fn get_airspaces_in_bbox(
        &self,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        limit: Option<i64>,
    ) -> Result<Vec<AirspaceGeoJson>> {
        let pool = self.pool.clone();
        let limit = limit.unwrap_or(500).min(1000); // Cap at 1000

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL to query with geometry and return GeoJSON
            #[derive(QueryableByName)]
            struct AirspaceGeoJsonRow {
                #[diesel(sql_type = sql_types::Uuid)]
                id: Uuid,
                #[diesel(sql_type = sql_types::Text)]
                openaip_id: String,
                #[diesel(sql_type = sql_types::Text)]
                name: String,
                #[diesel(sql_type = sql_types::Nullable<crate::schema::sql_types::AirspaceClass>)]
                airspace_class: Option<crate::airspace::AirspaceClass>,
                #[diesel(sql_type = crate::schema::sql_types::AirspaceType)]
                airspace_type: crate::airspace::AirspaceType,
                #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
                country_code: Option<String>,
                #[diesel(sql_type = sql_types::Nullable<sql_types::Integer>)]
                lower_value: Option<i32>,
                #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
                lower_unit: Option<String>,
                #[diesel(sql_type = sql_types::Nullable<crate::schema::sql_types::AltitudeReference>)]
                lower_reference: Option<crate::airspace::AltitudeReference>,
                #[diesel(sql_type = sql_types::Nullable<sql_types::Integer>)]
                upper_value: Option<i32>,
                #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
                upper_unit: Option<String>,
                #[diesel(sql_type = sql_types::Nullable<crate::schema::sql_types::AltitudeReference>)]
                upper_reference: Option<crate::airspace::AltitudeReference>,
                #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
                remarks: Option<String>,
                #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
                activity_type: Option<String>,
                #[diesel(sql_type = sql_types::Text)]
                geometry_geojson: String,
            }

            let results: Vec<AirspaceGeoJsonRow> = diesel::sql_query(
                r#"
                SELECT
                    id, openaip_id, name, airspace_class, airspace_type, country_code,
                    lower_value, lower_unit, lower_reference,
                    upper_value, upper_unit, upper_reference,
                    remarks, activity_type,
                    ST_AsGeoJSON(geometry)::text as geometry_geojson
                FROM airspaces
                WHERE geometry_geom && ST_MakeEnvelope($1, $2, $3, $4, 4326)
                ORDER BY lower_value NULLS LAST
                LIMIT $5
                "#,
            )
            .bind::<sql_types::Double, _>(west)
            .bind::<sql_types::Double, _>(south)
            .bind::<sql_types::Double, _>(east)
            .bind::<sql_types::Double, _>(north)
            .bind::<sql_types::BigInt, _>(limit)
            .load(&mut conn)?;

            // Convert to GeoJSON features
            let features: Vec<AirspaceGeoJson> = results
                .into_iter()
                .map(|row| {
                    let geometry: serde_json::Value =
                        serde_json::from_str(&row.geometry_geojson)
                            .unwrap_or(serde_json::json!(null));

                    AirspaceGeoJson {
                        feature_type: "Feature".to_string(),
                        geometry,
                        properties: AirspaceProperties {
                            id: row.id,
                            openaip_id: row.openaip_id,
                            name: row.name,
                            airspace_class: row.airspace_class,
                            airspace_type: row.airspace_type,
                            country_code: row.country_code,
                            lower_limit: AirspaceGeoJson::format_altitude(
                                row.lower_value,
                                row.lower_unit.as_deref(),
                                row.lower_reference,
                            ),
                            upper_limit: AirspaceGeoJson::format_altitude(
                                row.upper_value,
                                row.upper_unit.as_deref(),
                                row.upper_reference,
                            ),
                            remarks: row.remarks,
                            activity_type: row.activity_type,
                        },
                    }
                })
                .collect();

            Ok(features)
        })
        .await?
    }

    /// Get count of airspaces in database
    pub async fn count_airspaces(&self) -> Result<i64> {
        use crate::schema::airspaces;
        use diesel::dsl::count_star;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count = airspaces::table.select(count_star()).first(&mut conn)?;
            Ok(count)
        })
        .await?
    }
}
