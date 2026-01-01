mod hgt;
mod service;

// Re-export main types
pub use service::ElevationService;

// Backwards compatibility alias - ElevationDB is now ElevationService
pub type ElevationDB = ElevationService;
