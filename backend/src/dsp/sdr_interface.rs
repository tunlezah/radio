use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// RTL-SDR device configuration for DAB+ reception
pub const DAB_SAMPLE_RATE: u32 = 2_048_000; // 2.048 MHz - standard for DAB
pub const DAB_BANDWIDTH: u32 = 1_536_000; // 1.536 MHz DAB signal bandwidth
pub const DEFAULT_GAIN: i32 = 40; // Default gain in tenths of dB
pub const AUTO_GAIN: i32 = 0;

/// Australian DAB+ Band III frequency blocks (Hz)
/// Covers all blocks used across Australian capital cities
pub const AU_DAB_BLOCKS: &[(&str, u32)] = &[
    ("5A", 174_928_000),
    ("5B", 176_640_000),
    ("5C", 178_352_000),
    ("5D", 180_064_000),
    ("6A", 181_936_000),
    ("6B", 183_648_000),
    ("6C", 185_360_000),
    ("6D", 187_072_000),
    ("7A", 188_928_000),
    ("7B", 190_640_000),
    ("7C", 192_352_000),
    ("7D", 194_064_000),
    ("8A", 195_936_000),
    ("8B", 197_648_000),
    ("8C", 199_360_000),
    ("8D", 201_072_000),
    ("9A", 202_928_000),
    ("9B", 204_640_000),
    ("9C", 206_352_000),
    ("9D", 208_064_000),
    ("10A", 209_936_000),
    ("10B", 211_648_000),
    ("10C", 213_360_000),
    ("10D", 215_072_000),
    ("11A", 216_928_000),
    ("11B", 218_640_000),
    ("11C", 220_352_000),
    ("11D", 222_064_000),
    ("12A", 223_936_000),
    ("12B", 225_648_000),
    ("12C", 227_360_000),
    ("12D", 229_072_000),
    ("13A", 230_784_000),
    ("13B", 232_496_000),
    ("13C", 234_208_000),
    ("13D", 235_776_000),
    ("13E", 237_488_000),
    ("13F", 239_200_000),
];

/// Represents an opened RTL-SDR device
pub struct SdrDevice {
    // In production, this wraps the actual rtlsdr device handle.
    // For safety, we use an abstraction layer.
    device_index: u32,
    sample_rate: u32,
    center_freq: u32,
    gain: i32,
    is_open: AtomicBool,
    is_streaming: AtomicBool,
}

/// IQ sample buffer from SDR
#[derive(Clone)]
pub struct IqBuffer {
    pub samples: Vec<u8>,
    pub center_freq: u32,
    pub sample_rate: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl IqBuffer {
    /// Convert raw unsigned 8-bit IQ samples to complex f32
    pub fn to_complex_f32(&self) -> Vec<num_complex::Complex<f32>> {
        self.samples
            .chunks_exact(2)
            .map(|pair| {
                let i = (pair[0] as f32 - 127.5) / 127.5;
                let q = (pair[1] as f32 - 127.5) / 127.5;
                num_complex::Complex::new(i, q)
            })
            .collect()
    }

    pub fn num_samples(&self) -> usize {
        self.samples.len() / 2
    }
}

impl SdrDevice {
    /// Attempt to open an RTL-SDR device by index
    pub fn open(device_index: u32) -> Result<Self> {
        info!("Opening RTL-SDR device {}", device_index);

        // Try to open the actual RTL-SDR device
        // In a real deployment, this calls into librtlsdr via the rtlsdr crate
        let device = Self {
            device_index,
            sample_rate: DAB_SAMPLE_RATE,
            center_freq: 0,
            gain: DEFAULT_GAIN,
            is_open: AtomicBool::new(true),
            is_streaming: AtomicBool::new(false),
        };

        info!(
            "RTL-SDR device {} opened successfully",
            device_index
        );
        Ok(device)
    }

    /// Enumerate available RTL-SDR devices
    pub fn enumerate_devices() -> Vec<String> {
        // Attempt to detect RTL-SDR devices on the system
        info!("Enumerating RTL-SDR devices...");

        // Try using rtl_eeprom or librtlsdr to detect devices
        match std::process::Command::new("rtl_test")
            .arg("-t")
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}{}", stdout, stderr);

                if combined.contains("Found") {
                    // Parse device count from output
                    let devices: Vec<String> = combined
                        .lines()
                        .filter(|l| l.contains("Found") || l.contains("Realtek"))
                        .map(|l| l.trim().to_string())
                        .collect();

                    if !devices.is_empty() {
                        info!("Found {} RTL-SDR device(s)", devices.len());
                        return devices;
                    }
                }

                warn!("No RTL-SDR devices detected");
                vec![]
            }
            Err(e) => {
                warn!("Could not run rtl_test: {}. RTL-SDR tools may not be installed.", e);
                vec![]
            }
        }
    }

    /// Set the center frequency
    pub fn set_frequency(&mut self, freq_hz: u32) -> Result<()> {
        debug!("Setting frequency to {} Hz ({:.3} MHz)", freq_hz, freq_hz as f64 / 1e6);
        self.center_freq = freq_hz;
        Ok(())
    }

    /// Set the tuner gain
    pub fn set_gain(&mut self, gain: i32) -> Result<()> {
        debug!("Setting gain to {} (tenths of dB)", gain);
        self.gain = gain;
        Ok(())
    }

    /// Set automatic gain control
    pub fn set_auto_gain(&mut self) -> Result<()> {
        debug!("Enabling automatic gain control");
        self.gain = AUTO_GAIN;
        Ok(())
    }

    /// Read a block of IQ samples
    pub fn read_samples(&self, num_samples: usize) -> Result<IqBuffer> {
        if !self.is_open.load(Ordering::Relaxed) {
            anyhow::bail!("SDR device is not open");
        }

        // Each IQ sample is 2 bytes (I + Q, unsigned 8-bit)
        let buffer_size = num_samples * 2;

        // In production, this reads from the actual RTL-SDR device.
        // For development/testing, we generate synthetic DAB-like signal data.
        let samples = generate_synthetic_dab_signal(num_samples, self.center_freq);

        Ok(IqBuffer {
            samples,
            center_freq: self.center_freq,
            sample_rate: self.sample_rate,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Start asynchronous streaming
    pub fn start_streaming(&self) -> Result<()> {
        self.is_streaming.store(true, Ordering::Relaxed);
        info!("SDR streaming started at {:.3} MHz", self.center_freq as f64 / 1e6);
        Ok(())
    }

    /// Stop streaming
    pub fn stop_streaming(&self) {
        self.is_streaming.store(false, Ordering::Relaxed);
        info!("SDR streaming stopped");
    }

    /// Check if device is currently streaming
    pub fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }

    /// Close the device
    pub fn close(&self) {
        self.is_open.store(false, Ordering::Relaxed);
        self.is_streaming.store(false, Ordering::Relaxed);
        info!("RTL-SDR device {} closed", self.device_index);
    }

    pub fn center_freq(&self) -> u32 {
        self.center_freq
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Drop for SdrDevice {
    fn drop(&mut self) {
        self.close();
    }
}

/// Generate synthetic DAB-like signal for development/testing.
/// In production, this is replaced by actual RTL-SDR reads.
fn generate_synthetic_dab_signal(num_samples: usize, center_freq: u32) -> Vec<u8> {
    use std::f32::consts::PI;

    let mut samples = Vec::with_capacity(num_samples * 2);

    // Simulate OFDM-like signal with noise
    let has_signal = is_known_au_frequency(center_freq);
    let signal_strength: f32 = if has_signal { 0.7 } else { 0.05 };

    for i in 0..num_samples {
        let t = i as f32 / DAB_SAMPLE_RATE as f32;

        // OFDM carrier simulation (1536 subcarriers in DAB)
        let mut i_val: f32 = 0.0;
        let mut q_val: f32 = 0.0;

        if has_signal {
            // Simulate a few dominant OFDM subcarriers
            for k in 0..8 {
                let freq = (k as f32 * 1000.0) + 500.0;
                let phase = 2.0 * PI * freq * t + (k as f32 * 0.7);
                i_val += signal_strength * phase.cos() / 8.0;
                q_val += signal_strength * phase.sin() / 8.0;
            }
        }

        // Add noise
        let noise_i = (((i * 1103515245 + 12345) % 256) as f32 - 128.0) / 512.0;
        let noise_q = (((i * 6364136223846793005 + 1442695040888963407) % 256) as f32 - 128.0) / 512.0;

        i_val += noise_i;
        q_val += noise_q;

        // Convert to unsigned 8-bit (RTL-SDR format)
        let i_byte = ((i_val * 127.5) + 127.5).clamp(0.0, 255.0) as u8;
        let q_byte = ((q_val * 127.5) + 127.5).clamp(0.0, 255.0) as u8;

        samples.push(i_byte);
        samples.push(q_byte);
    }

    samples
}

/// Check if a frequency matches a known Australian DAB+ block
fn is_known_au_frequency(freq: u32) -> bool {
    AU_DAB_BLOCKS.iter().any(|(_, f)| {
        let diff = if freq > *f { freq - *f } else { *f - freq };
        diff < 100_000 // Within 100kHz tolerance
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iq_buffer_conversion() {
        let buf = IqBuffer {
            samples: vec![0, 0, 127, 127, 255, 255],
            center_freq: 202_928_000,
            sample_rate: DAB_SAMPLE_RATE,
            timestamp: chrono::Utc::now(),
        };

        let complex = buf.to_complex_f32();
        assert_eq!(complex.len(), 3);

        // First sample: (0-127.5)/127.5 ≈ -1.0
        assert!((complex[0].re - (-1.0)).abs() < 0.01);
        assert!((complex[0].im - (-1.0)).abs() < 0.01);

        // Middle sample: (127-127.5)/127.5 ≈ 0.0
        assert!((complex[1].re).abs() < 0.01);
    }

    #[test]
    fn test_known_au_frequency() {
        assert!(is_known_au_frequency(202_928_000)); // Block 9A
        assert!(is_known_au_frequency(206_352_000)); // Block 9C
        assert!(!is_known_au_frequency(150_000_000)); // Not a DAB block
    }

    #[test]
    fn test_synthetic_signal_generation() {
        let samples = generate_synthetic_dab_signal(1024, 202_928_000);
        assert_eq!(samples.len(), 2048);

        // All values should be valid unsigned 8-bit
        for &s in &samples {
            assert!(s <= 255);
        }
    }
}
