#!/usr/bin/env bash
#
# DAB+ Radio Web Application - Installer
# Idempotent, bulletproof installer for the DAB+ radio system
#
# Safe to run repeatedly - detects existing installs, repairs broken ones,
# and upgrades as needed.
#
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOG_FILE="/tmp/dab-radio-install.log"

# ============================================================================
# Helper functions
# ============================================================================

log_info()  { echo -e "${BLUE}[INFO]${NC}  $1" | tee -a "$LOG_FILE"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $1" | tee -a "$LOG_FILE"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $1" | tee -a "$LOG_FILE"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"; }

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This installer must be run as root (use sudo)"
        exit 1
    fi
}

# Check if a command exists
cmd_exists() { command -v "$1" &>/dev/null; }

# Check if a package is installed (Debian/Ubuntu)
pkg_installed() { dpkg -s "$1" &>/dev/null 2>&1; }

# Install a package if not already installed
ensure_package() {
    local pkg="$1"
    if pkg_installed "$pkg"; then
        log_ok "$pkg already installed"
    else
        log_info "Installing $pkg..."
        apt-get install -y "$pkg" >> "$LOG_FILE" 2>&1
        if pkg_installed "$pkg"; then
            log_ok "$pkg installed successfully"
        else
            log_error "Failed to install $pkg"
            return 1
        fi
    fi
}

# ============================================================================
# Installation steps
# ============================================================================

install_system_deps() {
    log_info "=== System Dependencies ==="

    # Update package list (only if stale)
    local apt_cache="/var/cache/apt/pkgcache.bin"
    if [[ ! -f "$apt_cache" ]] || [[ $(find "$apt_cache" -mmin +60 2>/dev/null) ]]; then
        log_info "Updating package lists..."
        apt-get update >> "$LOG_FILE" 2>&1
    else
        log_ok "Package lists are up to date"
    fi

    # Core dependencies
    ensure_package "build-essential"
    ensure_package "pkg-config"
    ensure_package "libusb-1.0-0-dev"
    ensure_package "curl"
    ensure_package "git"
}

install_rtlsdr() {
    log_info "=== RTL-SDR Setup ==="

    ensure_package "rtl-sdr"
    ensure_package "librtlsdr-dev"

    # Blacklist conflicting DVB drivers
    local blacklist_file="/etc/modprobe.d/blacklist-rtlsdr.conf"
    local blacklist_content="# Blacklist DVB drivers that conflict with RTL-SDR (managed by DAB+ installer)
blacklist dvb_usb_rtl28xxu
blacklist rtl2832
blacklist rtl2830
blacklist dvb_usb_rtl2832u"

    if [[ -f "$blacklist_file" ]]; then
        if grep -q "dvb_usb_rtl28xxu" "$blacklist_file"; then
            log_ok "DVB driver blacklist already configured"
        else
            echo "$blacklist_content" > "$blacklist_file"
            log_ok "DVB driver blacklist updated"
        fi
    else
        echo "$blacklist_content" > "$blacklist_file"
        log_ok "DVB driver blacklist created"
    fi

    # Unload conflicting modules if currently loaded
    for mod in dvb_usb_rtl28xxu rtl2832 rtl2830; do
        if lsmod | grep -q "$mod" 2>/dev/null; then
            log_warn "Unloading conflicting module: $mod"
            modprobe -r "$mod" 2>/dev/null || true
        fi
    done

    # Create udev rules for non-root access
    local udev_file="/etc/udev/rules.d/20-rtlsdr.rules"
    local udev_content='# RTL-SDR USB device rules (managed by DAB+ installer)
SUBSYSTEM=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="2832", MODE:="0666", GROUP="plugdev"
SUBSYSTEM=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="2838", MODE:="0666", GROUP="plugdev"
SUBSYSTEM=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="2834", MODE:="0666", GROUP="plugdev"'

    if [[ -f "$udev_file" ]] && grep -q "0bda" "$udev_file"; then
        log_ok "RTL-SDR udev rules already configured"
    else
        echo "$udev_content" > "$udev_file"
        udevadm control --reload-rules 2>/dev/null || true
        udevadm trigger 2>/dev/null || true
        log_ok "RTL-SDR udev rules installed"
    fi
}

install_audio_deps() {
    log_info "=== Audio Dependencies ==="

    # Install ALSA/PulseAudio support
    ensure_package "libasound2-dev" || ensure_package "libasound-dev" || true
    ensure_package "libpulse-dev" || true
}

install_mdns() {
    log_info "=== mDNS (Chromecast/AirPlay Discovery) ==="

    ensure_package "avahi-daemon"
    ensure_package "libavahi-client-dev" || true
    ensure_package "libnss-mdns" || true

    # Ensure avahi-daemon is running
    if systemctl is-active --quiet avahi-daemon 2>/dev/null; then
        log_ok "avahi-daemon is running"
    else
        log_info "Starting avahi-daemon..."
        systemctl enable avahi-daemon 2>/dev/null || true
        systemctl start avahi-daemon 2>/dev/null || true
        if systemctl is-active --quiet avahi-daemon 2>/dev/null; then
            log_ok "avahi-daemon started"
        else
            log_warn "Could not start avahi-daemon (Chromecast/AirPlay may not work)"
        fi
    fi
}

install_rust() {
    log_info "=== Rust Toolchain ==="

    if cmd_exists rustc; then
        local rust_version
        rust_version=$(rustc --version | awk '{print $2}')
        log_ok "Rust $rust_version already installed"
    else
        log_info "Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >> "$LOG_FILE" 2>&1
        source "$HOME/.cargo/env"
        log_ok "Rust $(rustc --version | awk '{print $2}') installed"
    fi
}

install_node() {
    log_info "=== Node.js ==="

    if cmd_exists node; then
        local node_version
        node_version=$(node --version)
        log_ok "Node.js $node_version already installed"
    else
        log_info "Installing Node.js..."
        if cmd_exists apt-get; then
            curl -fsSL https://deb.nodesource.com/setup_22.x | bash - >> "$LOG_FILE" 2>&1
            apt-get install -y nodejs >> "$LOG_FILE" 2>&1
        fi
        log_ok "Node.js $(node --version) installed"
    fi
}

build_backend() {
    log_info "=== Building Backend ==="

    cd "$PROJECT_DIR/backend"

    if [[ -f "target/release/dab_radio_backend" ]]; then
        local src_mtime
        src_mtime=$(find src -name '*.rs' -newer target/release/dab_radio_backend 2>/dev/null | head -1)
        if [[ -z "$src_mtime" ]]; then
            log_ok "Backend binary is up to date"
            return 0
        fi
    fi

    log_info "Compiling Rust backend (release mode)..."
    cargo build --release >> "$LOG_FILE" 2>&1
    log_ok "Backend compiled successfully"
}

build_frontend() {
    log_info "=== Building Frontend ==="

    cd "$PROJECT_DIR/frontend"

    if [[ -d "build" ]]; then
        local src_mtime
        src_mtime=$(find src -newer build/index.html 2>/dev/null | head -1)
        if [[ -z "$src_mtime" ]]; then
            log_ok "Frontend build is up to date"
            return 0
        fi
    fi

    log_info "Installing npm dependencies..."
    npm install >> "$LOG_FILE" 2>&1

    log_info "Building frontend..."
    npm run build >> "$LOG_FILE" 2>&1
    log_ok "Frontend built successfully"

    # Copy build to backend static directory
    mkdir -p "$PROJECT_DIR/backend/static"
    cp -r build/* "$PROJECT_DIR/backend/static/"
    log_ok "Frontend deployed to backend/static/"
}

create_systemd_service() {
    log_info "=== System Service ==="

    local service_file="/etc/systemd/system/dab-radio.service"
    local service_content="[Unit]
Description=DAB+ Radio Web Application
After=network.target avahi-daemon.service
Wants=avahi-daemon.service

[Service]
Type=simple
ExecStart=$PROJECT_DIR/backend/target/release/dab_radio_backend
WorkingDirectory=$PROJECT_DIR/backend
Environment=BIND_ADDR=0.0.0.0:8080
Environment=RUST_LOG=info
Restart=on-failure
RestartSec=5
User=root

[Install]
WantedBy=multi-user.target"

    if [[ -f "$service_file" ]]; then
        local existing_hash new_hash
        existing_hash=$(md5sum "$service_file" | awk '{print $1}')
        new_hash=$(echo "$service_content" | md5sum | awk '{print $1}')

        if [[ "$existing_hash" == "$new_hash" ]]; then
            log_ok "Systemd service already configured"
            return 0
        fi
    fi

    echo "$service_content" > "$service_file"
    systemctl daemon-reload 2>/dev/null || true
    log_ok "Systemd service installed"
}

run_verification() {
    log_info "=== Verification ==="

    local pass=0
    local fail=0

    # Check RTL-SDR tools
    if cmd_exists rtl_test; then
        log_ok "rtl-sdr tools: installed"
        ((pass++))
    else
        log_error "rtl-sdr tools: missing"
        ((fail++))
    fi

    # Check blacklist
    if [[ -f "/etc/modprobe.d/blacklist-rtlsdr.conf" ]]; then
        log_ok "DVB blacklist: configured"
        ((pass++))
    else
        log_warn "DVB blacklist: missing"
        ((fail++))
    fi

    # Check udev
    if [[ -f "/etc/udev/rules.d/20-rtlsdr.rules" ]]; then
        log_ok "Udev rules: configured"
        ((pass++))
    else
        log_warn "Udev rules: missing"
        ((fail++))
    fi

    # Check backend binary
    if [[ -f "$PROJECT_DIR/backend/target/release/dab_radio_backend" ]]; then
        log_ok "Backend binary: built"
        ((pass++))
    else
        log_warn "Backend binary: not built"
        ((fail++))
    fi

    # Check frontend build
    if [[ -d "$PROJECT_DIR/frontend/build" ]]; then
        log_ok "Frontend build: ready"
        ((pass++))
    else
        log_warn "Frontend build: not built"
        ((fail++))
    fi

    # Check avahi
    if systemctl is-active --quiet avahi-daemon 2>/dev/null; then
        log_ok "mDNS (avahi): running"
        ((pass++))
    else
        log_warn "mDNS (avahi): not running"
        ((fail++))
    fi

    echo ""
    log_info "Verification: $pass passed, $fail issues"

    if [[ $fail -gt 0 ]]; then
        log_warn "Some checks did not pass. The application may work with limited functionality."
    else
        log_ok "All checks passed!"
    fi
}

print_summary() {
    echo ""
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN} DAB+ Radio Installation Complete!${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo ""
    echo -e "  Start the application:"
    echo -e "    ${BLUE}cd $PROJECT_DIR/backend${NC}"
    echo -e "    ${BLUE}cargo run --release${NC}"
    echo ""
    echo -e "  Or use the systemd service:"
    echo -e "    ${BLUE}sudo systemctl start dab-radio${NC}"
    echo -e "    ${BLUE}sudo systemctl enable dab-radio${NC}"
    echo ""
    echo -e "  Open in browser: ${BLUE}http://localhost:8080${NC}"
    echo ""
    echo -e "  Logs: ${BLUE}$LOG_FILE${NC}"
    echo ""
}

# ============================================================================
# Main
# ============================================================================

main() {
    echo "" > "$LOG_FILE"
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE} DAB+ Radio Web Application Installer${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo ""

    check_root

    install_system_deps
    install_rtlsdr
    install_audio_deps
    install_mdns
    install_rust
    install_node
    build_backend
    build_frontend
    create_systemd_service
    run_verification
    print_summary
}

main "$@"
