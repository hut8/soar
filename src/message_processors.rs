use chrono::Utc;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{error, info, warn};
use zstd::stream::copy_encode;

use crate::MessageProcessor;

/// Archive message processor for file streaming
pub struct ArchiveMessageProcessor {
    archive: Option<Arc<MessageArchive>>,
}

impl ArchiveMessageProcessor {
    pub fn new(archive_dir: Option<String>) -> Self {
        let archive = archive_dir.map(|dir| Arc::new(MessageArchive::new(dir)));
        Self { archive }
    }
}

impl MessageProcessor for ArchiveMessageProcessor {
    fn process_message(&self, message: ogn_parser::AprsPacket) {
        tracing::trace!("Parsed APRS packet: {:?}", message);
        // Archive processor doesn't need to do anything with parsed messages
    }

    fn process_raw_message(&self, raw_message: &str) {
        // Log to file archive if configured
        if let Some(ref archive) = self.archive {
            archive.log_message(raw_message);
        }
    }
}

pub struct NoOpMessageProcessor;

impl MessageProcessor for NoOpMessageProcessor {
    fn process_message(&self, _message: ogn_parser::AprsPacket) {
        // No-op implementation
    }

    fn process_raw_message(&self, _raw_message: &str) {
        // No-op implementation
    }
}

/// Message archive for logging APRS messages to daily files
pub struct MessageArchive {
    base_dir: String,
    current_file: Mutex<Option<std::fs::File>>,
    current_date: Mutex<String>,
}

impl MessageArchive {
    pub fn new(base_dir: String) -> Self {
        Self {
            base_dir,
            current_file: Mutex::new(None),
            current_date: Mutex::new(String::new()),
        }
    }

    pub fn log_message(&self, message: &str) {
        let now = Utc::now();
        let date_str = now.format("%Y-%m-%d").to_string();

        let mut current_date = self.current_date.lock().unwrap();
        let mut current_file = self.current_file.lock().unwrap();

        // Check if we need to create a new file (new day or first time)
        if *current_date != date_str {
            // Close the current file if it exists and compress it
            if current_file.take().is_some() {
                self.compress_log_file(&current_date);
            }

            // Create the archive directory if it doesn't exist
            let archive_path = PathBuf::from(&self.base_dir);
            if let Err(e) = create_dir_all(&archive_path) {
                error!(
                    "Failed to create archive directory {}: {}",
                    archive_path.display(),
                    e
                );
                return;
            }

            // Create the new log file
            let log_file_path = archive_path.join(format!("{date_str}.log"));
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_path)
            {
                Ok(file) => {
                    info!("Opened new archive log file: {}", log_file_path.display());
                    *current_file = Some(file);
                    *current_date = date_str;
                }
                Err(e) => {
                    error!(
                        "Failed to open archive log file {}: {}",
                        log_file_path.display(),
                        e
                    );
                    return;
                }
            }
        }

        // Write the message to the current file
        if let Some(ref mut file) = *current_file {
            let timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
            if let Err(e) = writeln!(file, "[{timestamp}] {message}") {
                error!("Failed to write to archive log file: {}", e);
            } else if let Err(e) = file.flush() {
                error!("Failed to flush archive log file: {}", e);
            }
        }
    }

    /// Compress the log file for the given date using zstd
    fn compress_log_file(&self, date: &str) {
        let archive_path = PathBuf::from(&self.base_dir);
        let log_file_path = archive_path.join(format!("{date}.log"));
        let compressed_file_path = archive_path.join(format!("{date}.log.zst"));

        if !log_file_path.exists() {
            return;
        }

        match (
            File::open(&log_file_path),
            File::create(&compressed_file_path),
        ) {
            (Ok(input_file), Ok(output_file)) => {
                let mut input_reader = BufReader::new(input_file);
                let mut output_writer = BufWriter::new(output_file);

                // Compress with zstd level 3 (good compression/speed tradeoff)
                match copy_encode(&mut input_reader, &mut output_writer, 3) {
                    Ok(_) => {
                        info!(
                            "Compressed log file {} to {}",
                            log_file_path.display(),
                            compressed_file_path.display()
                        );

                        // Remove the original uncompressed file
                        if let Err(e) = std::fs::remove_file(&log_file_path) {
                            warn!(
                                "Failed to remove original log file {}: {}",
                                log_file_path.display(),
                                e
                            );
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to compress log file {}: {}",
                            log_file_path.display(),
                            e
                        );
                        // Remove the incomplete compressed file
                        let _ = std::fs::remove_file(&compressed_file_path);
                    }
                }
            }
            (Err(e), _) => {
                error!(
                    "Failed to open log file {} for compression: {}",
                    log_file_path.display(),
                    e
                );
            }
            (_, Err(e)) => {
                error!(
                    "Failed to create compressed file {}: {}",
                    compressed_file_path.display(),
                    e
                );
            }
        }
    }
}

#[test]
fn test_message_archive() {
    use std::fs;
    use std::path::Path;

    let temp_dir = "/tmp/test_aprs_archive";
    let archive = MessageArchive::new(temp_dir.to_string());

    // Log a test message
    archive.log_message("TEST>APRS:>Test archive message");

    // Check that the directory was created
    assert!(Path::new(temp_dir).exists());

    // Check that a log file was created with today's date
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let log_file_path = Path::new(temp_dir).join(format!("{today}.log"));
    assert!(log_file_path.exists());

    // Read the file content and verify the message was logged
    let content = fs::read_to_string(&log_file_path).expect("Failed to read log file");
    assert!(content.contains("TEST>APRS:>Test archive message"));

    // Clean up
    let _ = fs::remove_dir_all(temp_dir);
}
