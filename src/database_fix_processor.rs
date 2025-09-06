use tracing::{error, debug};
use sqlx::PgPool;

use crate::{Fix, FixProcessor};
use crate::fixes_repo::FixesRepository;
use crate::fixes;

/// Database fix processor that saves valid fixes to the database
pub struct DatabaseFixProcessor {
    fixes_repo: FixesRepository,
}

impl DatabaseFixProcessor {
    pub fn new(pool: PgPool) -> Self {
        Self {
            fixes_repo: FixesRepository::new(pool),
        }
    }
}

impl FixProcessor for DatabaseFixProcessor {
    fn process_fix(&self, fix: Fix, raw_message: &str) {
        // Convert the position::Fix to a database Fix struct
        let db_fix = fixes::Fix::from_position_fix(&fix, raw_message.to_string());

        // Save to database asynchronously
        let fixes_repo = self.fixes_repo.clone();
        tokio::spawn(async move {
            match fixes_repo.insert(&db_fix).await {
                Ok(_) => {
                    debug!("Successfully saved fix to database for aircraft {:?}", 
                           db_fix.aircraft_id);
                }
                Err(e) => {
                    error!("Failed to save fix to database: {:?}", e);
                }
            }
        });
    }
}