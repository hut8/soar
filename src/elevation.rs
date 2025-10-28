mod hgt;
mod service;

// Re-export main types
pub use service::{AglDatabaseTask, ElevationService, ElevationTask};

// Backwards compatibility alias - ElevationDB is now ElevationService
pub type ElevationDB = ElevationService;
