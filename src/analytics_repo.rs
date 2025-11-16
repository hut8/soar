use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Date, Double, Integer, Nullable, Text, Timestamptz};
use uuid::Uuid;

use crate::analytics::*;
use crate::web::PgPool;

#[derive(Clone)]
pub struct AnalyticsRepository {
    pool: PgPool,
}

impl AnalyticsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get daily flight analytics for a date range
    pub async fn get_daily_flights(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<FlightAnalyticsDaily>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            #[derive(QueryableByName)]
            struct Row {
                #[diesel(sql_type = Date)]
                date: NaiveDate,
                #[diesel(sql_type = Integer)]
                flight_count: i32,
                #[diesel(sql_type = BigInt)]
                total_duration_seconds: i64,
                #[diesel(sql_type = Integer)]
                avg_duration_seconds: i32,
                #[diesel(sql_type = BigInt)]
                total_distance_meters: i64,
                #[diesel(sql_type = Integer)]
                tow_flight_count: i32,
                #[diesel(sql_type = Integer)]
                cross_country_count: i32,
                #[diesel(sql_type = Timestamptz)]
                updated_at: DateTime<Utc>,
            }

            let results = diesel::sql_query(
                "SELECT date, flight_count, total_duration_seconds, avg_duration_seconds,
                        total_distance_meters, tow_flight_count, cross_country_count, updated_at
                 FROM flight_analytics_daily
                 WHERE date >= $1 AND date <= $2
                 ORDER BY date ASC",
            )
            .bind::<Date, _>(start_date)
            .bind::<Date, _>(end_date)
            .load::<Row>(&mut conn)?;

            Ok(results
                .into_iter()
                .map(|r| FlightAnalyticsDaily {
                    date: r.date,
                    flight_count: r.flight_count,
                    total_duration_seconds: r.total_duration_seconds,
                    avg_duration_seconds: r.avg_duration_seconds,
                    total_distance_meters: r.total_distance_meters,
                    tow_flight_count: r.tow_flight_count,
                    cross_country_count: r.cross_country_count,
                    updated_at: r.updated_at,
                })
                .collect())
        })
        .await?
    }

    /// Get flight duration distribution buckets
    pub async fn get_duration_distribution(&self) -> Result<Vec<FlightDurationBucket>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            #[derive(QueryableByName)]
            struct Row {
                #[diesel(sql_type = Text)]
                bucket_name: String,
                #[diesel(sql_type = Integer)]
                bucket_order: i32,
                #[diesel(sql_type = Integer)]
                min_minutes: i32,
                #[diesel(sql_type = Nullable<Integer>)]
                max_minutes: Option<i32>,
                #[diesel(sql_type = Integer)]
                flight_count: i32,
                #[diesel(sql_type = Timestamptz)]
                updated_at: DateTime<Utc>,
            }

            let results = diesel::sql_query(
                "SELECT bucket_name, bucket_order, min_minutes, max_minutes, flight_count, updated_at
                 FROM flight_duration_buckets
                 ORDER BY bucket_order ASC",
            )
            .load::<Row>(&mut conn)?;

            Ok(results
                .into_iter()
                .map(|r| FlightDurationBucket {
                    bucket_name: r.bucket_name,
                    bucket_order: r.bucket_order,
                    min_minutes: r.min_minutes,
                    max_minutes: r.max_minutes,
                    flight_count: r.flight_count,
                    updated_at: r.updated_at,
                })
                .collect())
        })
        .await?
    }

    /// Get hourly flight analytics
    pub async fn get_hourly_flights(&self, hours: i32) -> Result<Vec<FlightAnalyticsHourly>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            #[derive(QueryableByName)]
            struct Row {
                #[diesel(sql_type = Timestamptz)]
                hour: DateTime<Utc>,
                #[diesel(sql_type = Integer)]
                flight_count: i32,
                #[diesel(sql_type = Integer)]
                active_devices: i32,
                #[diesel(sql_type = Integer)]
                active_clubs: i32,
                #[diesel(sql_type = Timestamptz)]
                updated_at: DateTime<Utc>,
            }

            let results = diesel::sql_query(
                "SELECT hour, flight_count, active_devices, active_clubs, updated_at
                 FROM flight_analytics_hourly
                 WHERE hour >= NOW() - ($1 || ' hours')::INTERVAL
                 ORDER BY hour ASC",
            )
            .bind::<Integer, _>(hours)
            .load::<Row>(&mut conn)?;

            Ok(results
                .into_iter()
                .map(|r| FlightAnalyticsHourly {
                    hour: r.hour,
                    flight_count: r.flight_count,
                    active_devices: r.active_devices,
                    active_clubs: r.active_clubs,
                    updated_at: r.updated_at,
                })
                .collect())
        })
        .await?
    }

    /// Get device outliers (z-score > threshold)
    pub async fn get_device_outliers(&self, threshold: f64) -> Result<Vec<DeviceOutlier>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            #[derive(QueryableByName)]
            struct Row {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                device_id: Uuid,
                #[diesel(sql_type = Nullable<Text>)]
                registration: Option<String>,
                #[diesel(sql_type = Nullable<Text>)]
                aircraft_model: Option<String>,
                #[diesel(sql_type = Integer)]
                flight_count_30d: i32,
                #[diesel(sql_type = Double)]
                z_score: f64,
            }

            let results = diesel::sql_query(
                "SELECT device_id, registration, aircraft_model, flight_count_30d, z_score_30d as z_score
                 FROM device_analytics
                 WHERE z_score_30d > $1
                 ORDER BY z_score_30d DESC",
            )
            .bind::<Double, _>(threshold)
            .load::<Row>(&mut conn)?;

            Ok(results
                .into_iter()
                .map(|r| DeviceOutlier {
                    device_id: r.device_id,
                    registration: r.registration,
                    aircraft_model: r.aircraft_model,
                    flight_count_30d: r.flight_count_30d,
                    z_score: r.z_score,
                })
                .collect())
        })
        .await?
    }

    /// Get top devices by flight count
    pub async fn get_top_devices(&self, limit: i32, period_days: i32) -> Result<Vec<TopDevice>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            #[derive(QueryableByName)]
            struct Row {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                device_id: Uuid,
                #[diesel(sql_type = Nullable<Text>)]
                registration: Option<String>,
                #[diesel(sql_type = Nullable<Text>)]
                aircraft_model: Option<String>,
                #[diesel(sql_type = Integer)]
                flight_count: i32,
                #[diesel(sql_type = BigInt)]
                total_distance_meters: i64,
            }

            let query = match period_days {
                7 => "SELECT device_id, registration, aircraft_model, flight_count_7d as flight_count, total_distance_meters
                      FROM device_analytics
                      WHERE flight_count_7d > 0
                      ORDER BY flight_count_7d DESC
                      LIMIT $1",
                30 => "SELECT device_id, registration, aircraft_model, flight_count_30d as flight_count, total_distance_meters
                       FROM device_analytics
                       WHERE flight_count_30d > 0
                       ORDER BY flight_count_30d DESC
                       LIMIT $1",
                _ => "SELECT device_id, registration, aircraft_model, flight_count_total as flight_count, total_distance_meters
                      FROM device_analytics
                      WHERE flight_count_total > 0
                      ORDER BY flight_count_total DESC
                      LIMIT $1",
            };

            let results = diesel::sql_query(query)
                .bind::<Integer, _>(limit)
                .load::<Row>(&mut conn)?;

            Ok(results
                .into_iter()
                .map(|r| TopDevice {
                    device_id: r.device_id,
                    registration: r.registration,
                    aircraft_model: r.aircraft_model,
                    flight_count: r.flight_count,
                    total_distance_meters: r.total_distance_meters,
                })
                .collect())
        })
        .await?
    }

    /// Get club analytics for date range
    pub async fn get_club_analytics(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        club_id: Option<Uuid>,
    ) -> Result<Vec<ClubAnalyticsDaily>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            #[derive(QueryableByName)]
            struct Row {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                club_id: Uuid,
                #[diesel(sql_type = Date)]
                date: NaiveDate,
                #[diesel(sql_type = Nullable<Text>)]
                club_name: Option<String>,
                #[diesel(sql_type = Integer)]
                flight_count: i32,
                #[diesel(sql_type = Integer)]
                active_devices: i32,
                #[diesel(sql_type = BigInt)]
                total_airtime_seconds: i64,
                #[diesel(sql_type = Integer)]
                tow_count: i32,
                #[diesel(sql_type = Timestamptz)]
                updated_at: DateTime<Utc>,
            }

            let results = if let Some(cid) = club_id {
                diesel::sql_query(
                    "SELECT club_id, date, club_name, flight_count, active_devices, total_airtime_seconds, tow_count, updated_at
                     FROM club_analytics_daily
                     WHERE date >= $1 AND date <= $2 AND club_id = $3
                     ORDER BY date ASC",
                )
                .bind::<Date, _>(start_date)
                .bind::<Date, _>(end_date)
                .bind::<diesel::sql_types::Uuid, _>(cid)
                .load::<Row>(&mut conn)?
            } else {
                diesel::sql_query(
                    "SELECT club_id, date, club_name, flight_count, active_devices, total_airtime_seconds, tow_count, updated_at
                     FROM club_analytics_daily
                     WHERE date >= $1 AND date <= $2
                     ORDER BY date ASC",
                )
                .bind::<Date, _>(start_date)
                .bind::<Date, _>(end_date)
                .load::<Row>(&mut conn)?
            };

            Ok(results
                .into_iter()
                .map(|r| ClubAnalyticsDaily {
                    club_id: r.club_id,
                    date: r.date,
                    club_name: r.club_name,
                    flight_count: r.flight_count,
                    active_devices: r.active_devices,
                    total_airtime_seconds: r.total_airtime_seconds,
                    tow_count: r.tow_count,
                    updated_at: r.updated_at,
                })
                .collect())
        })
        .await?
    }

    /// Get airport activity for date range
    pub async fn get_airport_activity(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        limit: i32,
    ) -> Result<Vec<AirportActivity>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            #[derive(QueryableByName)]
            struct Row {
                #[diesel(sql_type = Integer)]
                airport_id: i32,
                #[diesel(sql_type = Nullable<Text>)]
                ident: Option<String>,
                #[diesel(sql_type = Nullable<Text>)]
                name: Option<String>,
                #[diesel(sql_type = BigInt)]
                departure_count: i64,
                #[diesel(sql_type = BigInt)]
                arrival_count: i64,
            }

            let results = diesel::sql_query(
                "SELECT airport_id,
                        MAX(airport_ident) as ident,
                        MAX(airport_name) as name,
                        SUM(departure_count) as departure_count,
                        SUM(arrival_count) as arrival_count
                 FROM airport_analytics_daily
                 WHERE date >= $1 AND date <= $2
                 GROUP BY airport_id
                 ORDER BY (SUM(departure_count) + SUM(arrival_count)) DESC
                 LIMIT $3",
            )
            .bind::<Date, _>(start_date)
            .bind::<Date, _>(end_date)
            .bind::<Integer, _>(limit)
            .load::<Row>(&mut conn)?;

            Ok(results
                .into_iter()
                .map(|r| AirportActivity {
                    airport_id: r.airport_id,
                    ident: r.ident,
                    name: r.name,
                    departure_count: r.departure_count,
                    arrival_count: r.arrival_count,
                })
                .collect())
        })
        .await?
    }

    /// Get analytics summary for dashboard
    pub async fn get_summary(&self) -> Result<AnalyticsSummary> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            #[derive(QueryableByName)]
            struct Row {
                #[diesel(sql_type = Integer)]
                flights_today: i32,
                #[diesel(sql_type = Integer)]
                flights_7d: i32,
                #[diesel(sql_type = Integer)]
                flights_30d: i32,
                #[diesel(sql_type = Integer)]
                active_devices_7d: i32,
                #[diesel(sql_type = Integer)]
                outlier_devices_count: i32,
                #[diesel(sql_type = Nullable<Double>)]
                data_quality_score: Option<f64>,
            }

            let result = diesel::sql_query(
                "SELECT
                    COALESCE((SELECT flight_count FROM flight_analytics_daily WHERE date = CURRENT_DATE), 0) as flights_today,
                    COALESCE((SELECT SUM(flight_count) FROM flight_analytics_daily WHERE date >= CURRENT_DATE - 7), 0) as flights_7d,
                    COALESCE((SELECT SUM(flight_count) FROM flight_analytics_daily WHERE date >= CURRENT_DATE - 30), 0) as flights_30d,
                    COALESCE((SELECT COUNT(*) FROM device_analytics WHERE flight_count_7d > 0), 0) as active_devices_7d,
                    COALESCE((SELECT COUNT(*) FROM device_analytics WHERE z_score_30d > 3.0), 0) as outlier_devices_count,
                    (SELECT quality_score FROM data_quality_metrics_daily ORDER BY metric_date DESC LIMIT 1) as data_quality_score
                ",
            )
            .get_result::<Row>(&mut conn)?;

            Ok(AnalyticsSummary {
                flights_today: result.flights_today,
                flights_7d: result.flights_7d,
                flights_30d: result.flights_30d,
                active_devices_7d: result.active_devices_7d,
                outlier_devices_count: result.outlier_devices_count,
                data_quality_score: result.data_quality_score,
            })
        })
        .await?
    }
}
