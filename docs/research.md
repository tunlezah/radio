# DAB+ Research Document

## Architecture Overview

### DAB+ Technical Foundation

DAB+ (Digital Audio Broadcasting Plus) is the enhanced version of the DAB standard (ETSI EN 300 401), using HE-AAC v2 codec instead of MPEG-1 Audio Layer II for superior audio quality at lower bitrates.

#### Signal Structure
- **Modulation**: OFDM (Orthogonal Frequency-Division Multiplexing)
- **Bandwidth**: 1.536 MHz per ensemble
- **Subcarriers**: 1536 (Mode I, used in Australia)
- **Guard interval**: 246 μs (Mode I)
- **Symbol duration**: 1.246 ms
- **Frame duration**: 96 ms (76 OFDM symbols per frame)

#### DAB Transmission Modes
| Mode | Subcarriers | Frame Duration | Max Tx Distance |
|------|------------|----------------|-----------------|
| I    | 1536       | 96 ms          | 96 km           |
| II   | 384        | 24 ms          | 24 km           |
| III  | 192        | 24 ms          | 12 km           |
| IV   | 768        | 48 ms          | 48 km           |

Australia uses **Mode I** exclusively.

### Australian DAB+ Frequency Allocations

Australia uses **VHF Band III** (174-230 MHz):

| City       | Block | Frequency (MHz) | Ensemble                    |
|------------|-------|------------------|-----------------------------|
| Sydney     | 9A    | 202.928          | ABC/SBS National            |
| Sydney     | 9B    | 204.640          | Commercial Mux 1            |
| Sydney     | 9C    | 206.352          | Commercial Mux 2            |
| Melbourne  | 9C    | 206.352          | ABC/SBS National            |
| Melbourne  | 9D    | 208.064          | Commercial Mux 1            |
| Brisbane   | 9D    | 208.064          | Mixed                       |
| Brisbane   | 10A   | 209.936          | Mixed                       |
| Adelaide   | 10A   | 209.936          | Mixed                       |
| Adelaide   | 10B   | 211.648          | Mixed                       |
| Perth      | 10B   | 211.648          | Mixed                       |
| Perth      | 10C   | 213.360          | Mixed                       |
| Hobart     | 11A   | 216.928          | Mixed                       |
| Darwin     | 11B   | 218.640          | Mixed                       |
| Canberra   | 11C   | 220.352          | Mixed                       |

### DSP Pipeline Architecture

```
RTL-SDR USB → IQ Samples (2.048 MSPS, 8-bit unsigned)
  → DC Offset Removal
  → Frequency Correction (PPM offset)
  → OFDM Synchronization
    → Null Symbol Detection (frame sync)
    → Phase Reference Symbol (fine sync)
  → FFT (2048-point)
  → Differential QPSK Demodulation
  → Frequency De-interleaving
  → Time De-interleaving (15 frames deep)
  → FIC Extraction (Fast Information Channel)
    → Viterbi Decoding (rate 1/4, K=7)
    → Energy Dispersal
    → CRC-16 Check
    → FIG Parsing
  → MSC Extraction (Main Service Channel)
    → Sub-channel Extraction
    → Convolutional Decoding / Reed-Solomon (DAB+)
    → Super Frame Assembly
    → HE-AAC v2 Decoding
    → PCM Audio Output
```

### Metadata Extraction

#### FIG Types (Fast Information Groups)
- **FIG 0/0**: Ensemble information (EId, change flags)
- **FIG 0/1**: Sub-channel organization (start address, size, protection)
- **FIG 0/2**: Service organization (service ID, components)
- **FIG 0/3**: Service component in packet mode
- **FIG 0/8**: Service component global definition
- **FIG 0/13**: User application information (PAD, SlideShow)
- **FIG 0/17**: Programme type (PTy)
- **FIG 1/0**: Ensemble label (16 chars)
- **FIG 1/1**: Service label (16 chars)
- **FIG 1/4**: Service component label

#### DLS (Dynamic Label Segment)
- Carried in PAD (Programme Associated Data)
- Maximum 128 characters per label
- Character sets: EBU Latin (default), UTF-8 (charset 15)
- Toggle bit mechanism for detecting new labels

#### SLS (MOT Slideshow)
- Images transmitted via MOT (Multimedia Object Transfer) protocol
- Supported formats: JPEG, PNG, BMP, GIF
- Recommended resolution: 320x240 (basic), 640x480 (enhanced)
- Segmented delivery with header and body segments

### Reference Implementations Comparison

#### welle.io
- **Language**: C++/Qt
- **Strengths**: Mature, well-tested, complete DAB/DAB+ implementation
- **DSP**: Custom OFDM, Viterbi, Reed-Solomon implementations
- **UI**: Qt-based desktop application
- **API**: Provides library API (welle-cli) for headless operation
- **Weakness**: Tightly coupled to Qt ecosystem

#### Key Reusable Concepts from welle.io
1. OFDM synchronization algorithm (null symbol detection + PRS correlation)
2. FIG parsing state machine
3. Audio super frame assembly and error concealment
4. Signal quality estimation via MER (Modulation Error Ratio)

### Streaming Architecture Recommendations

#### Audio Delivery to Browser
- **HTTP Streaming (Chunked)**: Simplest, ~200ms latency, widest compatibility
- **WebSocket Binary**: Good latency (~100ms), full duplex for metadata
- **WebRTC**: Lowest latency (<50ms) but complex setup for server-to-browser

**Recommendation**: HTTP chunked streaming for audio + WebSocket for metadata.
This provides good latency with maximum compatibility and simplicity.

#### Chromecast Integration
- Protocol: Google Cast v2 over TLS
- Discovery: mDNS `_googlecast._tcp.local.`
- Flow: Connect → Launch Media Receiver → Load audio stream URL
- Audio format: Must serve audio over HTTP URL accessible from Chromecast

#### AirPlay Integration
- Protocol: RAOP (Remote Audio Output Protocol) over RTSP
- Discovery: mDNS `_raop._tcp.local.`
- Audio format: ALAC (Apple Lossless) or AAC in RTP packets
- Encryption: AES-128-CBC for audio data (AirPlay 1)

### Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Weak signals in suburban areas | Multi-pass scanning, adaptive thresholds, longer dwell time |
| RTL-SDR frequency drift | PPM correction, periodic re-calibration |
| Audio dropouts | Frame-level error concealment, buffer management |
| USB bandwidth limitations | Dedicated USB port, no hub sharing |
| Multipath interference | Time/frequency de-interleaving (built into DAB spec) |
| Chromecast network issues | LAN-only operation, mDNS retry logic |
