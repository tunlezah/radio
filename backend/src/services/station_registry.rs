use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

/// A discovered DAB+ station
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Station {
    pub id: Uuid,
    pub name: String,
    pub ensemble_name: String,
    pub ensemble_id: u16,
    pub service_id: u32,
    pub frequency: u32,
    pub block_name: String,
    pub signal_strength: f32,
    pub program_type: String,
    pub is_active: bool,
    pub bitrate: u32,
    pub codec: String,
    pub last_seen: DateTime<Utc>,
}

/// Dynamic Label Segment (scrolling text metadata)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DlsInfo {
    pub text: String,
    pub charset: u8,
    pub updated_at: DateTime<Utc>,
}

/// Slideshow (MOT image) metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlsInfo {
    pub content_type: String,
    pub image_data_base64: String,
    pub width: u32,
    pub height: u32,
    pub updated_at: DateTime<Utc>,
}

/// Full metadata for a playing station
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StationMetadata {
    pub station_id: Uuid,
    pub dls: Option<DlsInfo>,
    pub sls: Option<SlsInfo>,
    pub signal_quality: f32,
    pub bit_error_rate: f32,
    pub audio_level_db: f32,
}

/// Thread-safe station registry
pub struct StationRegistry {
    stations: RwLock<HashMap<Uuid, Station>>,
    metadata: RwLock<HashMap<Uuid, StationMetadata>>,
}

impl StationRegistry {
    pub fn new() -> Self {
        Self {
            stations: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
        }
    }

    /// Add or update a station
    pub fn upsert_station(&self, station: Station) {
        let id = station.id;
        let name = station.name.clone();

        let mut stations = self.stations.write();

        // Check if station already exists (by service_id + frequency)
        let existing = stations
            .values()
            .find(|s| s.service_id == station.service_id && s.frequency == station.frequency)
            .map(|s| s.id);

        if let Some(existing_id) = existing {
            // Update existing station
            if let Some(s) = stations.get_mut(&existing_id) {
                s.signal_strength = station.signal_strength;
                s.is_active = station.is_active;
                s.last_seen = station.last_seen;
            }
        } else {
            info!("New station discovered: {} ({})", name, station.ensemble_name);
            stations.insert(id, station);
        }
    }

    /// Get all stations
    pub fn get_all_stations(&self) -> Vec<Station> {
        let stations = self.stations.read();
        let mut result: Vec<Station> = stations.values().cloned().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// Get a station by ID
    pub fn get_station(&self, id: &Uuid) -> Option<Station> {
        self.stations.read().get(id).cloned()
    }

    /// Find station by service ID
    pub fn find_by_service_id(&self, service_id: u32) -> Option<Station> {
        self.stations
            .read()
            .values()
            .find(|s| s.service_id == service_id)
            .cloned()
    }

    /// Update metadata for a station
    pub fn update_metadata(&self, station_id: Uuid, metadata: StationMetadata) {
        self.metadata.write().insert(station_id, metadata);
    }

    /// Update DLS text for a station
    pub fn update_dls(&self, station_id: Uuid, text: String) {
        let mut meta = self.metadata.write();
        let entry = meta.entry(station_id).or_insert_with(|| StationMetadata {
            station_id,
            dls: None,
            sls: None,
            signal_quality: 0.0,
            bit_error_rate: 0.0,
            audio_level_db: -60.0,
        });

        entry.dls = Some(DlsInfo {
            text,
            charset: 0,
            updated_at: Utc::now(),
        });
    }

    /// Get metadata for a station
    pub fn get_metadata(&self, station_id: &Uuid) -> Option<StationMetadata> {
        self.metadata.read().get(station_id).cloned()
    }

    /// Get station count
    pub fn station_count(&self) -> usize {
        self.stations.read().len()
    }

    /// Remove stale stations (not seen in given duration)
    pub fn prune_stale(&self, max_age: chrono::Duration) {
        let cutoff = Utc::now() - max_age;
        let mut stations = self.stations.write();
        let before = stations.len();

        stations.retain(|_, s| s.last_seen > cutoff);

        let removed = before - stations.len();
        if removed > 0 {
            warn!("Pruned {} stale stations", removed);
        }
    }

    /// Clear all stations (for fresh scan)
    pub fn clear(&self) {
        self.stations.write().clear();
        self.metadata.write().clear();
        info!("Station registry cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_station(name: &str, service_id: u32) -> Station {
        Station {
            id: Uuid::new_v4(),
            name: name.to_string(),
            ensemble_name: "Test Ensemble".to_string(),
            ensemble_id: 1000,
            service_id,
            frequency: 202_928_000,
            block_name: "9A".to_string(),
            signal_strength: 0.8,
            program_type: "PopMusic".to_string(),
            is_active: true,
            bitrate: 64,
            codec: "HE-AAC v2".to_string(),
            last_seen: Utc::now(),
        }
    }

    #[test]
    fn test_add_and_retrieve() {
        let registry = StationRegistry::new();
        let station = make_station("triple j", 0x1001);
        let id = station.id;

        registry.upsert_station(station);
        assert_eq!(registry.station_count(), 1);

        let retrieved = registry.get_station(&id).unwrap();
        assert_eq!(retrieved.name, "triple j");
    }

    #[test]
    fn test_get_all_sorted() {
        let registry = StationRegistry::new();
        registry.upsert_station(make_station("ZZZ", 0x1001));
        registry.upsert_station(make_station("AAA", 0x1002));
        registry.upsert_station(make_station("MMM", 0x1003));

        let all = registry.get_all_stations();
        assert_eq!(all[0].name, "AAA");
        assert_eq!(all[1].name, "MMM");
        assert_eq!(all[2].name, "ZZZ");
    }

    #[test]
    fn test_update_dls() {
        let registry = StationRegistry::new();
        let station = make_station("triple j", 0x1001);
        let id = station.id;
        registry.upsert_station(station);

        registry.update_dls(id, "Now Playing: Test Song".to_string());

        let meta = registry.get_metadata(&id).unwrap();
        assert_eq!(meta.dls.unwrap().text, "Now Playing: Test Song");
    }
}
