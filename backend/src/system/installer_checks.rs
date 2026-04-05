use serde::Serialize;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Serialize)]
pub struct SystemCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
}

/// Run all system dependency checks
pub fn run_all_checks() -> Vec<SystemCheck> {
    let mut checks = Vec::new();

    checks.push(check_rtlsdr_driver());
    checks.push(check_rtlsdr_tools());
    checks.push(check_usb_device());
    checks.push(check_blacklist());
    checks.push(check_udev_rules());
    checks.push(check_audio_system());
    checks.push(check_avahi());

    let pass_count = checks.iter().filter(|c| matches!(c.status, CheckStatus::Pass)).count();
    let total = checks.len();
    info!("System checks: {}/{} passed", pass_count, total);

    checks
}

fn check_rtlsdr_driver() -> SystemCheck {
    let result = Command::new("modinfo").arg("rtl2832").output();

    match result {
        Ok(output) if output.status.success() => SystemCheck {
            name: "RTL-SDR Kernel Driver".to_string(),
            status: CheckStatus::Pass,
            message: "rtl2832 driver available".to_string(),
            required: true,
        },
        _ => SystemCheck {
            name: "RTL-SDR Kernel Driver".to_string(),
            status: CheckStatus::Warning,
            message: "rtl2832 kernel driver not found (may use librtlsdr directly)".to_string(),
            required: true,
        },
    }
}

fn check_rtlsdr_tools() -> SystemCheck {
    let result = Command::new("which").arg("rtl_test").output();

    match result {
        Ok(output) if output.status.success() => SystemCheck {
            name: "RTL-SDR Tools".to_string(),
            status: CheckStatus::Pass,
            message: "rtl-sdr tools installed".to_string(),
            required: true,
        },
        _ => SystemCheck {
            name: "RTL-SDR Tools".to_string(),
            status: CheckStatus::Fail,
            message: "rtl-sdr tools not found. Install with: sudo apt install rtl-sdr".to_string(),
            required: true,
        },
    }
}

fn check_usb_device() -> SystemCheck {
    let result = Command::new("lsusb").output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("0bda:2832") || stdout.contains("0bda:2838") || stdout.contains("RTL2838") || stdout.contains("RTL2832") {
                SystemCheck {
                    name: "RTL-SDR USB Device".to_string(),
                    status: CheckStatus::Pass,
                    message: "RTL-SDR USB device detected".to_string(),
                    required: true,
                }
            } else {
                SystemCheck {
                    name: "RTL-SDR USB Device".to_string(),
                    status: CheckStatus::Warning,
                    message: "No RTL-SDR USB device detected. Please connect your SDR dongle.".to_string(),
                    required: true,
                }
            }
        }
        Err(_) => SystemCheck {
            name: "RTL-SDR USB Device".to_string(),
            status: CheckStatus::Warning,
            message: "Could not run lsusb to detect devices".to_string(),
            required: true,
        },
    }
}

fn check_blacklist() -> SystemCheck {
    // Check if dvb_usb_rtl28xxu is blacklisted (it conflicts with librtlsdr)
    let result = std::fs::read_to_string("/etc/modprobe.d/blacklist-rtlsdr.conf");

    match result {
        Ok(content) if content.contains("dvb_usb_rtl28xxu") => SystemCheck {
            name: "DVB Driver Blacklist".to_string(),
            status: CheckStatus::Pass,
            message: "dvb_usb_rtl28xxu is properly blacklisted".to_string(),
            required: false,
        },
        _ => {
            // Check if the module is loaded
            let loaded = Command::new("lsmod")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("dvb_usb_rtl28xxu"))
                .unwrap_or(false);

            if loaded {
                SystemCheck {
                    name: "DVB Driver Blacklist".to_string(),
                    status: CheckStatus::Fail,
                    message: "dvb_usb_rtl28xxu is loaded and conflicts with RTL-SDR. Run installer to fix.".to_string(),
                    required: false,
                }
            } else {
                SystemCheck {
                    name: "DVB Driver Blacklist".to_string(),
                    status: CheckStatus::Pass,
                    message: "No conflicting DVB driver loaded".to_string(),
                    required: false,
                }
            }
        }
    }
}

fn check_udev_rules() -> SystemCheck {
    let udev_path = "/etc/udev/rules.d/20-rtlsdr.rules";
    if std::path::Path::new(udev_path).exists() {
        SystemCheck {
            name: "Udev Rules".to_string(),
            status: CheckStatus::Pass,
            message: "RTL-SDR udev rules installed".to_string(),
            required: false,
        }
    } else {
        SystemCheck {
            name: "Udev Rules".to_string(),
            status: CheckStatus::Warning,
            message: "RTL-SDR udev rules not found. Non-root access may not work.".to_string(),
            required: false,
        }
    }
}

fn check_audio_system() -> SystemCheck {
    // Check for PulseAudio or PipeWire
    let pulse = Command::new("which").arg("pulseaudio").output();
    let pipewire = Command::new("which").arg("pipewire").output();

    let has_pulse = pulse.map(|o| o.status.success()).unwrap_or(false);
    let has_pipewire = pipewire.map(|o| o.status.success()).unwrap_or(false);

    if has_pipewire {
        SystemCheck {
            name: "Audio System".to_string(),
            status: CheckStatus::Pass,
            message: "PipeWire audio system available".to_string(),
            required: false,
        }
    } else if has_pulse {
        SystemCheck {
            name: "Audio System".to_string(),
            status: CheckStatus::Pass,
            message: "PulseAudio system available".to_string(),
            required: false,
        }
    } else {
        SystemCheck {
            name: "Audio System".to_string(),
            status: CheckStatus::Warning,
            message: "No audio system detected. Local audio playback may not work.".to_string(),
            required: false,
        }
    }
}

fn check_avahi() -> SystemCheck {
    let result = Command::new("which").arg("avahi-browse").output();

    match result {
        Ok(output) if output.status.success() => SystemCheck {
            name: "mDNS (Avahi)".to_string(),
            status: CheckStatus::Pass,
            message: "Avahi mDNS service available (needed for Chromecast/AirPlay)".to_string(),
            required: false,
        },
        _ => SystemCheck {
            name: "mDNS (Avahi)".to_string(),
            status: CheckStatus::Warning,
            message: "Avahi not found. Chromecast/AirPlay discovery may not work.".to_string(),
            required: false,
        },
    }
}
