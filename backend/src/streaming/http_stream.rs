use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{debug, info};

use crate::dsp::decoder::{AUDIO_CHANNELS, AUDIO_BITS_PER_SAMPLE, AUDIO_SAMPLE_RATE};
use crate::services::app_state::AppState;
use crate::streaming::audio_stream::frames_to_wav_header;

/// HTTP audio streaming endpoint
/// Provides low-latency audio via chunked transfer encoding
pub async fn stream_audio(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    info!("New audio stream connection from {:?}", req.peer_addr());

    let wav_header = frames_to_wav_header(AUDIO_SAMPLE_RATE, AUDIO_CHANNELS, AUDIO_BITS_PER_SAMPLE);

    // Create a streaming response with chunked transfer encoding
    let stream = futures::stream::unfold(
        (state, wav_header, true),
        |(state, header, is_first)| async move {
            if is_first {
                // Send WAV header first
                return Some((
                    Ok::<_, actix_web::Error>(bytes::Bytes::from(header.clone())),
                    (state, header, false),
                ));
            }

            // Generate audio data (in production, read from the audio stream manager)
            tokio::time::sleep(Duration::from_millis(20)).await;

            // Generate 20ms of audio (silence or test tone)
            let samples_per_frame = AUDIO_SAMPLE_RATE as usize / 50;
            let mut pcm_bytes = Vec::with_capacity(samples_per_frame * AUDIO_CHANNELS as usize * 2);

            let status = state.get_playback_status();
            if status.is_playing {
                // Generate test tone when playing
                static FRAME_COUNT: std::sync::atomic::AtomicU64 =
                    std::sync::atomic::AtomicU64::new(0);
                let frame_num = FRAME_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                let volume = status.volume;
                for i in 0..samples_per_frame {
                    let t = (frame_num as f64 * samples_per_frame as f64 + i as f64)
                        / AUDIO_SAMPLE_RATE as f64;
                    let sample = (2.0 * std::f64::consts::PI * 440.0 * t).sin();
                    let pcm = (sample * 16384.0 * volume as f64) as i16;

                    // Stereo
                    pcm_bytes.extend_from_slice(&pcm.to_le_bytes());
                    pcm_bytes.extend_from_slice(&pcm.to_le_bytes());
                }
            } else {
                // Silence when not playing
                pcm_bytes.resize(samples_per_frame * AUDIO_CHANNELS as usize * 2, 0);
            }

            Some((
                Ok(bytes::Bytes::from(pcm_bytes)),
                (state, header, false),
            ))
        },
    );

    HttpResponse::Ok()
        .content_type("audio/wav")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Transfer-Encoding", "chunked"))
        .streaming(stream)
}
