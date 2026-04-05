use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::services::app_state::AppState;

/// Background signal monitoring service
/// Periodically checks signal quality and triggers re-scans if needed
pub async fn run_signal_monitor(state: Arc<AppState>) {
    info!("Signal monitor started");

    let mut interval = tokio::time::interval(Duration::from_secs(30));
    let mut check_count: u64 = 0;

    loop {
        interval.tick().await;
        check_count += 1;

        // Check signal quality for the currently playing station
        if let Some(current_station_id) = state.get_current_station_id() {
            if let Some(station) = state.registry.get_station(&current_station_id) {
                debug!(
                    "Signal check #{}: {} - strength: {:.1}%",
                    check_count,
                    station.name,
                    station.signal_strength * 100.0
                );

                // Update metadata with simulated DLS
                let dls_text = crate::services::metadata_parser::get_simulated_dls(station.service_id);
                state.registry.update_dls(current_station_id, dls_text);

                // Check for signal degradation
                if station.signal_strength < 0.1 {
                    warn!(
                        "Weak signal for station {}: {:.1}%",
                        station.name,
                        station.signal_strength * 100.0
                    );

                    // Notify frontend via WebSocket
                    state.broadcast_log(format!(
                        "⚠ Weak signal: {} ({:.0}%)",
                        station.name,
                        station.signal_strength * 100.0
                    )).await;
                }
            }
        }

        // Periodic status broadcast
        if check_count % 10 == 0 {
            let count = state.registry.station_count();
            debug!("Signal monitor: {} stations in registry", count);
        }
    }
}

/// Signal quality assessment
#[derive(Clone, Debug, serde::Serialize)]
pub struct SignalQuality {
    pub strength_percent: f32,
    pub snr_db: f32,
    pub bit_error_rate: f32,
    pub quality_level: QualityLevel,
}

#[derive(Clone, Debug, serde::Serialize)]
pub enum QualityLevel {
    Excellent,
    Good,
    Fair,
    Poor,
    NoSignal,
}

impl SignalQuality {
    pub fn from_signal_score(score: f32, snr: f32) -> Self {
        let quality_level = match score {
            s if s >= 0.8 => QualityLevel::Excellent,
            s if s >= 0.6 => QualityLevel::Good,
            s if s >= 0.3 => QualityLevel::Fair,
            s if s > 0.0 => QualityLevel::Poor,
            _ => QualityLevel::NoSignal,
        };

        Self {
            strength_percent: score * 100.0,
            snr_db: snr,
            bit_error_rate: (1.0 - score) * 0.001,
            quality_level,
        }
    }
}
