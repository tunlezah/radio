use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Audio output format
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;
pub const AUDIO_CHANNELS: u16 = 2;
pub const AUDIO_BITS_PER_SAMPLE: u16 = 16;

/// Decoded audio frame
#[derive(Clone)]
pub struct AudioFrame {
    pub pcm_data: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp_ms: u64,
    pub frame_number: u64,
}

impl AudioFrame {
    /// Convert PCM i16 to f32 samples normalized to [-1.0, 1.0]
    pub fn to_f32(&self) -> Vec<f32> {
        self.pcm_data
            .iter()
            .map(|&s| s as f32 / 32768.0)
            .collect()
    }

    /// Convert to WAV-compatible byte buffer
    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pcm_data.len() * 2);
        for &sample in &self.pcm_data {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }

    /// Duration in milliseconds
    pub fn duration_ms(&self) -> f64 {
        (self.pcm_data.len() as f64 / self.channels as f64) / self.sample_rate as f64 * 1000.0
    }
}

/// DAB+ Audio Decoder
/// Handles HE-AAC v2 decoding from DAB+ audio super frames
pub struct DabDecoder {
    service_id: u32,
    sub_channel_id: u8,
    frame_counter: u64,
    is_decoding: bool,
    // Superframe assembly buffer
    superframe_buffer: Vec<u8>,
    superframe_size: usize,
    // Reed-Solomon error correction state
    rs_errors_corrected: u64,
    rs_errors_uncorrectable: u64,
}

impl DabDecoder {
    pub fn new(service_id: u32, sub_channel_id: u8) -> Self {
        info!(
            "Creating DAB+ decoder for service 0x{:04X}, sub-channel {}",
            service_id, sub_channel_id
        );

        Self {
            service_id,
            sub_channel_id,
            frame_counter: 0,
            is_decoding: false,
            superframe_buffer: Vec::with_capacity(65536),
            superframe_size: 0,
            rs_errors_corrected: 0,
            rs_errors_uncorrectable: 0,
        }
    }

    /// Start decoding
    pub fn start(&mut self) -> Result<()> {
        info!("Starting decoder for service 0x{:04X}", self.service_id);
        self.is_decoding = true;
        self.frame_counter = 0;
        self.superframe_buffer.clear();
        Ok(())
    }

    /// Stop decoding
    pub fn stop(&mut self) {
        info!("Stopping decoder for service 0x{:04X}", self.service_id);
        self.is_decoding = false;
    }

    /// Process a DAB+ audio super frame
    /// A DAB+ super frame contains 5 audio frames with Reed-Solomon protection
    pub fn process_superframe(&mut self, data: &[u8]) -> Result<Vec<AudioFrame>> {
        if !self.is_decoding {
            anyhow::bail!("Decoder is not active");
        }

        // DAB+ super frame structure:
        // - Header (2 bytes): fire code for synchronization
        // - Audio data: 5 AU (Access Units) of HE-AAC v2
        // - Reed-Solomon parity bytes (last 24 bytes per CIF)

        // Verify minimum size
        if data.len() < 24 {
            warn!("Superframe too small: {} bytes", data.len());
            return Ok(vec![]);
        }

        // Check fire code for frame synchronization
        let fire_code = ((data[0] as u16) << 8) | data[1] as u16;
        let dac_rate = (fire_code >> 14) & 1;
        let sbr_flag = (fire_code >> 13) & 1;
        let aac_channel_mode = (fire_code >> 12) & 1;
        let ps_flag = (fire_code >> 11) & 1;

        debug!(
            "Superframe: dac_rate={}, sbr={}, channels={}, ps={}",
            dac_rate, sbr_flag, aac_channel_mode, ps_flag
        );

        // Extract Access Unit boundaries from the header
        let num_aus = if dac_rate == 1 && sbr_flag == 1 {
            2
        } else if dac_rate == 1 || sbr_flag == 1 {
            3
        } else {
            4
        };

        // Apply Reed-Solomon error correction
        let corrected_data = self.apply_reed_solomon(data)?;

        // Decode each Access Unit
        let mut frames = Vec::new();
        let frame_size = (corrected_data.len() - 2) / num_aus;

        for i in 0..num_aus {
            let offset = 2 + (i * frame_size);
            let end = offset + frame_size;

            if end > corrected_data.len() {
                break;
            }

            let au_data = &corrected_data[offset..end];

            match self.decode_aac_frame(au_data) {
                Ok(frame) => frames.push(frame),
                Err(e) => {
                    debug!("Failed to decode AU {}: {}", i, e);
                    // Generate silence frame as concealment
                    frames.push(self.generate_silence_frame());
                }
            }
        }

        Ok(frames)
    }

    /// Decode a single HE-AAC v2 Access Unit to PCM audio
    fn decode_aac_frame(&mut self, _au_data: &[u8]) -> Result<AudioFrame> {
        self.frame_counter += 1;

        // In production, this uses the symphonia AAC decoder or fdkaac.
        // For development, generate a test tone.
        let samples_per_frame = AUDIO_SAMPLE_RATE as usize / 50; // 20ms frames
        let total_samples = samples_per_frame * AUDIO_CHANNELS as usize;

        let mut pcm_data = Vec::with_capacity(total_samples);
        let freq = 440.0; // A4 test tone

        for i in 0..samples_per_frame {
            let t = (self.frame_counter as f64 * samples_per_frame as f64 + i as f64)
                / AUDIO_SAMPLE_RATE as f64;
            let sample = (2.0 * std::f64::consts::PI * freq * t).sin();
            let pcm = (sample * 16384.0) as i16; // -6dB test tone

            // Stereo: same sample on both channels
            pcm_data.push(pcm);
            pcm_data.push(pcm);
        }

        Ok(AudioFrame {
            pcm_data,
            sample_rate: AUDIO_SAMPLE_RATE,
            channels: AUDIO_CHANNELS,
            timestamp_ms: self.frame_counter * 20, // 20ms per frame
            frame_number: self.frame_counter,
        })
    }

    /// Apply Reed-Solomon error correction to super frame data
    fn apply_reed_solomon(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        // RS(120,110) - corrects up to 5 byte errors per block
        // In production, this uses a proper RS decoder.
        // For now, pass through (assuming no errors in synthetic data).
        self.rs_errors_corrected += 0;
        Ok(data.to_vec())
    }

    /// Generate a silence frame for error concealment
    fn generate_silence_frame(&mut self) -> AudioFrame {
        self.frame_counter += 1;
        let samples_per_frame = AUDIO_SAMPLE_RATE as usize / 50;
        let total_samples = samples_per_frame * AUDIO_CHANNELS as usize;

        AudioFrame {
            pcm_data: vec![0i16; total_samples],
            sample_rate: AUDIO_SAMPLE_RATE,
            channels: AUDIO_CHANNELS,
            timestamp_ms: self.frame_counter * 20,
            frame_number: self.frame_counter,
        }
    }

    pub fn is_decoding(&self) -> bool {
        self.is_decoding
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_counter
    }

    pub fn rs_stats(&self) -> (u64, u64) {
        (self.rs_errors_corrected, self.rs_errors_uncorrectable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_frame_to_f32() {
        let frame = AudioFrame {
            pcm_data: vec![0, 16384, -16384, 32767],
            sample_rate: 48000,
            channels: 2,
            timestamp_ms: 0,
            frame_number: 0,
        };

        let f32_data = frame.to_f32();
        assert_eq!(f32_data.len(), 4);
        assert!((f32_data[0]).abs() < 0.001);
        assert!((f32_data[1] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_decoder_lifecycle() {
        let mut decoder = DabDecoder::new(0x1001, 0);
        assert!(!decoder.is_decoding());

        decoder.start().unwrap();
        assert!(decoder.is_decoding());

        decoder.stop();
        assert!(!decoder.is_decoding());
    }

    #[test]
    fn test_silence_frame() {
        let mut decoder = DabDecoder::new(0x1001, 0);
        let frame = decoder.generate_silence_frame();

        assert!(frame.pcm_data.iter().all(|&s| s == 0));
        assert_eq!(frame.sample_rate, 48000);
        assert_eq!(frame.channels, 2);
    }
}
