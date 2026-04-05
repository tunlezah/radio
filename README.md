# DAB+ Digital Radio Web Application

A commercial-grade DAB+ digital radio web application for Australian stations. Scans and discovers all DAB+ stations, decodes and streams audio locally, extracts full metadata (DLS, SLS), and supports Chromecast + AirPlay casting.

## Features

- **Full DAB+ Scanning** - Multi-pass scanning of all Australian Band III frequency blocks (174-240 MHz)
- **Audio Decoding** - HE-AAC v2 decoding pipeline with Reed-Solomon error correction
- **Rich Metadata** - Real-time DLS (Dynamic Label Segment) text and SLS (Slideshow) images
- **Modern UI** - Single-screen responsive interface with dark/light/system themes
- **Chromecast & AirPlay** - Server-side casting via mDNS discovery (LAN-only)
- **Low-Latency Streaming** - HTTP chunked audio streaming with WebSocket metadata updates
- **Robust Installer** - Fully idempotent installer that handles dependencies, repairs, and upgrades

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (React + Tailwind)           │
│  ┌──────────┐ ┌─────────────┐ ┌──────────┐ ┌────────┐ │
│  │ Station   │ │ Now Playing │ │ Controls │ │  Logs  │ │
│  │ List      │ │ + Metadata  │ │ + Volume │ │ Panel  │ │
│  └──────────┘ └─────────────┘ └──────────┘ └────────┘ │
├─────────────────────────────────────────────────────────┤
│                REST API + WebSocket                      │
├─────────────────────────────────────────────────────────┤
│                  Backend (Rust + Actix)                   │
│  ┌──────┐ ┌──────────┐ ┌───────────┐ ┌──────────────┐ │
│  │ DSP  │ │ Services │ │ Streaming │ │   Casting    │ │
│  │      │ │          │ │           │ │              │ │
│  │ SDR  │ │ Station  │ │ HTTP      │ │ Chromecast   │ │
│  │ Scan │ │ Registry │ │ Audio     │ │ AirPlay      │ │
│  │ OFDM │ │ Metadata │ │ Stream    │ │ mDNS         │ │
│  │ AAC  │ │ Signal   │ │           │ │              │ │
│  └──────┘ └──────────┘ └───────────┘ └──────────────┘ │
├─────────────────────────────────────────────────────────┤
│                    RTL-SDR Hardware                       │
└─────────────────────────────────────────────────────────┘
```

## Requirements

- **Hardware**: RTL-SDR USB dongle (RTL2832U-based)
- **OS**: Linux (Ubuntu/Debian recommended)
- **Software**: Rust 1.70+, Node.js 18+

## Quick Start

### Using the Installer

```bash
sudo ./installer/install.sh
```

The installer is fully idempotent and handles:
- System dependency installation
- RTL-SDR driver setup and DVB driver blacklisting
- udev rules for non-root SDR access
- mDNS/Avahi setup for casting
- Backend compilation
- Frontend build and deployment
- Systemd service creation

### Manual Setup

```bash
# Install RTL-SDR
sudo apt install rtl-sdr librtlsdr-dev

# Build backend
cd backend
cargo build --release

# Build frontend
cd frontend
npm install
npm run build

# Copy frontend to backend
cp -r frontend/build/* backend/static/

# Run
cd backend
cargo run --release
```

Open http://localhost:8080 in your browser.

## API Reference

### REST Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/stations` | List all discovered stations |
| GET | `/api/stations/{id}` | Get station details |
| GET | `/api/stations/{id}/metadata` | Get station metadata (DLS, SLS) |
| GET | `/api/status` | System status |
| POST | `/api/scan` | Start DAB+ scan |
| POST | `/api/play/{station_id}` | Start playing a station |
| POST | `/api/stop` | Stop playback |
| POST | `/api/volume` | Set volume (0.0-1.0) |
| GET | `/api/cast/devices` | List cast devices |
| POST | `/api/cast/discover` | Discover Chromecast/AirPlay devices |
| POST | `/api/cast/{device_id}` | Cast to device |
| POST | `/api/cast/stop` | Stop casting |
| GET | `/api/logs` | Get system logs |
| GET | `/api/stream/audio` | Audio stream (WAV) |
| GET | `/api/system/check` | System dependency check |

### WebSocket

Connect to `ws://host:8080/ws` for real-time updates:

- Station list updates
- Metadata changes (DLS text, SLS images)
- Scan progress
- Playback status
- Cast device discovery
- System logs

## Project Structure

```
radio/
├── backend/              # Rust backend
│   └── src/
│       ├── dsp/          # DSP pipeline (SDR, scanner, decoder)
│       ├── services/     # Station registry, metadata, monitoring
│       ├── streaming/    # Audio streaming (HTTP)
│       ├── casting/      # Chromecast + AirPlay
│       ├── api/          # REST + WebSocket endpoints
│       └── system/       # Installer checks, dependency management
├── frontend/             # React + Tailwind frontend
│   └── src/
│       ├── components/   # UI components
│       ├── hooks/        # React hooks (WebSocket, API, theme)
│       └── types/        # TypeScript types
├── installer/            # Idempotent installer
├── docs/                 # Documentation
└── tasks/                # Task tracking
```

## License

MIT
