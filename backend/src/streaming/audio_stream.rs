use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::{debug, info};

use crate::dsp::decoder::AudioFrame;

/// Audio stream manager - handles real-time audio delivery
pub struct AudioStreamManager {
    /// Channel for sending audio frames to consumers
    frame_sender: Sender<AudioFrame>,
    frame_receiver: Arc<Mutex<Receiver<AudioFrame>>>,
    /// Current stream state
    is_active: Arc<std::sync::atomic::AtomicBool>,
    /// Stream statistics
    frames_delivered: Arc<std::sync::atomic::AtomicU64>,
    buffer_underruns: Arc<std::sync::atomic::AtomicU64>,
}

impl AudioStreamManager {
    pub fn new() -> Self {
        // Buffer up to 50 frames (1 second at 20ms/frame)
        let (sender, receiver) = bounded(50);

        Self {
            frame_sender: sender,
            frame_receiver: Arc::new(Mutex::new(receiver)),
            is_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            frames_delivered: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            buffer_underruns: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Start the audio stream
    pub fn start(&self) {
        self.is_active
            .store(true, std::sync::atomic::Ordering::Relaxed);
        info!("Audio stream started");
    }

    /// Stop the audio stream
    pub fn stop(&self) {
        self.is_active
            .store(false, std::sync::atomic::Ordering::Relaxed);
        // Drain remaining frames
        let rx = self.frame_receiver.lock();
        while rx.try_recv().is_ok() {}
        info!("Audio stream stopped");
    }

    /// Push a decoded audio frame into the stream
    pub fn push_frame(&self, frame: AudioFrame) -> Result<()> {
        if !self.is_active.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }

        match self.frame_sender.try_send(frame) {
            Ok(()) => {
                self.frames_delivered
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                // Drop oldest frame to prevent blocking
                let rx = self.frame_receiver.lock();
                let _ = rx.try_recv();
                drop(rx);
                self.buffer_underruns
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                debug!("Audio buffer full, dropped oldest frame");
                Ok(())
            }
            Err(e) => anyhow::bail!("Failed to send audio frame: {}", e),
        }
    }

    /// Get the next audio frame (blocking with timeout)
    pub fn next_frame(&self, timeout: std::time::Duration) -> Option<AudioFrame> {
        let rx = self.frame_receiver.lock();
        rx.recv_timeout(timeout).ok()
    }

    /// Get stream statistics
    pub fn stats(&self) -> StreamStats {
        StreamStats {
            is_active: self.is_active.load(std::sync::atomic::Ordering::Relaxed),
            frames_delivered: self
                .frames_delivered
                .load(std::sync::atomic::Ordering::Relaxed),
            buffer_underruns: self
                .buffer_underruns
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct StreamStats {
    pub is_active: bool,
    pub frames_delivered: u64,
    pub buffer_underruns: u64,
}

/// Convert audio frames to WAV format for HTTP streaming
pub fn frames_to_wav_header(sample_rate: u32, channels: u16, bits_per_sample: u16) -> Vec<u8> {
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;

    let mut header = Vec::with_capacity(44);

    // RIFF header
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // Streaming: unknown size
    header.extend_from_slice(b"WAVE");

    // fmt chunk
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes()); // Chunk size
    header.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    header.extend_from_slice(&channels.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&byte_rate.to_le_bytes());
    header.extend_from_slice(&block_align.to_le_bytes());
    header.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    header.extend_from_slice(b"data");
    header.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // Streaming: unknown size

    header
}
