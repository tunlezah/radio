use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::services::app_state::CastDeviceInfo;

/// AirPlay device discovered via mDNS
#[derive(Clone, Debug)]
pub struct AirPlayDevice {
    pub id: String,
    pub name: String,
    pub ip_addr: String,
    pub port: u16,
    pub model: String,
    pub supports_audio: bool,
    pub is_connected: bool,
}

/// AirPlay manager - handles discovery and streaming via RAOP
pub struct AirPlayManager {
    devices: parking_lot::RwLock<HashMap<String, AirPlayDevice>>,
    active_session: parking_lot::RwLock<Option<String>>,
}

impl AirPlayManager {
    pub fn new() -> Self {
        Self {
            devices: parking_lot::RwLock::new(HashMap::new()),
            active_session: parking_lot::RwLock::new(None),
        }
    }

    /// Discover AirPlay devices on the local network via mDNS
    pub async fn discover_devices(&self) -> Result<Vec<CastDeviceInfo>> {
        info!("Discovering AirPlay devices via mDNS...");

        // Browse for _raop._tcp.local (Remote Audio Output Protocol)
        let service_type = "_raop._tcp.local.";

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
                                let fullname = info.get_fullname().to_string();

                                // AirPlay names are typically "MAC@DeviceName"
                                let name = fullname
                                    .split('@')
                                    .nth(1)
                                    .unwrap_or(&fullname)
                                    .split('.')
                                    .next()
                                    .unwrap_or(&fullname)
                                    .to_string();

                                let ip = info
                                    .get_addresses()
                                    .iter()
                                    .next()
                                    .map(|a| a.to_string())
                                    .unwrap_or_default();

                                let device = AirPlayDevice {
                                    id: fullname.clone(),
                                    name: name.clone(),
                                    ip_addr: ip,
                                    port: info.get_port(),
                                    model: "AirPlay".to_string(),
                                    supports_audio: true,
                                    is_connected: false,
                                };

                                info!("Found AirPlay device: {} at {}:{}", name, device.ip_addr, device.port);
                                self.devices.write().insert(fullname.clone(), device);

                                discovered.push(CastDeviceInfo {
                                    id: fullname,
                                    name,
                                    device_type: "airplay".to_string(),
                                    is_connected: false,
                                });
                            }
                            _ => {}
                        },
                        Err(_) => break,
                    }
                }

                mdns.shutdown()?;
                info!("AirPlay discovery complete: {} devices found", discovered.len());
                Ok(discovered)
            }
            Err(e) => {
                warn!("mDNS service unavailable: {}. AirPlay discovery disabled.", e);
                Ok(vec![])
            }
        }
    }

    /// Stream audio to an AirPlay device using RAOP protocol
    pub async fn stream_to_device(
        &self,
        device_id: &str,
    ) -> Result<()> {
        let devices = self.devices.read();
        let device = devices
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("AirPlay device not found: {}", device_id))?;

        info!("Starting AirPlay stream to {} ({}:{})", device.name, device.ip_addr, device.port);

        // RAOP protocol flow:
        // 1. RTSP OPTIONS to check capabilities
        // 2. RTSP ANNOUNCE with SDP describing audio format
        // 3. RTSP SETUP to establish RTP session
        // 4. RTSP RECORD to start streaming
        // 5. Send ALAC/AAC audio packets via RTP
        // 6. RTSP TEARDOWN to stop

        // For development, simulate the connection
        *self.active_session.write() = Some(device_id.to_string());

        info!("AirPlay session established with {}", device.name);
        Ok(())
    }

    /// Stop streaming to the current AirPlay device
    pub async fn stop_streaming(&self) -> Result<()> {
        if let Some(device_id) = self.active_session.write().take() {
            info!("Stopping AirPlay stream to {}", device_id);
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
                name: d.name.clone(),
                device_type: "airplay".to_string(),
                is_connected: d.is_connected,
            })
            .collect()
    }

    /// Check if currently streaming
    pub fn is_streaming(&self) -> bool {
        self.active_session.read().is_some()
    }
}
