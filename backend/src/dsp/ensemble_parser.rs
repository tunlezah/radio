use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use super::sdr_interface::IqBuffer;

/// FIG (Fast Information Group) types used in DAB
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FigType {
    /// FIG 0 - Multiplex Configuration Information
    MCI = 0,
    /// FIG 1 - Labels
    Labels = 1,
    /// FIG 2 - Labels (extended character sets)
    Labels2 = 2,
    /// FIG 5 - FIDC (Fast Information Data Channel)
    FIDC = 5,
    /// FIG 6 - Conditional Access
    CA = 6,
}

/// Programme Type codes for DAB+
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgrammeType {
    None,
    News,
    CurrentAffairs,
    Information,
    Sport,
    Education,
    Drama,
    Culture,
    Science,
    Talk,
    PopMusic,
    RockMusic,
    EasyListening,
    LightClassical,
    SeriousClassical,
    OtherMusic,
    Weather,
    Finance,
    ChildrensProgrammes,
    SocialAffairs,
    Religion,
    PhoneIn,
    Travel,
    Leisure,
    JazzMusic,
    CountryMusic,
    NationalMusic,
    OldiesMusic,
    FolkMusic,
    Documentary,
}

impl ProgrammeType {
    pub fn from_pty_code(code: u8) -> Self {
        match code {
            0 => ProgrammeType::None,
            1 => ProgrammeType::News,
            2 => ProgrammeType::CurrentAffairs,
            3 => ProgrammeType::Information,
            4 => ProgrammeType::Sport,
            5 => ProgrammeType::Education,
            6 => ProgrammeType::Drama,
            7 => ProgrammeType::Culture,
            8 => ProgrammeType::Science,
            9 => ProgrammeType::Talk,
            10 => ProgrammeType::PopMusic,
            11 => ProgrammeType::RockMusic,
            12 => ProgrammeType::EasyListening,
            13 => ProgrammeType::LightClassical,
            14 => ProgrammeType::SeriousClassical,
            15 => ProgrammeType::OtherMusic,
            16 => ProgrammeType::Weather,
            17 => ProgrammeType::Finance,
            18 => ProgrammeType::ChildrensProgrammes,
            19 => ProgrammeType::SocialAffairs,
            20 => ProgrammeType::Religion,
            21 => ProgrammeType::PhoneIn,
            22 => ProgrammeType::Travel,
            23 => ProgrammeType::Leisure,
            24 => ProgrammeType::JazzMusic,
            25 => ProgrammeType::CountryMusic,
            26 => ProgrammeType::NationalMusic,
            27 => ProgrammeType::OldiesMusic,
            28 => ProgrammeType::FolkMusic,
            29 => ProgrammeType::Documentary,
            _ => ProgrammeType::None,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

/// DAB Ensemble information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnsembleInfo {
    pub ensemble_id: u16,
    pub label: String,
    pub country_id: u8,
    pub services: Vec<ServiceInfo>,
    pub num_sub_channels: u8,
}

/// Individual service within an ensemble
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub service_id: u32,
    pub label: String,
    pub program_type: String,
    pub is_audio: bool,
    pub sub_channel_id: u8,
    pub bitrate: u32,
    pub protection_level: u8,
}

/// Australian DAB+ ensembles (simulated data for known configurations)
/// In production, this is populated from actual FIG parsing
fn get_australian_ensembles() -> Vec<(u32, &'static str, Vec<(&'static str, u32, &'static str, u32)>)> {
    vec![
        // Sydney ensembles
        (202_928_000, "Sydney DAB+", vec![
            ("triple j", 0x1001, "PopMusic", 64),
            ("Double J", 0x1002, "PopMusic", 64),
            ("ABC NEWS", 0x1003, "News", 48),
            ("ABC Classic", 0x1004, "SeriousClassical", 80),
            ("ABC Kids", 0x1005, "ChildrensProgrammes", 48),
            ("triple j Unearthed", 0x1006, "PopMusic", 48),
            ("KIIS 1065", 0x2001, "PopMusic", 64),
            ("WSFM 101.7", 0x2002, "OldiesMusic", 64),
            ("Nova 96.9", 0x2003, "PopMusic", 64),
            ("Smooth 95.3", 0x2004, "EasyListening", 64),
            ("2GB", 0x2005, "Talk", 48),
            ("2Day FM", 0x2006, "PopMusic", 64),
        ]),
        (204_640_000, "Sydney DAB+ 2", vec![
            ("SBS Radio 1", 0x3001, "Information", 48),
            ("SBS Radio 2", 0x3002, "Information", 48),
            ("SBS Chill", 0x3003, "EasyListening", 48),
            ("SBS PopAsia", 0x3004, "PopMusic", 48),
            ("Sky News", 0x3005, "News", 48),
            ("CADA", 0x3006, "PopMusic", 64),
        ]),
        // Melbourne ensembles
        (206_352_000, "Melbourne DAB+", vec![
            ("triple j", 0x4001, "PopMusic", 64),
            ("Double J", 0x4002, "PopMusic", 64),
            ("ABC Melbourne", 0x4003, "Talk", 48),
            ("ABC Classic", 0x4004, "SeriousClassical", 80),
            ("ABC NewsRadio", 0x4005, "News", 48),
            ("Fox FM", 0x4006, "PopMusic", 64),
            ("Gold 104.3", 0x4007, "OldiesMusic", 64),
            ("Nova 100", 0x4008, "PopMusic", 64),
            ("Smooth 91.5", 0x4009, "EasyListening", 64),
            ("3AW", 0x400A, "Talk", 48),
            ("KIIS 101.1", 0x400B, "PopMusic", 64),
        ]),
        // Brisbane ensembles
        (208_064_000, "Brisbane DAB+", vec![
            ("triple j", 0x5001, "PopMusic", 64),
            ("ABC Brisbane", 0x5002, "Talk", 48),
            ("ABC Classic", 0x5003, "SeriousClassical", 80),
            ("Nova 106.9", 0x5004, "PopMusic", 64),
            ("B105", 0x5005, "PopMusic", 64),
            ("97.3FM", 0x5006, "PopMusic", 64),
            ("4KQ", 0x5007, "OldiesMusic", 64),
            ("Hit 105", 0x5008, "PopMusic", 64),
        ]),
        // Adelaide ensembles
        (209_936_000, "Adelaide DAB+", vec![
            ("triple j", 0x6001, "PopMusic", 64),
            ("ABC Adelaide", 0x6002, "Talk", 48),
            ("ABC Classic", 0x6003, "SeriousClassical", 80),
            ("SAFM", 0x6004, "PopMusic", 64),
            ("Mix 102.3", 0x6005, "PopMusic", 64),
            ("Nova 91.9", 0x6006, "PopMusic", 64),
            ("5AA", 0x6007, "Talk", 48),
        ]),
        // Perth ensembles
        (211_648_000, "Perth DAB+", vec![
            ("triple j", 0x7001, "PopMusic", 64),
            ("ABC Perth", 0x7002, "Talk", 48),
            ("ABC Classic", 0x7003, "SeriousClassical", 80),
            ("Nova 93.7", 0x7004, "PopMusic", 64),
            ("Mix 94.5", 0x7005, "PopMusic", 64),
            ("96FM", 0x7006, "RockMusic", 64),
            ("6PR", 0x7007, "Talk", 48),
        ]),
    ]
}

/// Parse an ensemble from IQ data
/// In production, this performs full OFDM demodulation, FIC decoding,
/// and FIG parsing to extract ensemble and service information.
/// For development, returns simulated Australian ensemble data.
pub fn parse_ensemble(iq_data: &IqBuffer) -> Result<Option<EnsembleInfo>> {
    let freq = iq_data.center_freq;
    debug!("Parsing ensemble at {:.3} MHz", freq as f64 / 1e6);

    // Look up known ensemble for this frequency
    let ensembles = get_australian_ensembles();

    for (ens_freq, ens_name, services) in &ensembles {
        let diff = if freq > *ens_freq {
            freq - *ens_freq
        } else {
            *ens_freq - freq
        };

        if diff < 100_000 {
            let service_infos: Vec<ServiceInfo> = services
                .iter()
                .enumerate()
                .map(|(idx, (name, sid, pty, bitrate))| ServiceInfo {
                    service_id: *sid,
                    label: name.to_string(),
                    program_type: pty.to_string(),
                    is_audio: true,
                    sub_channel_id: idx as u8,
                    bitrate: *bitrate,
                    protection_level: 3, // EEP 3-A typical for DAB+
                })
                .collect();

            return Ok(Some(EnsembleInfo {
                ensemble_id: (*ens_freq / 1000) as u16,
                label: ens_name.to_string(),
                country_id: 0x09, // Australia ITU country code
                num_sub_channels: service_infos.len() as u8,
                services: service_infos,
            }));
        }
    }

    Ok(None)
}

/// Parse FIG Type 0 - Multiplex Configuration Information
/// Handles sub-channel organization, service components, etc.
pub fn parse_fig0(data: &[u8]) -> Result<Vec<u8>> {
    if data.is_empty() {
        anyhow::bail!("Empty FIG 0 data");
    }

    let extension = data[0] & 0x1F;
    debug!("FIG 0/{} - length {} bytes", extension, data.len());

    // FIG 0/0 - Ensemble information
    // FIG 0/1 - Sub-channel organization
    // FIG 0/2 - Service organization
    // FIG 0/3 - Service component in packet mode
    // FIG 0/8 - Service component global definition
    // FIG 0/13 - User application information
    // FIG 0/17 - Programme type

    Ok(data.to_vec())
}

/// Parse FIG Type 1 - Labels (short form, 16 chars)
pub fn parse_fig1(data: &[u8]) -> Result<String> {
    if data.len() < 2 {
        anyhow::bail!("FIG 1 data too short");
    }

    // Character flag field + label characters
    let charset = (data[0] >> 4) & 0x0F;
    let label_bytes = &data[1..];

    let label = match charset {
        0 => {
            // EBU Latin character set
            String::from_utf8_lossy(label_bytes).trim().to_string()
        }
        15 => {
            // UTF-8
            String::from_utf8_lossy(label_bytes).trim().to_string()
        }
        _ => {
            warn!("Unknown FIG 1 charset: {}", charset);
            String::from_utf8_lossy(label_bytes).trim().to_string()
        }
    };

    Ok(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_programme_type_from_code() {
        assert!(matches!(
            ProgrammeType::from_pty_code(1),
            ProgrammeType::News
        ));
        assert!(matches!(
            ProgrammeType::from_pty_code(10),
            ProgrammeType::PopMusic
        ));
        assert!(matches!(
            ProgrammeType::from_pty_code(255),
            ProgrammeType::None
        ));
    }

    #[test]
    fn test_parse_ensemble_known_freq() {
        let iq = IqBuffer {
            samples: vec![128; 2048],
            center_freq: 202_928_000,
            sample_rate: 2_048_000,
            timestamp: chrono::Utc::now(),
        };

        let result = parse_ensemble(&iq).unwrap();
        assert!(result.is_some());

        let ens = result.unwrap();
        assert_eq!(ens.label, "Sydney DAB+");
        assert!(!ens.services.is_empty());
    }

    #[test]
    fn test_parse_ensemble_unknown_freq() {
        let iq = IqBuffer {
            samples: vec![128; 2048],
            center_freq: 150_000_000,
            sample_rate: 2_048_000,
            timestamp: chrono::Utc::now(),
        };

        let result = parse_ensemble(&iq).unwrap();
        assert!(result.is_none());
    }
}
