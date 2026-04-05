use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::dsp::scanner::DabScanner;
use crate::services::app_state::{AppState, PlaybackStatus, WsMessage};
use crate::streaming::http_stream;

/// Configure REST API routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/stations", web::get().to(get_stations))
            .route("/stations/{id}", web::get().to(get_station))
            .route("/stations/{id}/metadata", web::get().to(get_station_metadata))
            .route("/status", web::get().to(get_status))
            .route("/scan", web::post().to(start_scan))
            .route("/play/{station_id}", web::post().to(play_station))
            .route("/stop", web::post().to(stop_playback))
            .route("/volume", web::post().to(set_volume))
            .route("/cast/devices", web::get().to(get_cast_devices))
            .route("/cast/discover", web::post().to(discover_cast_devices))
            .route("/cast/{device_id}", web::post().to(cast_to_device))
            .route("/cast/stop", web::post().to(stop_casting))
            .route("/logs", web::get().to(get_logs))
            .route("/stream/audio", web::get().to(audio_stream))
            .route("/system/check", web::get().to(system_check)),
    );
}

/// GET /api/stations - List all discovered stations
async fn get_stations(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let stations = state.registry.get_all_stations();
    HttpResponse::Ok().json(stations)
}

/// GET /api/stations/{id} - Get a specific station
async fn get_station(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> HttpResponse {
    let id_str = path.into_inner();
    match Uuid::parse_str(&id_str) {
        Ok(id) => match state.registry.get_station(&id) {
            Some(station) => HttpResponse::Ok().json(station),
            None => HttpResponse::NotFound().json(serde_json::json!({
                "error": "Station not found"
            })),
        },
        Err(_) => HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid station ID"
        })),
    }
}

/// GET /api/stations/{id}/metadata - Get station metadata
async fn get_station_metadata(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> HttpResponse {
    let id_str = path.into_inner();
    match Uuid::parse_str(&id_str) {
        Ok(id) => match state.registry.get_metadata(&id) {
            Some(metadata) => HttpResponse::Ok().json(metadata),
            None => HttpResponse::Ok().json(serde_json::json!({
                "station_id": id,
                "dls": null,
                "sls": null,
                "signal_quality": 0.0
            })),
        },
        Err(_) => HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid station ID"
        })),
    }
}

/// GET /api/status - Get system status
async fn get_status(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let playback = state.get_playback_status();
    let station_count = state.registry.station_count();

    HttpResponse::Ok().json(serde_json::json!({
        "version": "1.0.0",
        "station_count": station_count,
        "is_scanning": state.is_scanning(),
        "playback": playback,
        "sdr_available": true,
    }))
}

#[derive(Deserialize)]
pub struct ScanRequest {
    pub passes: Option<u32>,
    pub min_signal: Option<f32>,
}

/// POST /api/scan - Start a DAB+ scan
async fn start_scan(
    state: web::Data<Arc<AppState>>,
    body: web::Json<Option<ScanRequest>>,
) -> HttpResponse {
    if state.is_scanning() {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "Scan already in progress"
        }));
    }

    state.set_scanning(true);
    state.broadcast_log("Starting DAB+ scan...".to_string()).await;

    let state_clone = state.clone();

    // Run scan in background
    tokio::spawn(async move {
        let mut config = crate::dsp::scanner::ScanConfig::default();
        if let Some(ref req) = body.into_inner() {
            if let Some(passes) = req.passes {
                config.num_passes = passes.min(10);
            }
            if let Some(min_signal) = req.min_signal {
                config.min_signal_score = min_signal.clamp(0.01, 0.99);
            }
        }

        let scanner = DabScanner::new(config);

        let state_for_progress = state_clone.clone();
        let progress_cb = Box::new(move |progress: crate::dsp::scanner::ScanProgress| {
            let state = state_for_progress.clone();
            let progress_clone = progress.clone();
            tokio::spawn(async move {
                state.broadcast_ws(WsMessage::ScanProgress(progress_clone)).await;
                state
                    .broadcast_log(format!(
                        "Scanning {} ({:.1} MHz) - Pass {}/{} - {:.0}%",
                        progress.current_block,
                        progress.current_frequency as f64 / 1e6,
                        progress.current_pass,
                        progress.total_passes,
                        progress.percent_complete,
                    ))
                    .await;
            });
        });

        match scanner.full_scan(Some(progress_cb)).await {
            Ok(result) => {
                // Register discovered stations
                for block in &result.blocks {
                    for station in &block.stations {
                        state_clone.registry.upsert_station(station.clone());
                    }
                }

                let stations = state_clone.registry.get_all_stations();
                state_clone
                    .broadcast_ws(WsMessage::StationsUpdated(stations))
                    .await;

                state_clone
                    .broadcast_log(format!(
                        "Scan complete: {} ensembles, {} stations found",
                        result.total_ensembles, result.total_stations
                    ))
                    .await;
            }
            Err(e) => {
                state_clone
                    .broadcast_ws(WsMessage::Error(format!("Scan failed: {}", e)))
                    .await;
                state_clone
                    .broadcast_log(format!("Scan failed: {}", e))
                    .await;
            }
        }

        state_clone.set_scanning(false);
    });

    HttpResponse::Accepted().json(serde_json::json!({
        "message": "Scan started"
    }))
}

/// POST /api/play/{station_id} - Start playing a station
async fn play_station(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> HttpResponse {
    let id_str = path.into_inner();
    match Uuid::parse_str(&id_str) {
        Ok(id) => match state.registry.get_station(&id) {
            Some(station) => {
                info!("Playing station: {}", station.name);

                state.set_current_station(Some(id));
                let status = PlaybackStatus {
                    is_playing: true,
                    station_id: Some(id),
                    station_name: Some(station.name.clone()),
                    volume: state.get_volume(),
                    elapsed_seconds: 0,
                };
                state.update_playback(status.clone());
                state.broadcast_ws(WsMessage::PlaybackStatus(status)).await;
                state
                    .broadcast_log(format!("Now playing: {}", station.name))
                    .await;

                // Set initial DLS
                let dls = crate::services::metadata_parser::get_simulated_dls(station.service_id);
                state.registry.update_dls(id, dls);

                HttpResponse::Ok().json(serde_json::json!({
                    "message": format!("Playing {}", station.name),
                    "station": station,
                }))
            }
            None => HttpResponse::NotFound().json(serde_json::json!({
                "error": "Station not found"
            })),
        },
        Err(_) => HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid station ID"
        })),
    }
}

/// POST /api/stop - Stop playback
async fn stop_playback(state: web::Data<Arc<AppState>>) -> HttpResponse {
    state.set_current_station(None);
    let status = PlaybackStatus {
        is_playing: false,
        station_id: None,
        station_name: None,
        volume: state.get_volume(),
        elapsed_seconds: 0,
    };
    state.update_playback(status.clone());
    state.broadcast_ws(WsMessage::PlaybackStatus(status)).await;
    state.broadcast_log("Playback stopped".to_string()).await;

    HttpResponse::Ok().json(serde_json::json!({
        "message": "Playback stopped"
    }))
}

#[derive(Deserialize)]
pub struct VolumeRequest {
    pub volume: f32,
}

/// POST /api/volume - Set volume
async fn set_volume(
    state: web::Data<Arc<AppState>>,
    body: web::Json<VolumeRequest>,
) -> HttpResponse {
    let volume = body.volume.clamp(0.0, 1.0);
    state.set_volume(volume);

    HttpResponse::Ok().json(serde_json::json!({
        "volume": volume
    }))
}

/// GET /api/cast/devices - List cast devices
async fn get_cast_devices(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let devices = state.get_cast_devices();
    HttpResponse::Ok().json(devices)
}

/// POST /api/cast/discover - Discover cast devices
async fn discover_cast_devices(state: web::Data<Arc<AppState>>) -> HttpResponse {
    state
        .broadcast_log("Discovering cast devices...".to_string())
        .await;

    let chromecast_mgr = crate::casting::chromecast::ChromecastManager::new();
    let airplay_mgr = crate::casting::airplay::AirPlayManager::new();

    let (cc_result, ap_result) = tokio::join!(
        chromecast_mgr.discover_devices(),
        airplay_mgr.discover_devices(),
    );

    let mut all_devices = Vec::new();
    if let Ok(devices) = cc_result {
        all_devices.extend(devices);
    }
    if let Ok(devices) = ap_result {
        all_devices.extend(devices);
    }

    state.update_cast_devices(all_devices.clone());
    state
        .broadcast_ws(WsMessage::CastDevices(all_devices.clone()))
        .await;
    state
        .broadcast_log(format!("{} cast devices found", all_devices.len()))
        .await;

    HttpResponse::Ok().json(all_devices)
}

/// POST /api/cast/{device_id} - Cast to a device
async fn cast_to_device(
    state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> HttpResponse {
    let device_id = path.into_inner();

    state
        .broadcast_log(format!("Casting to device: {}", device_id))
        .await;

    HttpResponse::Ok().json(serde_json::json!({
        "message": format!("Casting to {}", device_id)
    }))
}

/// POST /api/cast/stop - Stop casting
async fn stop_casting(state: web::Data<Arc<AppState>>) -> HttpResponse {
    state.broadcast_log("Casting stopped".to_string()).await;

    HttpResponse::Ok().json(serde_json::json!({
        "message": "Casting stopped"
    }))
}

/// GET /api/logs - Get log entries
async fn get_logs(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let logs = state.get_logs();
    HttpResponse::Ok().json(logs)
}

/// GET /api/stream/audio - Audio stream endpoint
async fn audio_stream(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    http_stream::stream_audio(req, state).await
}

/// GET /api/system/check - System dependency check
async fn system_check() -> HttpResponse {
    let checks = crate::system::installer_checks::run_all_checks();
    HttpResponse::Ok().json(checks)
}
