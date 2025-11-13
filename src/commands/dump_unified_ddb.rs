use anyhow::Result;
use std::fs::File;
use std::io::Write;
use tracing::info;

use soar::devices::read_flarmnet_file;

const DDB_URL_UNIFIED_FLARMNET: &str = "https://turbo87.github.io/united-flarmnet/united.fln";

/// Download unified FlarmNet database and dump to JSONL file
pub async fn handle_dump_unified_ddb(
    output_path: String,
    source_path: Option<String>,
) -> Result<()> {
    // Determine the source file path and whether we need to clean up
    let (temp_path, cleanup_temp) = match source_path {
        Some(local_path) => {
            info!("Using local unified FlarmNet database from: {}", local_path);
            // Verify the file exists
            if !std::path::Path::new(&local_path).exists() {
                return Err(anyhow::anyhow!(
                    "Local source file does not exist: {}",
                    local_path
                ));
            }
            (local_path, false)
        }
        None => {
            info!(
                "Downloading unified FlarmNet database from {}",
                DDB_URL_UNIFIED_FLARMNET
            );

            // Download the unified FlarmNet file
            let response = reqwest::get(DDB_URL_UNIFIED_FLARMNET).await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Failed to download unified FlarmNet database: HTTP {}",
                    response.status()
                ));
            }

            let content = response.text().await?;
            info!(
                "Downloaded unified FlarmNet database ({} bytes)",
                content.len()
            );

            // Save to temporary file for parsing (use .fln extension since it's FlarmNet format)
            let temp_path = format!("{}.tmp.fln", output_path);
            std::fs::write(&temp_path, &content)?;
            info!("Saved to temporary file: {}", temp_path);

            (temp_path, true)
        }
    };

    // Parse the FlarmNet file
    info!("Parsing unified FlarmNet database...");
    let devices = read_flarmnet_file(&temp_path)?;
    info!("Successfully parsed {} devices", devices.len());

    // Clean up temp file only if we downloaded it
    if cleanup_temp {
        std::fs::remove_file(&temp_path)?;
    }

    // Write devices to JSONL file (one JSON object per line)
    info!(
        "Writing {} devices to JSONL file: {}",
        devices.len(),
        output_path
    );
    let mut output_file = File::create(&output_path)?;

    for (idx, device) in devices.iter().enumerate() {
        let json_line = serde_json::to_string(device)?;
        writeln!(output_file, "{}", json_line)?;

        // Progress indicator every 1000 devices
        if (idx + 1) % 1000 == 0 {
            info!("Written {} / {} devices", idx + 1, devices.len());
        }
    }

    info!(
        "Successfully wrote {} devices to {}",
        devices.len(),
        output_path
    );
    Ok(())
}
