use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::ingest_config::{
    DataStream, IngestConfigFile, StreamFormat, TomlDataStream, ingest_config_path,
};

use super::{DataListResponse, DataResponse, json_error};

/// Request body for creating a new data stream
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDataStreamRequest {
    pub name: String,
    pub format: StreamFormat,
    pub host: String,
    pub port: u16,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub callsign: Option<String>,
    pub filter: Option<String>,
}

fn default_enabled() -> bool {
    true
}

/// Request body for updating a data stream
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataStreamRequest {
    pub name: Option<String>,
    pub format: Option<StreamFormat>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub enabled: Option<bool>,
    pub callsign: Option<Option<String>>,
    pub filter: Option<Option<String>>,
}

/// GET /data-streams
/// List all configured data streams
pub async fn list_data_streams(_admin: AdminUser) -> impl IntoResponse {
    let config_path = ingest_config_path();

    match IngestConfigFile::load(&config_path) {
        Ok(config) => {
            let streams: Vec<DataStream> = config.data_streams();
            Json(DataListResponse { data: streams }).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to load ingest config");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to load config: {}", e),
            )
            .into_response()
        }
    }
}

/// POST /data-streams
/// Create a new data stream
pub async fn create_data_stream(
    _admin: AdminUser,
    Json(req): Json<CreateDataStreamRequest>,
) -> impl IntoResponse {
    let config_path = ingest_config_path();

    // Validate input
    let name = req.name.trim().to_string();
    let host = req.host.trim().to_string();
    if name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Name is required").into_response();
    }
    if host.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Host is required").into_response();
    }
    if req.port == 0 {
        return json_error(StatusCode::BAD_REQUEST, "Port must be between 1 and 65535")
            .into_response();
    }

    let mut config = match IngestConfigFile::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to load ingest config");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to load config: {}", e),
            )
            .into_response();
        }
    };

    let now = Utc::now();
    let stream = TomlDataStream {
        id: Uuid::new_v4(),
        name,
        format: req.format,
        host,
        port: req.port,
        enabled: req.enabled,
        callsign: req.callsign,
        filter: req.filter,
        created_at: now,
        updated_at: now,
    };

    config.streams.push(stream.clone());

    if let Err(e) = config.save(&config_path) {
        error!(error = %e, "Failed to save ingest config");
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to save config: {}", e),
        )
        .into_response();
    }

    let data_stream: DataStream = stream.into();
    (
        StatusCode::CREATED,
        Json(DataResponse { data: data_stream }),
    )
        .into_response()
}

/// PUT /data-streams/{id}
/// Update an existing data stream
pub async fn update_data_stream(
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDataStreamRequest>,
) -> impl IntoResponse {
    let config_path = ingest_config_path();

    let mut config = match IngestConfigFile::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to load ingest config");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to load config: {}", e),
            )
            .into_response();
        }
    };

    let stream = match config.streams.iter_mut().find(|s| s.id == id) {
        Some(s) => s,
        None => {
            return json_error(StatusCode::NOT_FOUND, "Stream not found").into_response();
        }
    };

    if let Some(name) = req.name {
        stream.name = name;
    }
    if let Some(format) = req.format {
        stream.format = format;
    }
    if let Some(host) = req.host {
        stream.host = host;
    }
    if let Some(port) = req.port {
        stream.port = port;
    }
    if let Some(enabled) = req.enabled {
        stream.enabled = enabled;
    }
    if let Some(callsign) = req.callsign {
        stream.callsign = callsign;
    }
    if let Some(filter) = req.filter {
        stream.filter = filter;
    }

    stream.updated_at = Utc::now();

    let updated: DataStream = stream.clone().into();

    if let Err(e) = config.save(&config_path) {
        error!(error = %e, "Failed to save ingest config");
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to save config: {}", e),
        )
        .into_response();
    }

    Json(DataResponse { data: updated }).into_response()
}

/// DELETE /data-streams/{id}
/// Delete a data stream
pub async fn delete_data_stream(_admin: AdminUser, Path(id): Path<Uuid>) -> impl IntoResponse {
    let config_path = ingest_config_path();

    let mut config = match IngestConfigFile::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to load ingest config");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to load config: {}", e),
            )
            .into_response();
        }
    };

    let original_len = config.streams.len();
    config.streams.retain(|s| s.id != id);

    if config.streams.len() == original_len {
        return json_error(StatusCode::NOT_FOUND, "Stream not found").into_response();
    }

    if let Err(e) = config.save(&config_path) {
        error!(error = %e, "Failed to save ingest config");
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to save config: {}", e),
        )
        .into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}
