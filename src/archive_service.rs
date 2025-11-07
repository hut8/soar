use crate::queue_config::ARCHIVE_QUEUE_SIZE;
use anyhow::Result;
use tracing::{error, info, warn};

/// Archive service for managing daily log files and compression
#[derive(Clone)]
pub struct ArchiveService {
    sender: flume::Sender<String>,
}

impl ArchiveService {
    /// Create a new archive service and start the background archival task
    pub async fn new(base_dir: String) -> Result<Self> {
        use chrono::Local;
        use std::fs;
        use std::io::Write;
        use std::path::PathBuf;

        // Create base directory if it doesn't exist
        fs::create_dir_all(&base_dir)?;

        // Check for yesterday's uncompressed file and compress it
        let yesterday = Local::now().date_naive() - chrono::Duration::days(1);
        let yesterday_file = PathBuf::from(&base_dir).join(format!("{}.log", yesterday));
        if yesterday_file.exists() {
            info!(
                "Found uncompressed file from yesterday: {:?}, compressing...",
                yesterday_file
            );
            if let Err(e) = Self::compress_file(&yesterday_file).await {
                warn!("Failed to compress yesterday's file: {}", e);
            }
        }

        // Use bounded channel to prevent unbounded memory growth
        // Should handle bursts while limiting memory to ~1.5MB (assuming ~150 bytes per APRS message)
        let (sender, receiver) = flume::bounded::<String>(ARCHIVE_QUEUE_SIZE);

        info!(
            "Archive service initialized with bounded channel (capacity: {} messages, ~1.5MB buffer)",
            ARCHIVE_QUEUE_SIZE
        );

        // Spawn background task for file writing and management
        tokio::spawn(async move {
            let mut current_file: Option<std::io::BufWriter<std::fs::File>> = None;
            let mut current_date: Option<chrono::NaiveDate> = None;
            let mut messages_written = 0u64;
            let mut messages_since_flush = 0u64;
            let mut last_stats_log = std::time::Instant::now();
            let mut last_flush = std::time::Instant::now();

            while let Ok(message) = receiver.recv_async().await {
                let now = Local::now();
                let today = now.date_naive();

                // Check if we need to rotate to a new file
                if current_date != Some(today) {
                    // Close current file if exists
                    if let Some(mut file) = current_file.take() {
                        if let Err(e) = file.flush() {
                            warn!("Failed to flush archive file: {}", e);
                        }
                        drop(file);

                        // Compress the previous day's file
                        if let Some(prev_date) = current_date {
                            let prev_file =
                                PathBuf::from(&base_dir).join(format!("{}.log", prev_date));
                            tokio::spawn(async move {
                                if let Err(e) = Self::compress_file(&prev_file).await {
                                    warn!("Failed to compress archive file: {}", e);
                                }
                            });
                        }
                    }

                    // Open new file for today
                    let log_path = PathBuf::from(&base_dir).join(format!("{}.log", today));
                    match std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_path)
                    {
                        Ok(file) => {
                            info!("Opened new archive file: {:?}", log_path);
                            // Wrap file in BufWriter with 1MB buffer for efficient writes
                            current_file =
                                Some(std::io::BufWriter::with_capacity(1024 * 1024, file));
                            current_date = Some(today);
                            messages_since_flush = 0;
                            last_flush = std::time::Instant::now();
                        }
                        Err(e) => {
                            error!("Failed to open archive file {:?}: {}", log_path, e);
                            continue;
                        }
                    }
                }

                // Write message to current file
                if let Some(file) = &mut current_file {
                    if let Err(e) = writeln!(file, "{}", message) {
                        error!(
                            "Failed to write to archive file: {} - this may cause message backlog",
                            e
                        );
                    } else {
                        messages_written += 1;
                        messages_since_flush += 1;
                    }
                }

                // Flush buffer periodically to ensure data durability
                // Flush every 1000 messages OR every 30 seconds, whichever comes first
                let should_flush =
                    messages_since_flush >= 1000 || last_flush.elapsed().as_secs() >= 30;

                if should_flush && let Some(file) = &mut current_file {
                    let flush_start = std::time::Instant::now();
                    if let Err(e) = file.flush() {
                        error!("Failed to flush archive buffer: {}", e);
                    } else {
                        let flush_duration = flush_start.elapsed();

                        // Warn if flush takes more than 100ms (indicates I/O issues)
                        if flush_duration.as_millis() > 100 {
                            warn!(
                                "Slow archive flush detected: {}ms for {} messages - disk I/O may be bottleneck",
                                flush_duration.as_millis(),
                                messages_since_flush
                            );
                        }

                        messages_since_flush = 0;
                        last_flush = std::time::Instant::now();
                    }
                }

                // Log statistics every 5 minutes
                if last_stats_log.elapsed().as_secs() >= 300 {
                    let queue_len = receiver.len();
                    info!(
                        "Archive stats: {} messages written in last 5min, {} messages queued",
                        messages_written, queue_len
                    );
                    if queue_len > 5000 {
                        warn!(
                            "Archive queue is building up ({} messages) - disk writes may be too slow",
                            queue_len
                        );
                    }
                    messages_written = 0;
                    last_stats_log = std::time::Instant::now();
                }
            }

            // Flush final file on shutdown
            if let Some(mut file) = current_file {
                let _ = file.flush();
            }
        });

        Ok(Self { sender })
    }

    /// Compress a log file using zstd
    async fn compress_file(file_path: &std::path::PathBuf) -> Result<()> {
        use std::fs::File;
        use std::io::{BufReader, BufWriter};

        let compressed_path = file_path.with_extension("log.zst");

        // Read the original file
        let input_file = File::open(file_path)?;
        let reader = BufReader::new(input_file);

        // Create the compressed file
        let output_file = File::create(&compressed_path)?;
        let writer = BufWriter::new(output_file);

        // Compress with zstd (compression level 3 is a good balance)
        let mut encoder = zstd::Encoder::new(writer, 3)?;
        std::io::copy(&mut BufReader::new(reader), &mut encoder)?;
        encoder.finish()?;

        // Delete the original file after successful compression
        std::fs::remove_file(file_path)?;

        info!(
            "Compressed {:?} to {:?} and deleted original",
            file_path, compressed_path
        );

        Ok(())
    }

    /// Archive a message (blocking)
    /// This will block if the archive queue is full, applying backpressure to the caller
    pub async fn archive(&self, message: &str) {
        // Use send_async to block until space is available - never drop messages
        match self.sender.send_async(message.to_string()).await {
            Ok(_) => {
                // Message successfully queued
            }
            Err(flume::SendError(_)) => {
                // Archive service has shut down
                error!("Archive service channel is closed - cannot archive message");
            }
        }
    }
}
