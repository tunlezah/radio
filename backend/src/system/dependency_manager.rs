use anyhow::Result;
use std::process::Command;
use tracing::{info, warn};

/// Dependency installation and management
pub struct DependencyManager;

impl DependencyManager {
    /// Get the list of required system packages
    pub fn required_packages() -> Vec<&'static str> {
        vec![
            "rtl-sdr",
            "librtlsdr-dev",
            "libusb-1.0-0-dev",
            "avahi-daemon",
            "libavahi-client-dev",
        ]
    }

    /// Check which packages are missing
    pub fn check_missing_packages() -> Vec<String> {
        let mut missing = Vec::new();

        for pkg in Self::required_packages() {
            let result = Command::new("dpkg")
                .args(["-s", pkg])
                .output();

            match result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if !stdout.contains("Status: install ok installed") {
                        missing.push(pkg.to_string());
                    }
                }
                Err(_) => {
                    missing.push(pkg.to_string());
                }
            }
        }

        missing
    }

    /// Create blacklist file for conflicting DVB drivers
    pub fn create_blacklist_file() -> Result<()> {
        let content = "# Blacklist DVB drivers that conflict with RTL-SDR\n\
                        blacklist dvb_usb_rtl28xxu\n\
                        blacklist rtl2832\n\
                        blacklist rtl2830\n";

        std::fs::write("/etc/modprobe.d/blacklist-rtlsdr.conf", content)?;
        info!("Created RTL-SDR blacklist file");
        Ok(())
    }

    /// Create udev rules for non-root RTL-SDR access
    pub fn create_udev_rules() -> Result<()> {
        let content = r#"# RTL-SDR USB device rules
SUBSYSTEM=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="2832", MODE:="0666"
SUBSYSTEM=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="2838", MODE:="0666"
SUBSYSTEM=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="2834", MODE:="0666"
"#;

        std::fs::write("/etc/udev/rules.d/20-rtlsdr.rules", content)?;

        // Reload udev rules
        let _ = Command::new("udevadm").args(["control", "--reload-rules"]).output();
        let _ = Command::new("udevadm").arg("trigger").output();

        info!("Created RTL-SDR udev rules");
        Ok(())
    }
}
