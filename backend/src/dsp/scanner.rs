use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use super::ensemble_parser::EnsembleInfo;
use super::sdr_interface::{SdrDevice, AU_DAB_BLOCKS, DAB_SAMPLE_RATE, IqBuffer};
use crate::services::station_registry::Station;

/// Scan configuration
#[derive(Clone, Debug)]
pub struct ScanConfig {
    /// Number of scan passes (more = better detection of weak signals)
    pub num_passes: u32,
    /// Samples to collect per frequency for analysis
    pub samples_per_freq: usize,
    /// Minimum signal score to consider a frequency active (0.0 - 1.0)
    pub min_signal_score: f32,
    /// Whether to use adaptive thresholds based on noise floor
    pub adaptive_threshold: bool,
    /// Dwell time per frequency in milliseconds
    pub dwell_time_ms: u64,
    /// Retry weak signals with longer dwell time
    pub retry_weak_signals: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            num_passes: 3,
            samples_per_freq: DAB_SAMPLE_RATE as usize, // 1 second of samples
            min_signal_score: 0.15,
            adaptive_threshold: true,
            dwell_time_ms: 1000,
            retry_weak_signals: true,
        }
    }
}

/// Result from scanning a single frequency block
#[derive(Clone, Debug)]
pub struct BlockScanResult {
    pub block_name: String,
    pub frequency: u32,
    pub signal_score: f32,
    pub noise_floor: f32,
    pub snr_db: f32,
    pub ensemble: Option<EnsembleInfo>,
    pub stations: Vec<Station>,
    pub scan_duration: Duration,
}

/// Result from a complete scan
#[derive(Clone, Debug)]
pub struct ScanResult {
    pub blocks: Vec<BlockScanResult>,
    pub total_stations: usize,
    pub total_ensembles: usize,
    pub scan_duration: Duration,
    pub passes_completed: u32,
}

/// Callback for scan progress updates
pub type ScanProgressCallback = Box<dyn Fn(ScanProgress) + Send + Sync>;

#[derive(Clone, Debug, serde::Serialize)]
pub struct ScanProgress {
    pub current_block: String,
    pub current_frequency: u32,
    pub blocks_scanned: usize,
    pub total_blocks: usize,
    pub current_pass: u32,
    pub total_passes: u32,
    pub stations_found: usize,
    pub percent_complete: f32,
}

/// DAB+ Scanner - handles multi-pass scanning of Band III
pub struct DabScanner {
    config: ScanConfig,
}

impl DabScanner {
    pub fn new(config: ScanConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(ScanConfig::default())
    }

    /// Perform a complete scan of all Australian DAB+ blocks
    pub async fn full_scan(
        &self,
        progress_cb: Option<ScanProgressCallback>,
    ) -> Result<ScanResult> {
        let start = Instant::now();
        info!("Starting full DAB+ scan ({} passes, {} blocks)",
            self.config.num_passes, AU_DAB_BLOCKS.len());

        let mut all_results: Vec<BlockScanResult> = Vec::new();
        let mut weak_blocks: Vec<(&str, u32)> = Vec::new();

        for pass in 0..self.config.num_passes {
            info!("Scan pass {}/{}", pass + 1, self.config.num_passes);

            let blocks_to_scan = if pass == 0 {
                AU_DAB_BLOCKS.to_vec()
            } else {
                // Subsequent passes focus on blocks where signal was detected
                // or where weak signals were found
                let mut blocks: Vec<(&str, u32)> = all_results
                    .iter()
                    .filter(|r| r.signal_score > self.config.min_signal_score * 0.5)
                    .map(|r| {
                        AU_DAB_BLOCKS
                            .iter()
                            .find(|(_, f)| *f == r.frequency)
                            .copied()
                            .unwrap_or(("", r.frequency))
                    })
                    .collect();

                if self.config.retry_weak_signals {
                    blocks.extend(weak_blocks.iter().copied());
                }

                blocks.dedup_by_key(|b| b.1);
                blocks
            };

            for (idx, (block_name, freq)) in blocks_to_scan.iter().enumerate() {
                if let Some(ref cb) = progress_cb {
                    let total_blocks = AU_DAB_BLOCKS.len();
                    let overall_progress = (pass as f32 * total_blocks as f32 + idx as f32)
                        / (self.config.num_passes as f32 * total_blocks as f32);

                    cb(ScanProgress {
                        current_block: block_name.to_string(),
                        current_frequency: *freq,
                        blocks_scanned: idx,
                        total_blocks,
                        current_pass: pass + 1,
                        total_passes: self.config.num_passes,
                        stations_found: all_results.iter().map(|r| r.stations.len()).sum(),
                        percent_complete: overall_progress * 100.0,
                    });
                }

                match self.scan_block(block_name, *freq).await {
                    Ok(result) => {
                        if result.signal_score > self.config.min_signal_score * 0.5
                            && result.signal_score < self.config.min_signal_score
                        {
                            weak_blocks.push((block_name, *freq));
                        }

                        // Merge or update results
                        if let Some(existing) = all_results
                            .iter_mut()
                            .find(|r| r.frequency == *freq)
                        {
                            // Keep the better result
                            if result.signal_score > existing.signal_score {
                                *existing = result;
                            }
                        } else if result.signal_score > self.config.min_signal_score * 0.3 {
                            all_results.push(result);
                        }
                    }
                    Err(e) => {
                        warn!("Error scanning block {} ({} MHz): {}",
                            block_name, *freq as f64 / 1e6, e);
                    }
                }
            }
        }

        // Filter results to only include blocks with sufficient signal
        let active_blocks: Vec<BlockScanResult> = all_results
            .into_iter()
            .filter(|r| r.signal_score >= self.config.min_signal_score)
            .collect();

        let total_stations: usize = active_blocks.iter().map(|b| b.stations.len()).sum();
        let total_ensembles = active_blocks.iter().filter(|b| b.ensemble.is_some()).count();

        let result = ScanResult {
            blocks: active_blocks,
            total_stations,
            total_ensembles,
            scan_duration: start.elapsed(),
            passes_completed: self.config.num_passes,
        };

        info!("Scan complete: {} ensembles, {} stations in {:?}",
            result.total_ensembles, result.total_stations, result.scan_duration);

        Ok(result)
    }

    /// Scan a single frequency block
    async fn scan_block(&self, block_name: &str, freq: u32) -> Result<BlockScanResult> {
        let start = Instant::now();
        debug!("Scanning block {} at {:.3} MHz", block_name, freq as f64 / 1e6);

        // Collect IQ samples at this frequency
        let iq_data = self.collect_samples(freq).await?;

        // Analyze signal quality
        let (signal_score, noise_floor, snr_db) = analyze_signal_quality(&iq_data);

        // Try to detect ensemble if signal is strong enough
        let (ensemble, stations) = if signal_score > self.config.min_signal_score {
            let ensemble_info = super::ensemble_parser::parse_ensemble(&iq_data)?;
            let stations = if let Some(ref ens) = ensemble_info {
                ens.services
                    .iter()
                    .map(|svc| Station {
                        id: uuid::Uuid::new_v4(),
                        name: svc.label.clone(),
                        ensemble_name: ens.label.clone(),
                        ensemble_id: ens.ensemble_id,
                        service_id: svc.service_id,
                        frequency: freq,
                        block_name: block_name.to_string(),
                        signal_strength: signal_score,
                        program_type: svc.program_type.clone(),
                        is_active: true,
                        bitrate: svc.bitrate,
                        codec: "HE-AAC v2".to_string(),
                        last_seen: chrono::Utc::now(),
                    })
                    .collect()
            } else {
                vec![]
            };
            (ensemble_info, stations)
        } else {
            (None, vec![])
        };

        Ok(BlockScanResult {
            block_name: block_name.to_string(),
            frequency: freq,
            signal_score,
            noise_floor,
            snr_db,
            ensemble,
            stations,
            scan_duration: start.elapsed(),
        })
    }

    /// Collect IQ samples at a given frequency
    async fn collect_samples(&self, freq: u32) -> Result<IqBuffer> {
        // In production, this tunes the SDR and reads samples.
        // For development, we use the synthetic signal generator.
        let mut device = SdrDevice::open(0)?;
        device.set_frequency(freq)?;
        device.set_auto_gain()?;

        // Allow tuner to settle
        tokio::time::sleep(Duration::from_millis(50)).await;

        let samples = device.read_samples(self.config.samples_per_freq)?;
        Ok(samples)
    }
}

/// Analyze signal quality from IQ samples
fn analyze_signal_quality(iq_data: &IqBuffer) -> (f32, f32, f32) {
    let complex = iq_data.to_complex_f32();

    if complex.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    // Calculate power spectrum
    let powers: Vec<f32> = complex.iter().map(|c| c.norm_sqr()).collect();
    let mean_power: f32 = powers.iter().sum::<f32>() / powers.len() as f32;

    // Estimate noise floor (lower quartile of power values)
    let mut sorted_powers = powers.clone();
    sorted_powers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let noise_floor = sorted_powers[sorted_powers.len() / 4];

    // Calculate SNR
    let signal_power = mean_power - noise_floor;
    let snr_db = if noise_floor > 0.0 {
        10.0 * (signal_power / noise_floor).log10()
    } else {
        0.0
    };

    // Normalize signal score to 0.0 - 1.0
    let signal_score = (snr_db / 30.0).clamp(0.0, 1.0);

    (signal_score, noise_floor, snr_db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_config_defaults() {
        let config = ScanConfig::default();
        assert_eq!(config.num_passes, 3);
        assert!(config.adaptive_threshold);
        assert!(config.retry_weak_signals);
    }

    #[tokio::test]
    async fn test_scan_block() {
        let scanner = DabScanner::with_default_config();
        let result = scanner.scan_block("9A", 202_928_000).await;
        assert!(result.is_ok());

        let block = result.unwrap();
        assert_eq!(block.block_name, "9A");
        assert_eq!(block.frequency, 202_928_000);
    }
}
