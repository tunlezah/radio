use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures::StreamExt as _;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::services::app_state::AppState;

/// Configure WebSocket routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/ws", web::get().to(ws_handler));
}

/// WebSocket handler - provides real-time updates to the frontend
async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    info!("WebSocket client connected from {:?}", req.peer_addr());

    // Subscribe to broadcast messages
    let mut ws_rx = state.subscribe_ws();

    // Send initial state
    let stations = state.registry.get_all_stations();
    let initial_msg = serde_json::json!({
        "type": "initial_state",
        "data": {
            "stations": stations,
            "playback": state.get_playback_status(),
            "is_scanning": state.is_scanning(),
            "cast_devices": state.get_cast_devices(),
        }
    });

    if let Ok(json) = serde_json::to_string(&initial_msg) {
        let _ = session.text(json).await;
    }

    // Spawn task to handle bidirectional communication
    let state_clone = state.clone();
    actix_web::rt::spawn(async move {
        let mut heartbeat = tokio::time::interval(Duration::from_secs(15));

        loop {
            tokio::select! {
                // Handle broadcast messages from the server
                Ok(msg) = ws_rx.recv() => {
                    if session.text(msg).await.is_err() {
                        break;
                    }
                }

                // Handle messages from the client
                Some(Ok(msg)) = msg_stream.next() => {
                    match msg {
                        Message::Text(text) => {
                            debug!("WS received: {}", text);
                            handle_ws_message(&text, &state_clone, &mut session).await;
                        }
                        Message::Ping(data) => {
                            let _ = session.pong(&data).await;
                        }
                        Message::Close(reason) => {
                            info!("WebSocket client disconnecting: {:?}", reason);
                            let _ = session.close(reason).await;
                            break;
                        }
                        _ => {}
                    }
                }

                // Send heartbeat pings
                _ = heartbeat.tick() => {
                    if session.ping(b"").await.is_err() {
                        break;
                    }
                }

                else => break,
            }
        }

        info!("WebSocket client disconnected");
    });

    Ok(response)
}

/// Handle incoming WebSocket messages from clients
async fn handle_ws_message(
    text: &str,
    state: &web::Data<Arc<AppState>>,
    session: &mut actix_ws::Session,
) {
    if let Ok(msg) = serde_json::from_str::<serde_json::Value>(text) {
        match msg.get("action").and_then(|a| a.as_str()) {
            Some("get_stations") => {
                let stations = state.registry.get_all_stations();
                let response = serde_json::json!({
                    "type": "StationsUpdated",
                    "data": stations
                });
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = session.text(json).await;
                }
            }
            Some("get_status") => {
                let response = serde_json::json!({
                    "type": "PlaybackStatus",
                    "data": state.get_playback_status()
                });
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = session.text(json).await;
                }
            }
            Some("get_metadata") => {
                if let Some(station_id) = msg.get("station_id").and_then(|s| s.as_str()) {
                    if let Ok(id) = uuid::Uuid::parse_str(station_id) {
                        if let Some(metadata) = state.registry.get_metadata(&id) {
                            let response = serde_json::json!({
                                "type": "MetadataUpdated",
                                "data": metadata
                            });
                            if let Ok(json) = serde_json::to_string(&response) {
                                let _ = session.text(json).await;
                            }
                        }
                    }
                }
            }
            Some(action) => {
                warn!("Unknown WS action: {}", action);
            }
            None => {
                debug!("WS message without action: {}", text);
            }
        }
    }
}
