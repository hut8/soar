use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use std::io::Read;
use std::path::Path;
use tokio::fs;

/// HGT (Height) tile representing SRTM elevation data
/// Supports both 1 arcsecond (3601x3601) and 3 arcsecond (1201x1201) resolutions
#[derive(Debug, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub struct HGT {
    buffer: Vec<u8>,
    sw_lat_lng: (f64, f64),
    size: usize,
}

impl HGT {
    /// Create a new HGT tile from decompressed buffer
    ///
    /// Buffer sizes:
    /// - 25,934,402 bytes = 3601 x 3601 x 2 bytes (1 arcsecond resolution)
    /// - 2,884,802 bytes = 1201 x 1201 x 2 bytes (3 arcsecond resolution)
    pub fn new(buffer: Vec<u8>, sw_lat_lng: (f64, f64)) -> Result<Self> {
        let size = match buffer.len() {
            25934402 => 3601, // 1 arcsecond
            2884802 => 1201,  // 3 arcsecond
            _ => bail!(
                "Unknown HGT tile format (expected 1 or 3 arcsecond resolution, got {} bytes)",
                buffer.len()
            ),
        };

        Ok(Self {
            buffer,
            sw_lat_lng,
            size,
        })
    }

    /// Load HGT tile from gzipped file
    pub async fn from_file(path: &Path, sw_lat_lng: (f64, f64)) -> Result<Self> {
        let compressed = fs::read(path)
            .await
            .with_context(|| format!("Failed to read HGT file: {:?}", path))?;

        // Decompress
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut buffer = Vec::new();
        decoder
            .read_to_end(&mut buffer)
            .context("Failed to decompress HGT file")?;

        Self::new(buffer, sw_lat_lng)
    }

    /// Get elevation at specific lat/lng coordinates using bilinear interpolation
    /// Returns elevation in meters
    pub fn get_elevation(&self, lat: f64, lng: f64) -> Result<i16> {
        let size = self.size - 1;
        let row = (lat - self.sw_lat_lng.0) * size as f64;
        let col = (lng - self.sw_lat_lng.1) * size as f64;

        if row < 0.0 || col < 0.0 || row > size as f64 || col > size as f64 {
            bail!(
                "Latitude/longitude ({}, {}) is outside tile bounds (SW: {:?}, row={}, col={})",
                lat,
                lng,
                self.sw_lat_lng,
                row,
                col
            );
        }

        self.interpolate(row, col)
    }

    /// Bilinear interpolation between the 4 nearest elevation points
    fn interpolate(&self, row: f64, col: f64) -> Result<i16> {
        let row_low = row.floor();
        let row_high = row_low + 1.0;
        let row_frac = row - row_low;

        let col_low = col.floor();
        let col_high = col_low + 1.0;
        let col_frac = col - col_low;

        let value_low_low = self.get_row_col_value(row_low, col_low)?;
        let value_low_high = self.get_row_col_value(row_low, col_high)?;
        let value_high_low = self.get_row_col_value(row_high, col_low)?;
        let value_high_high = self.get_row_col_value(row_high, col_high)?;

        // Interpolate along columns
        let value_low =
            (value_low_low as f64 * (1.0 - col_frac) + value_low_high as f64 * col_frac) as i16;
        let value_high =
            (value_high_low as f64 * (1.0 - col_frac) + value_high_high as f64 * col_frac) as i16;

        // Interpolate along rows
        let value = (value_low as f64 * (1.0 - row_frac) + value_high as f64 * row_frac) as i16;
        Ok(value)
    }

    /// Get elevation value at specific row/col position in the tile
    /// HGT format: big-endian 16-bit signed integers
    fn get_row_col_value(&self, row: f64, col: f64) -> Result<i16> {
        // HGT files are stored from north to south, so we need to flip the row
        let offset = ((self.size - row as usize - 1) * self.size + col as usize) * 2;

        if offset + 1 >= self.buffer.len() {
            bail!(
                "Offset {} exceeds buffer length {}",
                offset,
                self.buffer.len()
            );
        }

        let elevation = i16::from_be_bytes([self.buffer[offset], self.buffer[offset + 1]]);
        Ok(elevation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hgt_creation_valid_buffer() {
        let buffer = vec![0; 25934402]; // Valid buffer size for 1 arcsecond resolution
        let sw_lat_lng = (0.0, 0.0);
        let hgt = HGT::new(buffer, sw_lat_lng);
        assert!(hgt.is_ok());
        let hgt = hgt.unwrap();
        assert_eq!(hgt.size, 3601);
    }

    #[test]
    fn test_hgt_creation_3_arcsec() {
        let buffer = vec![0; 2884802]; // Valid buffer size for 3 arcsecond resolution
        let sw_lat_lng = (0.0, 0.0);
        let hgt = HGT::new(buffer, sw_lat_lng);
        assert!(hgt.is_ok());
        let hgt = hgt.unwrap();
        assert_eq!(hgt.size, 1201);
    }

    #[test]
    fn test_hgt_creation_invalid_buffer() {
        let buffer = vec![0; 100]; // Invalid buffer size
        let sw_lat_lng = (0.0, 0.0);
        let hgt = HGT::new(buffer, sw_lat_lng);
        assert!(hgt.is_err());
    }

    #[test]
    fn test_get_elevation_valid_coordinates() {
        let buffer = vec![0; 25934402]; // Valid buffer size for 1 arcsecond resolution
        let sw_lat_lng = (0.0, 0.0);
        let hgt = HGT::new(buffer, sw_lat_lng).unwrap();
        let elevation = hgt.get_elevation(0.5, 0.5);
        assert!(elevation.is_ok());
        assert_eq!(elevation.unwrap(), 0); // Default buffer values lead to elevation 0
    }

    #[test]
    fn test_get_elevation_out_of_bounds() {
        let buffer = vec![0; 25934402]; // Valid buffer size for 1 arcsecond resolution
        let sw_lat_lng = (0.0, 0.0);
        let hgt = HGT::new(buffer, sw_lat_lng).unwrap();
        let elevation = hgt.get_elevation(-1.0, -1.0);
        assert!(elevation.is_err());
    }
}
