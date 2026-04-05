# Lessons Learned

## Implementation Notes

### Backend (Rust)
- parking_lot RwLock guards cannot be held across `.await` points (not Send)
- Must scope lock guards carefully in async contexts
- actix-ws MessageStream is !Send, use `actix_web::rt::spawn` instead of `tokio::spawn`
- RTL-SDR crate (rtlsdr) provides basic bindings but may need direct librtlsdr FFI for advanced features

### Frontend (React + TypeScript)
- `useRef<T>()` requires explicit null initial value in strict mode: `useRef<T | null>(null)`
- `new Set()` spreading requires `Array.from()` when targeting ES5
- Tailwind CSS v4 uses `@import "tailwindcss"` instead of `@tailwind` directives

### DSP
- DAB+ uses Mode I in Australia (1536 subcarriers, 96ms frames)
- Reed-Solomon RS(120,110) for DAB+ super frames
- Viterbi decoder: rate 1/4, constraint length 7
- IQ samples from RTL-SDR are unsigned 8-bit, need centering at 127.5

### General
- Installer must be fully idempotent - check before every action
- mDNS discovery needs avahi-daemon running on Linux
- DVB kernel modules must be blacklisted for RTL-SDR to work
