use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::services::station_registry::StationRegistry;

/// WebSocket message types
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    StationsUpdated(Vec<crate::services::station_registry::Station>),
    MetadataUpdated(crate::services::station_registry::StationMetadata),
    ScanProgress(crate::dsp::scanner::ScanProgress),
    PlaybackStatus(PlaybackStatus),
    CastDevices(Vec<CastDeviceInfo>),
    Log(String),
    Error(String),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlaybackStatus {
    pub is_playing: bool,
    pub station_id: Option<Uuid>,
    pub station_name: Option<String>,
    pub volume: f32,
    pub elapsed_seconds: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CastDeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: String, // "chromecast" or "airplay"
    pub is_connected: bool,
}

/// Global application state shared across all handlers
pub struct AppState {
    pub registry: StationRegistry,
    current_station: RwLock<Option<Uuid>>,
    playback_status: RwLock<PlaybackStatus>,
    scan_in_progress: RwLock<bool>,
    ws_sender: broadcast::Sender<String>,
    logs: RwLock<Vec<String>>,
    cast_devices: RwLock<Vec<CastDeviceInfo>>,
}

impl AppState {
    pub fn new() -> Self {
        let (ws_sender, _) = broadcast::channel(256);

        Self {
            registry: StationRegistry::new(),
            current_station: RwLock::new(None),
            playback_status: RwLock::new(PlaybackStatus {
                is_playing: false,
                station_id: None,
                station_name: None,
                volume: 0.75,
                elapsed_seconds: 0,
            }),
            scan_in_progress: RwLock::new(false),
            ws_sender,
            logs: RwLock::new(Vec::new()),
            cast_devices: RwLock::new(Vec::new()),
        }
    }

    pub fn get_current_station_id(&self) -> Option<Uuid> {
        *self.current_station.read()
    }

    pub fn set_current_station(&self, station_id: Option<Uuid>) {
        *self.current_station.write() = station_id;
    }

    pub fn get_playback_status(&self) -> PlaybackStatus {
        self.playback_status.read().clone()
    }

    pub fn update_playback(&self, status: PlaybackStatus) {
        *self.playback_status.write() = status;
    }

    pub fn is_scanning(&self) -> bool {
        *self.scan_in_progress.read()
    }

    pub fn set_scanning(&self, scanning: bool) {
        *self.scan_in_progress.write() = scanning;
    }

    pub fn subscribe_ws(&self) -> broadcast::Receiver<String> {
        self.ws_sender.subscribe()
    }

    pub async fn broadcast_ws(&self, msg: WsMessage) {
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = self.ws_sender.send(json);
        }
    }

    pub async fn broadcast_log(&self, msg: String) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S").to_string();
        let log_entry = format!("[{}] {}", timestamp, msg);

        {
            let mut logs = self.logs.write();
            logs.push(log_entry.clone());

            // Keep last 1000 log entries
            if logs.len() > 1000 {
                let drain_count = logs.len() - 1000;
                logs.drain(..drain_count);
            }
        }

        self.broadcast_ws(WsMessage::Log(log_entry)).await;
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.read().clone()
    }

    pub fn get_cast_devices(&self) -> Vec<CastDeviceInfo> {
        self.cast_devices.read().clone()
    }

    pub fn update_cast_devices(&self, devices: Vec<CastDeviceInfo>) {
        *self.cast_devices.write() = devices;
    }

    pub fn set_volume(&self, volume: f32) {
        self.playback_status.write().volume = volume.clamp(0.0, 1.0);
    }

    pub fn get_volume(&self) -> f32 {
        self.playback_status.read().volume
    }
}
