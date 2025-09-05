use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::receivers::{
    Receiver, ReceiverLinkRecord, ReceiverPhotoRecord, ReceiverRecord, ReceiversData,
};

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
        let mut transaction = self.pool.begin().await?;
        let mut upserted_count = 0;

        for receiver in receivers {
            // Skip receivers without callsign as it's our unique identifier
            let callsign = match &receiver.callsign {
                Some(cs) if !cs.trim().is_empty() => cs.trim(),
                _ => {
                    warn!("Skipping receiver without callsign: {:?}", receiver);
                    continue;
                }
            };

            // Insert or update the main receiver record
            let receiver_result = sqlx::query!(
                r#"
                INSERT INTO receivers (callsign, description, contact, email, country)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (callsign)
                DO UPDATE SET
                    description = EXCLUDED.description,
                    contact = EXCLUDED.contact,
                    email = EXCLUDED.email,
                    country = EXCLUDED.country,
                    updated_at = NOW()
                RETURNING id
                "#,
                callsign,
                receiver.description,
                receiver.contact,
                receiver.email,
                receiver.country
            )
            .fetch_one(&mut *transaction)
            .await;

            let receiver_id = match receiver_result {
                Ok(row) => row.id,
                Err(e) => {
                    warn!("Failed to upsert receiver {}: {}", callsign, e);
                    continue;
                }
            };

            // Delete existing photos and links for this receiver
            let _ = sqlx::query!(
                "DELETE FROM receivers_photos WHERE receiver_id = $1",
                receiver_id
            )
            .execute(&mut *transaction)
            .await;

            let _ = sqlx::query!(
                "DELETE FROM receivers_links WHERE receiver_id = $1",
                receiver_id
            )
            .execute(&mut *transaction)
            .await;

            // Insert photos
            if let Some(photos) = &receiver.photos {
                for photo_url in photos {
                    if !photo_url.trim().is_empty() {
                        let photo_result = sqlx::query!(
                            "INSERT INTO receivers_photos (receiver_id, photo_url) VALUES ($1, $2)",
                            receiver_id,
                            photo_url.trim()
                        )
                        .execute(&mut *transaction)
                        .await;

                        if let Err(e) = photo_result {
                            warn!("Failed to insert photo for receiver {}: {}", callsign, e);
                        }
                    }
                }
            }

            // Insert links
            if let Some(links) = &receiver.links {
                for link in links {
                    if !link.href.trim().is_empty() {
                        let rel_value = link.rel.as_ref().map(|r| r.trim()).filter(|r| !r.is_empty());
                        let link_result = sqlx::query!(
                            "INSERT INTO receivers_links (receiver_id, rel, href) VALUES ($1, $2, $3)",
                            receiver_id,
                            rel_value,
                            link.href.trim()
                        )
                        .execute(&mut *transaction)
                        .await;

                        if let Err(e) = link_result {
                            warn!("Failed to insert link for receiver {}: {}", callsign, e);
                        }
                    }
                }
            }

            upserted_count += 1;
        }

        transaction.commit().await?;
        info!("Successfully upserted {} receivers", upserted_count);

        Ok(upserted_count)
    }

    /// Get the total count of receivers in the database
    pub async fn get_receiver_count(&self) -> Result<i64> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM receivers")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Get a receiver by callsign
    pub async fn get_receiver_by_callsign(&self, callsign: &str) -> Result<Option<ReceiverRecord>> {
        let result = sqlx::query!(
            r#"
            SELECT id, callsign, description, contact, email, country, created_at, updated_at
            FROM receivers
            WHERE callsign = $1
            "#,
            callsign
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            Ok(Some(ReceiverRecord {
                id: row.id,
                callsign: row.callsign,
                description: row.description,
                contact: row.contact,
                email: row.email,
                country: row.country,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all photos for a receiver
    pub async fn get_receiver_photos(&self, receiver_id: i32) -> Result<Vec<ReceiverPhotoRecord>> {
        let results = sqlx::query!(
            r#"
            SELECT id, receiver_id, photo_url, created_at
            FROM receivers_photos
            WHERE receiver_id = $1
            ORDER BY id
            "#,
            receiver_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut photos = Vec::new();
        for row in results {
            photos.push(ReceiverPhotoRecord {
                id: row.id,
                receiver_id: row.receiver_id,
                photo_url: row.photo_url,
                created_at: row.created_at,
            });
        }

        Ok(photos)
    }

    /// Get all links for a receiver
    pub async fn get_receiver_links(&self, receiver_id: i32) -> Result<Vec<ReceiverLinkRecord>> {
        let results = sqlx::query!(
            r#"
            SELECT id, receiver_id, rel, href, created_at
            FROM receivers_links
            WHERE receiver_id = $1
            ORDER BY id
            "#,
            receiver_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut links = Vec::new();
        for row in results {
            links.push(ReceiverLinkRecord {
                id: row.id,
                receiver_id: row.receiver_id,
                rel: row.rel,
                href: row.href,
                created_at: row.created_at,
            });
        }

        Ok(links)
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
    pub async fn search_by_callsign(&self, callsign: &str) -> Result<Vec<ReceiverRecord>> {
        let results = sqlx::query!(
            r#"
            SELECT id, callsign, description, contact, email, country, created_at, updated_at
            FROM receivers
            WHERE callsign ILIKE $1
            ORDER BY callsign
            "#,
            format!("%{}%", callsign)
        )
        .fetch_all(&self.pool)
        .await?;

        let mut receivers = Vec::new();
        for row in results {
            receivers.push(ReceiverRecord {
                id: row.id,
                callsign: row.callsign,
                description: row.description,
                contact: row.contact,
                email: row.email,
                country: row.country,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(receivers)
    }

    /// Search receivers by country
    pub async fn search_by_country(&self, country: &str) -> Result<Vec<ReceiverRecord>> {
        let results = sqlx::query!(
            r#"
            SELECT id, callsign, description, contact, email, country, created_at, updated_at
            FROM receivers
            WHERE country = $1
            ORDER BY callsign
            "#,
            country
        )
        .fetch_all(&self.pool)
        .await?;

        let mut receivers = Vec::new();
        for row in results {
            receivers.push(ReceiverRecord {
                id: row.id,
                callsign: row.callsign,
                description: row.description,
                contact: row.contact,
                email: row.email,
                country: row.country,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(receivers)
    }

    /// Get all receivers with pagination
    pub async fn get_receivers_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ReceiverRecord>> {
        let results = sqlx::query!(
            r#"
            SELECT id, callsign, description, contact, email, country, created_at, updated_at
            FROM receivers
            ORDER BY callsign
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let mut receivers = Vec::new();
        for row in results {
            receivers.push(ReceiverRecord {
                id: row.id,
                callsign: row.callsign,
                description: row.description,
                contact: row.contact,
                email: row.email,
                country: row.country,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(receivers)
    }

    /// Delete a receiver and all associated photos and links
    pub async fn delete_receiver(&self, callsign: &str) -> Result<bool> {
        let mut transaction = self.pool.begin().await?;

        // Get receiver ID first
        let receiver = sqlx::query!("SELECT id FROM receivers WHERE callsign = $1", callsign)
            .fetch_optional(&mut *transaction)
            .await?;

        let receiver_id = match receiver {
            Some(r) => r.id,
            None => return Ok(false), // Receiver not found
        };

        // Delete photos and links (will cascade due to foreign key constraints, but being explicit)
        sqlx::query!(
            "DELETE FROM receivers_photos WHERE receiver_id = $1",
            receiver_id
        )
        .execute(&mut *transaction)
        .await?;

        sqlx::query!(
            "DELETE FROM receivers_links WHERE receiver_id = $1",
            receiver_id
        )
        .execute(&mut *transaction)
        .await?;

        // Delete the receiver
        let result = sqlx::query!("DELETE FROM receivers WHERE id = $1", receiver_id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(result.rows_affected() > 0)
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
