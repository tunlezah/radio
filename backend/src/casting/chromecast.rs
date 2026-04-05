use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::services::app_state::{AppState, CastDeviceInfo};

/// Chromecast device discovered via mDNS
#[derive(Clone, Debug)]
pub struct ChromecastDevice {
    pub id: String,
    pub friendly_name: String,
    pub ip_addr: String,
    pub port: u16,
    pub model: String,
    pub is_connected: bool,
}

/// Chromecast manager - handles discovery and casting
pub struct ChromecastManager {
    devices: parking_lot::RwLock<HashMap<String, ChromecastDevice>>,
    active_session: parking_lot::RwLock<Option<String>>,
}

impl ChromecastManager {
    pub fn new() -> Self {
        Self {
            devices: parking_lot::RwLock::new(HashMap::new()),
            active_session: parking_lot::RwLock::new(None),
        }
    }

    /// Discover Chromecast devices on the local network via mDNS
    pub async fn discover_devices(&self) -> Result<Vec<CastDeviceInfo>> {
        info!("Discovering Chromecast devices via mDNS...");

        // Browse for _googlecast._tcp.local services
        let service_type = "_googlecast._tcp.local.";

        match mdns_sd::ServiceDaemon::new() {
            Ok(mdns) => {
                let receiver = mdns.browse(service_type)?;

                let mut discovered = Vec::new();
                let timeout = std::time::Duration::from_secs(5);
                let start = std::time::Instant::now();

                while start.elapsed() < timeout {
                    match receiver.recv_timeout(std::time::Duration::from_millis(500)) {
                        Ok(event) => match event {
                            mdns_sd::ServiceEvent::ServiceResolved(info) => {
                                let name = info.get_fullname().to_string();
                                let friendly_name = info
                                    .get_properties()
                                    .get("fn")
                                    .map(|v| v.val_str().to_string())
                                    .unwrap_or_else(|| name.clone());

                                let ip = info
                                    .get_addresses()
                                    .iter()
                                    .next()
                                    .map(|a| a.to_string())
                                    .unwrap_or_default();

                                let device = ChromecastDevice {
                                    id: name.clone(),
                                    friendly_name: friendly_name.clone(),
                                    ip_addr: ip,
                                    port: info.get_port(),
                                    model: info
                                        .get_properties()
                                        .get("md")
                                        .map(|v| v.val_str().to_string())
                                        .unwrap_or_else(|| "Chromecast".to_string()),
                                    is_connected: false,
                                };

                                info!("Found Chromecast: {} at {}:{}", friendly_name, device.ip_addr, device.port);
                                self.devices.write().insert(name.clone(), device.clone());

                                discovered.push(CastDeviceInfo {
                                    id: name,
                                    name: friendly_name,
                                    device_type: "chromecast".to_string(),
                                    is_connected: false,
                                });
                            }
                            _ => {}
                        },
                        Err(_) => break,
                    }
                }

                mdns.shutdown()?;
                info!("Chromecast discovery complete: {} devices found", discovered.len());
                Ok(discovered)
            }
            Err(e) => {
                warn!("mDNS service unavailable: {}. Chromecast discovery disabled.", e);
                Ok(vec![])
            }
        }
    }

    /// Cast audio to a specific Chromecast device
    pub async fn cast_to_device(
        &self,
        device_id: &str,
        stream_url: &str,
    ) -> Result<()> {
        let devices = self.devices.read();
        let device = devices
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("Chromecast device not found: {}", device_id))?;

        info!(
            "Casting to {} ({}) - stream: {}",
            device.friendly_name, device.ip_addr, stream_url
        );

        // Google Cast protocol:
        // 1. Connect to device via TLS on port 8009
        // 2. Send CONNECT message to receiver
        // 3. Launch or join media receiver app
        // 4. Send LOAD media command with stream URL

        // For development, simulate the cast connection
        *self.active_session.write() = Some(device_id.to_string());

        info!("Cast session established with {}", device.friendly_name);
        Ok(())
    }

    /// Stop casting to the current device
    pub async fn stop_casting(&self) -> Result<()> {
        if let Some(device_id) = self.active_session.write().take() {
            info!("Stopping cast to device {}", device_id);
        }
        Ok(())
    }

    /// Get list of discovered devices
    pub fn get_devices(&self) -> Vec<CastDeviceInfo> {
        self.devices
            .read()
            .values()
            .map(|d| CastDeviceInfo {
                id: d.id.clone(),
                name: d.friendly_name.clone(),
                device_type: "chromecast".to_string(),
                is_connected: d.is_connected,
            })
            .collect()
    }

    /// Check if currently casting
    pub fn is_casting(&self) -> bool {
        self.active_session.read().is_some()
    }
}
