use anyhow::Result;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use tracing::{info, warn};

use crate::receivers::{
    NewReceiverLinkModel, NewReceiverModel, NewReceiverPhotoModel, Receiver, ReceiverLinkModel,
    ReceiverLinkRecord, ReceiverModel, ReceiverPhotoModel, ReceiverPhotoRecord, ReceiverRecord,
    ReceiversData, UpdateReceiverModel,
};
use crate::schema::{receivers, receivers_links, receivers_photos};

type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct ReceiverRepository {
    pool: PgPool,
}

impl ReceiverRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert receivers from JSON data into the database
    /// This will insert new receivers or update existing ones based on callsign
    pub async fn upsert_receivers_from_data(&self, data: ReceiversData) -> Result<usize> {
        let receivers = data.receivers.unwrap_or_default();
        self.upsert_receivers(receivers).await
    }

    /// Upsert receivers into the database
    /// This will insert new receivers or update existing ones based on callsign
    pub async fn upsert_receivers<I>(&self, receivers: I) -> Result<usize>
    where
        I: IntoIterator<Item = Receiver>,
    {
        let receivers_vec: Vec<Receiver> = receivers.into_iter().collect();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<usize> {
            let mut conn = pool.get()?;
            let mut upserted_count = 0;

            // Use a transaction for all operations
            conn.transaction::<_, anyhow::Error, _>(|conn| {
                for receiver in receivers_vec {
                    // Skip receivers without callsign as it's our unique identifier
                    let callsign = match &receiver.callsign {
                        Some(cs) if !cs.trim().is_empty() => cs.trim(),
                        _ => {
                            warn!("Skipping receiver without callsign: {:?}", receiver);
                            continue;
                        }
                    };

                    // Insert or update the main receiver record
                    let new_receiver = NewReceiverModel {
                        callsign: callsign.to_string(),
                        description: receiver.description.clone(),
                        contact: receiver.contact.clone(),
                        email: receiver.email.clone(),
                        country: receiver.country.clone(),
                        latitude: None,
                        longitude: None,
                    };

                    let receiver_result = diesel::insert_into(receivers::table)
                        .values(&new_receiver)
                        .on_conflict(receivers::callsign)
                        .do_update()
                        .set((
                            receivers::description.eq(&new_receiver.description),
                            receivers::contact.eq(&new_receiver.contact),
                            receivers::email.eq(&new_receiver.email),
                            receivers::country.eq(&new_receiver.country),
                            receivers::updated_at.eq(Utc::now()),
                        ))
                        .returning(receivers::id)
                        .get_result::<i32>(conn);

                    let receiver_id = match receiver_result {
                        Ok(id) => id,
                        Err(e) => {
                            warn!("Failed to upsert receiver {}: {}", callsign, e);
                            continue;
                        }
                    };

                    // Delete existing photos and links for this receiver
                    let _ = diesel::delete(
                        receivers_photos::table
                            .filter(receivers_photos::receiver_id.eq(receiver_id)),
                    )
                    .execute(conn);

                    let _ = diesel::delete(
                        receivers_links::table.filter(receivers_links::receiver_id.eq(receiver_id)),
                    )
                    .execute(conn);

                    // Insert photos
                    if let Some(photos) = &receiver.photos {
                        for photo_url in photos {
                            if !photo_url.trim().is_empty() {
                                let new_photo = NewReceiverPhotoModel {
                                    receiver_id,
                                    photo_url: photo_url.trim().to_string(),
                                };

                                let photo_result = diesel::insert_into(receivers_photos::table)
                                    .values(&new_photo)
                                    .execute(conn);

                                if let Err(e) = photo_result {
                                    warn!(
                                        "Failed to insert photo for receiver {}: {}",
                                        callsign, e
                                    );
                                }
                            }
                        }
                    }

                    // Insert links
                    if let Some(links) = &receiver.links {
                        for link in links {
                            if !link.href.trim().is_empty() {
                                let rel_value = link
                                    .rel
                                    .as_ref()
                                    .map(|r| r.trim())
                                    .filter(|r| !r.is_empty())
                                    .map(|r| r.to_string());

                                let new_link = NewReceiverLinkModel {
                                    receiver_id,
                                    rel: rel_value,
                                    href: link.href.trim().to_string(),
                                };

                                let link_result = diesel::insert_into(receivers_links::table)
                                    .values(&new_link)
                                    .execute(conn);

                                if let Err(e) = link_result {
                                    warn!("Failed to insert link for receiver {}: {}", callsign, e);
                                }
                            }
                        }
                    }

                    upserted_count += 1;
                }

                Ok(())
            })?;

            info!("Successfully upserted {} receivers", upserted_count);
            Ok(upserted_count)
        })
        .await?
    }

    /// Get the total count of receivers in the database
    pub async fn get_receiver_count(&self) -> Result<i64> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<i64> {
            let mut conn = pool.get()?;
            let count: i64 = receivers::table.count().get_result(&mut conn)?;
            Ok(count)
        })
        .await?
    }

    /// Get a receiver by callsign
    pub async fn get_receiver_by_callsign(&self, callsign: &str) -> Result<Option<ReceiverRecord>> {
        let pool = self.pool.clone();
        let callsign = callsign.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<ReceiverRecord>> {
            let mut conn = pool.get()?;
            let receiver_model = receivers::table
                .filter(receivers::callsign.eq(&callsign))
                .select(ReceiverModel::as_select())
                .first::<ReceiverModel>(&mut conn)
                .optional()?;

            Ok(receiver_model.map(ReceiverRecord::from))
        })
        .await?
    }

    /// Get all photos for a receiver
    pub async fn get_receiver_photos(&self, receiver_id: i32) -> Result<Vec<ReceiverPhotoRecord>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverPhotoRecord>> {
            let mut conn = pool.get()?;
            let photo_models = receivers_photos::table
                .filter(receivers_photos::receiver_id.eq(receiver_id))
                .order(receivers_photos::id.asc())
                .select(ReceiverPhotoModel::as_select())
                .load::<ReceiverPhotoModel>(&mut conn)?;

            Ok(photo_models
                .into_iter()
                .map(ReceiverPhotoRecord::from)
                .collect())
        })
        .await?
    }

    /// Get all links for a receiver
    pub async fn get_receiver_links(&self, receiver_id: i32) -> Result<Vec<ReceiverLinkRecord>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverLinkRecord>> {
            let mut conn = pool.get()?;
            let link_models = receivers_links::table
                .filter(receivers_links::receiver_id.eq(receiver_id))
                .order(receivers_links::id.asc())
                .select(ReceiverLinkModel::as_select())
                .load::<ReceiverLinkModel>(&mut conn)?;

            Ok(link_models
                .into_iter()
                .map(ReceiverLinkRecord::from)
                .collect())
        })
        .await?
    }

    /// Get a complete receiver with photos and links
    pub async fn get_complete_receiver(
        &self,
        callsign: &str,
    ) -> Result<
        Option<(
            ReceiverRecord,
            Vec<ReceiverPhotoRecord>,
            Vec<ReceiverLinkRecord>,
        )>,
    > {
        let receiver = match self.get_receiver_by_callsign(callsign).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        let photos = self.get_receiver_photos(receiver.id).await?;
        let links = self.get_receiver_links(receiver.id).await?;

        Ok(Some((receiver, photos, links)))
    }

    /// Search receivers by callsign (case-insensitive partial match)
    pub async fn search_by_callsign(&self, callsign_param: &str) -> Result<Vec<ReceiverRecord>> {
        let pool = self.pool.clone();
        let search_pattern = format!("%{}%", callsign_param);

        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverRecord>> {
            let mut conn = pool.get()?;
            let receiver_models = receivers::table
                .filter(receivers::callsign.ilike(&search_pattern))
                .order(receivers::callsign.asc())
                .select(ReceiverModel::as_select())
                .load::<ReceiverModel>(&mut conn)?;

            Ok(receiver_models
                .into_iter()
                .map(ReceiverRecord::from)
                .collect())
        })
        .await?
    }

    /// Search receivers by country
    pub async fn search_by_country(&self, country_param: &str) -> Result<Vec<ReceiverRecord>> {
        let pool = self.pool.clone();
        let country_param = country_param.to_string();

        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverRecord>> {
            let mut conn = pool.get()?;
            let receiver_models = receivers::table
                .filter(receivers::country.eq(&country_param))
                .order(receivers::callsign.asc())
                .select(ReceiverModel::as_select())
                .load::<ReceiverModel>(&mut conn)?;

            Ok(receiver_models
                .into_iter()
                .map(ReceiverRecord::from)
                .collect())
        })
        .await?
    }

    /// Get all receivers with pagination
    pub async fn get_receivers_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ReceiverRecord>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverRecord>> {
            let mut conn = pool.get()?;
            let receiver_models = receivers::table
                .order(receivers::callsign.asc())
                .limit(limit)
                .offset(offset)
                .select(ReceiverModel::as_select())
                .load::<ReceiverModel>(&mut conn)?;

            Ok(receiver_models
                .into_iter()
                .map(ReceiverRecord::from)
                .collect())
        })
        .await?
    }

    /// Delete a receiver and all associated photos and links
    pub async fn delete_receiver(&self, callsign: &str) -> Result<bool> {
        let pool = self.pool.clone();
        let callsign = callsign.to_string();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            conn.transaction::<_, anyhow::Error, _>(|conn| {
                // Get receiver ID first
                let receiver_id_result = receivers::table
                    .filter(receivers::callsign.eq(&callsign))
                    .select(receivers::id)
                    .first::<i32>(conn)
                    .optional()?;

                let receiver_id = match receiver_id_result {
                    Some(id) => id,
                    None => return Ok(false), // Receiver not found
                };

                // Delete photos and links (will cascade due to foreign key constraints, but being explicit)
                diesel::delete(
                    receivers_photos::table.filter(receivers_photos::receiver_id.eq(receiver_id)),
                )
                .execute(conn)?;

                diesel::delete(
                    receivers_links::table.filter(receivers_links::receiver_id.eq(receiver_id)),
                )
                .execute(conn)?;

                // Delete the receiver
                let rows_affected =
                    diesel::delete(receivers::table.filter(receivers::id.eq(receiver_id)))
                        .execute(conn)?;

                Ok(rows_affected > 0)
            })
        })
        .await?
    }

    /// Update receiver position by callsign
    pub async fn update_receiver_position(
        &self,
        callsign: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<bool> {
        let pool = self.pool.clone();
        let callsign = callsign.to_string();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut conn = pool.get()?;

            let update = UpdateReceiverModel {
                latitude: Some(latitude),
                longitude: Some(longitude),
                updated_at: Utc::now(),
            };

            let rows_affected =
                diesel::update(receivers::table.filter(receivers::callsign.eq(&callsign)))
                    .set(&update)
                    .execute(&mut conn)?;

            Ok(rows_affected > 0)
        })
        .await?
    }

    /// Get a receiver by ID
    pub async fn get_receiver_by_id(&self, id: i32) -> Result<Option<ReceiverModel>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<ReceiverModel>> {
            let mut conn = pool.get()?;
            let receiver = receivers::table
                .filter(receivers::id.eq(id))
                .select(ReceiverModel::as_select())
                .first::<ReceiverModel>(&mut conn)
                .optional()?;

            Ok(receiver)
        })
        .await?
    }

    /// Get receivers in a bounding box
    pub async fn get_receivers_in_bounding_box(
        &self,
        nw_lat: f64,
        nw_lng: f64,
        se_lat: f64,
        se_lng: f64,
    ) -> Result<Vec<ReceiverModel>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverModel>> {
            let mut conn = pool.get()?;

            // Build the bounding box envelope query
            let bbox_sql = r#"
                SELECT r.*
                FROM receivers r
                WHERE r.location IS NOT NULL
                  AND ST_Intersects(
                      r.location,
                      ST_MakeEnvelope($1, $2, $3, $4, 4326)::geography
                  )
                ORDER BY r.callsign
            "#;

            #[derive(QueryableByName)]
            struct ReceiverRow {
                #[diesel(sql_type = diesel::sql_types::Int4)]
                id: i32,
                #[diesel(sql_type = diesel::sql_types::Text)]
                callsign: String,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                description: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                contact: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                email: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                country: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                created_at: chrono::DateTime<chrono::Utc>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                updated_at: chrono::DateTime<chrono::Utc>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
                latitude: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
                longitude: Option<f64>,
            }

            let receiver_rows: Vec<ReceiverRow> = diesel::sql_query(bbox_sql)
                .bind::<diesel::sql_types::Double, _>(nw_lng) // min_lon
                .bind::<diesel::sql_types::Double, _>(se_lat) // min_lat
                .bind::<diesel::sql_types::Double, _>(se_lng) // max_lon
                .bind::<diesel::sql_types::Double, _>(nw_lat) // max_lat
                .load(&mut conn)?;

            let receivers: Vec<ReceiverModel> = receiver_rows
                .into_iter()
                .map(|row| ReceiverModel {
                    id: row.id,
                    callsign: row.callsign,
                    description: row.description,
                    contact: row.contact,
                    email: row.email,
                    country: row.country,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    latitude: row.latitude,
                    longitude: row.longitude,
                })
                .collect();

            Ok(receivers)
        })
        .await?
    }

    /// Search receivers by text query (searches across callsign, description, country, contact, email)
    pub async fn search_by_query(&self, query_param: &str) -> Result<Vec<ReceiverModel>> {
        let pool = self.pool.clone();
        let search_pattern = format!("%{}%", query_param);

        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverModel>> {
            let mut conn = pool.get()?;
            let receiver_models = receivers::table
                .filter(
                    receivers::callsign
                        .ilike(&search_pattern)
                        .or(receivers::description.ilike(&search_pattern))
                        .or(receivers::country.ilike(&search_pattern))
                        .or(receivers::contact.ilike(&search_pattern))
                        .or(receivers::email.ilike(&search_pattern)),
                )
                .order(receivers::callsign.asc())
                .select(ReceiverModel::as_select())
                .load::<ReceiverModel>(&mut conn)?;

            Ok(receiver_models)
        })
        .await?
    }

    /// Get receivers within a radius (in miles) from a point
    pub async fn get_receivers_within_radius(
        &self,
        latitude: f64,
        longitude: f64,
        radius_miles: f64,
    ) -> Result<Vec<ReceiverModel>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<ReceiverModel>> {
            let mut conn = pool.get()?;

            // Convert miles to meters (PostGIS uses meters)
            let radius_meters = radius_miles * 1609.34;

            // Build the radius query using ST_DWithin
            let radius_sql = r#"
                SELECT r.*
                FROM receivers r
                WHERE r.location IS NOT NULL
                  AND ST_DWithin(
                      r.location,
                      ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography,
                      $3
                  )
                ORDER BY r.callsign
            "#;

            #[derive(QueryableByName)]
            struct ReceiverRow {
                #[diesel(sql_type = diesel::sql_types::Int4)]
                id: i32,
                #[diesel(sql_type = diesel::sql_types::Text)]
                callsign: String,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                description: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                contact: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                email: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                country: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                created_at: chrono::DateTime<chrono::Utc>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                updated_at: chrono::DateTime<chrono::Utc>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
                latitude: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float8>)]
                longitude: Option<f64>,
            }

            let receiver_rows: Vec<ReceiverRow> = diesel::sql_query(radius_sql)
                .bind::<diesel::sql_types::Double, _>(longitude)
                .bind::<diesel::sql_types::Double, _>(latitude)
                .bind::<diesel::sql_types::Double, _>(radius_meters)
                .load(&mut conn)?;

            let receivers: Vec<ReceiverModel> = receiver_rows
                .into_iter()
                .map(|row| ReceiverModel {
                    id: row.id,
                    callsign: row.callsign,
                    description: row.description,
                    contact: row.contact,
                    email: row.email,
                    country: row.country,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    latitude: row.latitude,
                    longitude: row.longitude,
                })
                .collect();

            Ok(receivers)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use crate::receivers::{Receiver, ReceiverLink};

    fn create_test_receiver() -> Receiver {
        Receiver {
            callsign: Some("TEST123".to_string()),
            description: Some("Test receiver description".to_string()),
            photos: Some(vec![
                "http://example.com/photo1.jpg".to_string(),
                "http://example.com/photo2.jpg".to_string(),
            ]),
            contact: Some("Test Contact".to_string()),
            email: Some("test@example.com".to_string()),
            links: Some(vec![
                ReceiverLink {
                    rel: Some("homepage".to_string()),
                    href: "http://example.com".to_string(),
                },
                ReceiverLink {
                    rel: Some("photo".to_string()),
                    href: "http://example.com/photo.jpg".to_string(),
                },
            ]),
            country: Some("US".to_string()),
        }
    }

    #[test]
    fn test_receiver_creation() {
        let receiver = create_test_receiver();
        assert_eq!(receiver.callsign, Some("TEST123".to_string()));
        assert_eq!(
            receiver.description,
            Some("Test receiver description".to_string())
        );
        assert_eq!(receiver.contact, Some("Test Contact".to_string()));
        assert_eq!(receiver.email, Some("test@example.com".to_string()));
        assert_eq!(receiver.country, Some("US".to_string()));

        let photos = receiver.photos.as_ref().unwrap();
        assert_eq!(photos.len(), 2);
        assert_eq!(photos[0], "http://example.com/photo1.jpg");

        let links = receiver.links.as_ref().unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].rel, Some("homepage".to_string()));
        assert_eq!(links[0].href, "http://example.com");
    }
}
