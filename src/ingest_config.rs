use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;
use uuid::Uuid;

use crate::SocketTarget;

/// Stream data format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "lowercase")]
pub enum StreamFormat {
    Aprs,
    Adsb,
    Sbs,
}

impl std::fmt::Display for StreamFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamFormat::Aprs => write!(f, "aprs"),
            StreamFormat::Adsb => write!(f, "adsb"),
            StreamFormat::Sbs => write!(f, "sbs"),
        }
    }
}

/// Data stream configuration — API/TypeScript layer (camelCase)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DataStream {
    pub id: Uuid,
    pub name: String,
    pub format: StreamFormat,
    pub host: String,
    pub port: u16,
    pub enabled: bool,
    pub callsign: Option<String>,
    pub filter: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data stream configuration — TOML file layer (snake_case)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlDataStream {
    pub id: Uuid,
    pub name: String,
    pub format: StreamFormat,
    pub host: String,
    pub port: u16,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callsign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TomlDataStream> for DataStream {
    fn from(t: TomlDataStream) -> Self {
        Self {
            id: t.id,
            name: t.name,
            format: t.format,
            host: t.host,
            port: t.port,
            enabled: t.enabled,
            callsign: t.callsign,
            filter: t.filter,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

impl From<DataStream> for TomlDataStream {
    fn from(d: DataStream) -> Self {
        Self {
            id: d.id,
            name: d.name,
            format: d.format,
            host: d.host,
            port: d.port,
            enabled: d.enabled,
            callsign: d.callsign,
            filter: d.filter,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

/// Top-level ingest configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfigFile {
    /// Optional instance identifier for running multiple ingest instances on the same host.
    /// Used to differentiate lock names and queue filenames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,
    /// Socket address for connecting to soar-run.
    ///
    /// Formats:
    /// - `None` (default): uses the default Unix socket path from `soar::socket_path()`
    /// - `"unix:///path/to/socket"`: explicit Unix socket path
    /// - `"tcp://host:port"`: TCP connection (for remote ingest)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_address: Option<String>,
    #[serde(default)]
    pub streams: Vec<TomlDataStream>,
}

fn default_retry_delay() -> u64 {
    5
}

impl IngestConfigFile {
    /// Load config from a TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let contents =
            std::fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))?;
        let config: IngestConfigFile =
            toml::from_str(&contents).with_context(|| format!("Failed to parse {:?}", path))?;
        Ok(config)
    }

    /// Save config to a TOML file (atomic: write to .tmp then rename)
    pub fn save(&self, path: &Path) -> Result<()> {
        let contents =
            toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;
        let tmp_path = path.with_extension("toml.tmp");
        std::fs::write(&tmp_path, &contents)
            .with_context(|| format!("Failed to write {:?}", tmp_path))?;
        std::fs::rename(&tmp_path, path)
            .with_context(|| format!("Failed to rename {:?} to {:?}", tmp_path, path))?;
        Ok(())
    }

    /// Get streams as API-layer DataStream values
    pub fn data_streams(&self) -> Vec<DataStream> {
        self.streams.iter().cloned().map(DataStream::from).collect()
    }

    /// Parse `socket_address` into a `SocketTarget`.
    ///
    /// - `None` → default Unix socket from `soar::socket_path()`
    /// - `"unix:///path/to/sock"` → `SocketTarget::Unix(path)`
    /// - `"tcp://host:port"` → `SocketTarget::Tcp(addr)`
    pub fn socket_target(&self) -> Result<SocketTarget> {
        match &self.socket_address {
            None => Ok(SocketTarget::Unix(crate::socket_path())),
            Some(addr) => parse_socket_address(addr),
        }
    }
}

/// Parse a socket address string into a `SocketTarget`.
fn parse_socket_address(addr: &str) -> Result<SocketTarget> {
    if let Some(path) = addr.strip_prefix("unix://") {
        if path.is_empty() {
            anyhow::bail!(
                "Invalid Unix socket address: '{}'. Path must not be empty",
                addr
            );
        }
        Ok(SocketTarget::Unix(PathBuf::from(path)))
    } else if let Some(host_port) = addr.strip_prefix("tcp://") {
        let (host, port_str) = host_port.split_once(':').ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid TCP socket address: '{}'. Expected 'tcp://host:port'",
                addr
            )
        })?;

        if host.trim().is_empty() {
            anyhow::bail!(
                "Invalid TCP socket address: '{}'. Host must not be empty",
                addr
            );
        }

        let port_str = port_str.trim();
        if port_str.is_empty() {
            anyhow::bail!(
                "Invalid TCP socket address: '{}'. Port must not be empty",
                addr
            );
        }

        // Validate port is a valid u16
        let _port: u16 = port_str.parse().with_context(|| {
            format!(
                "Invalid TCP socket address: '{}'. Port '{}' must be a number 0-65535",
                addr, port_str
            )
        })?;

        Ok(SocketTarget::Tcp(host_port.to_string()))
    } else {
        anyhow::bail!(
            "Invalid socket_address format: '{}'. Expected 'unix:///path' or 'tcp://host:port'",
            addr
        );
    }
}

/// Resolve the ingest config file path.
///
/// Priority:
/// 1. `SOAR_INGEST_CONFIG` env var
/// 2. `/etc/soar/ingest.toml` (production/staging)
/// 3. `./ingest.toml` (development)
pub fn ingest_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("SOAR_INGEST_CONFIG") {
        return PathBuf::from(path);
    }

    match std::env::var("SOAR_ENV").as_deref() {
        Ok("production") | Ok("staging") => PathBuf::from("/etc/soar/ingest.toml"),
        _ => PathBuf::from("./ingest.toml"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_roundtrip() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 10,
            socket_address: None,
            streams: vec![TomlDataStream {
                id: Uuid::new_v4(),
                name: "Test OGN".to_string(),
                format: StreamFormat::Aprs,
                host: "aprs.glidernet.org".to_string(),
                port: 10152,
                enabled: true,
                callsign: Some("N0CALL".to_string()),
                filter: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }],
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: IngestConfigFile = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.retry_delay, 10);
        assert_eq!(parsed.streams.len(), 1);
        assert_eq!(parsed.streams[0].name, "Test OGN");
        assert_eq!(parsed.streams[0].format, StreamFormat::Aprs);
    }

    #[test]
    fn test_config_load_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-ingest.toml");

        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: None,
            streams: vec![
                TomlDataStream {
                    id: Uuid::new_v4(),
                    name: "OGN Full Feed".to_string(),
                    format: StreamFormat::Aprs,
                    host: "aprs.glidernet.org".to_string(),
                    port: 10152,
                    enabled: true,
                    callsign: Some("N0CALL".to_string()),
                    filter: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
                TomlDataStream {
                    id: Uuid::new_v4(),
                    name: "Local Beast".to_string(),
                    format: StreamFormat::Adsb,
                    host: "192.168.1.100".to_string(),
                    port: 30005,
                    enabled: false,
                    callsign: None,
                    filter: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            ],
        };

        config.save(&path).unwrap();
        let loaded = IngestConfigFile::load(&path).unwrap();

        assert_eq!(loaded.retry_delay, 5);
        assert_eq!(loaded.streams.len(), 2);
        assert_eq!(loaded.streams[0].name, "OGN Full Feed");
        assert_eq!(loaded.streams[1].name, "Local Beast");
        assert!(!loaded.streams[1].enabled);
    }

    #[test]
    fn test_socket_target_default() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: None,
            streams: vec![],
        };
        let target = config.socket_target().unwrap();
        assert!(matches!(target, SocketTarget::Unix(_)));
    }

    #[test]
    fn test_socket_target_unix() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: Some("unix:///var/run/soar/run.sock".to_string()),
            streams: vec![],
        };
        let target = config.socket_target().unwrap();
        match target {
            SocketTarget::Unix(path) => {
                assert_eq!(path, std::path::PathBuf::from("/var/run/soar/run.sock"));
            }
            _ => panic!("Expected Unix target"),
        }
    }

    #[test]
    fn test_socket_target_tcp_ip() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: Some("tcp://10.0.0.1:9384".to_string()),
            streams: vec![],
        };
        let target = config.socket_target().unwrap();
        match target {
            SocketTarget::Tcp(addr) => {
                assert_eq!(addr, "10.0.0.1:9384");
            }
            _ => panic!("Expected TCP target"),
        }
    }

    #[test]
    fn test_socket_target_tcp_hostname() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: Some("tcp://supervillain:9384".to_string()),
            streams: vec![],
        };
        let target = config.socket_target().unwrap();
        match target {
            SocketTarget::Tcp(addr) => {
                assert_eq!(addr, "supervillain:9384");
            }
            _ => panic!("Expected TCP target"),
        }
    }

    #[test]
    fn test_socket_target_invalid() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: Some("http://example.com".to_string()),
            streams: vec![],
        };
        assert!(config.socket_target().is_err());
    }

    #[test]
    fn test_socket_target_tcp_missing_port() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: Some("tcp://supervillain".to_string()),
            streams: vec![],
        };
        assert!(config.socket_target().is_err());
    }

    #[test]
    fn test_socket_target_tcp_empty_host() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: Some("tcp://:9384".to_string()),
            streams: vec![],
        };
        assert!(config.socket_target().is_err());
    }

    #[test]
    fn test_socket_target_tcp_invalid_port() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: Some("tcp://host:notaport".to_string()),
            streams: vec![],
        };
        assert!(config.socket_target().is_err());
    }

    #[test]
    fn test_socket_target_unix_empty_path() {
        let config = IngestConfigFile {
            instance_id: None,
            retry_delay: 5,
            socket_address: Some("unix://".to_string()),
            streams: vec![],
        };
        assert!(config.socket_target().is_err());
    }

    #[test]
    fn test_data_stream_conversion() {
        let toml_stream = TomlDataStream {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            format: StreamFormat::Sbs,
            host: "localhost".to_string(),
            port: 30003,
            enabled: true,
            callsign: None,
            filter: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let data_stream: DataStream = toml_stream.clone().into();
        assert_eq!(data_stream.name, toml_stream.name);
        assert_eq!(data_stream.format, StreamFormat::Sbs);

        let back: TomlDataStream = data_stream.into();
        assert_eq!(back.name, toml_stream.name);
    }
}
