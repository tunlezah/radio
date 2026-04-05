use anyhow::Result;
use tracing::{debug, warn};

/// Parse Dynamic Label Segment (DLS) from PAD data
/// DLS provides scrolling text information (song title, artist, etc.)
pub struct DlsParser {
    buffer: Vec<u8>,
    label: String,
    charset: u8,
    toggle_bit: bool,
    segment_count: u8,
}

impl DlsParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            label: String::new(),
            charset: 0,
            toggle_bit: false,
            segment_count: 0,
        }
    }

    /// Process a DLS data segment
    /// Returns Some(text) when a complete label has been assembled
    pub fn process_segment(&mut self, data: &[u8]) -> Option<String> {
        if data.len() < 2 {
            return None;
        }

        // DLS segment header
        let command = data[0];
        let toggle = (command >> 7) & 1 == 1;
        let first = (command >> 6) & 1 == 1;
        let last = (command >> 5) & 1 == 1;

        // Detect new label (toggle bit change)
        if toggle != self.toggle_bit {
            self.toggle_bit = toggle;
            self.buffer.clear();
            self.segment_count = 0;
        }

        // Append segment data
        self.buffer.extend_from_slice(&data[1..]);
        self.segment_count += 1;

        // If this is the last segment, assemble the label
        if last {
            let label = match self.charset {
                0 | 15 => String::from_utf8_lossy(&self.buffer).trim().to_string(),
                _ => {
                    // EBU character set - approximate with UTF-8
                    String::from_utf8_lossy(&self.buffer).trim().to_string()
                }
            };

            if !label.is_empty() && label != self.label {
                self.label = label.clone();
                debug!("DLS update: {}", label);
                return Some(label);
            }
        }

        None
    }

    pub fn current_label(&self) -> &str {
        &self.label
    }
}

/// Parse Slideshow (SLS/MOT) data
/// SLS provides images associated with the current programme
pub struct SlsParser {
    buffer: Vec<u8>,
    expected_size: usize,
    content_type: String,
    transport_id: u16,
}

impl SlsParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            expected_size: 0,
            content_type: String::new(),
            transport_id: 0,
        }
    }

    /// Process a MOT (Multimedia Object Transfer) segment
    /// Returns Some((content_type, data)) when a complete image is assembled
    pub fn process_segment(&mut self, data: &[u8]) -> Option<(String, Vec<u8>)> {
        if data.len() < 4 {
            return None;
        }

        // MOT header parsing
        let segment_number = ((data[0] as u16) << 8) | data[1] as u16;
        let transport_id = ((data[2] as u16) << 8) | data[3] as u16;

        // New object
        if transport_id != self.transport_id {
            self.transport_id = transport_id;
            self.buffer.clear();
            self.expected_size = 0;
        }

        if segment_number == 0 && data.len() > 12 {
            // Header segment - extract content type and size
            let body_size = ((data[4] as u32) << 20)
                | ((data[5] as u32) << 12)
                | ((data[6] as u32) << 4)
                | ((data[7] as u32) >> 4);

            let content_type = (data[8] >> 1) & 0x3F;
            let content_sub_type = ((data[8] as u16 & 1) << 8) | data[9] as u16;

            self.expected_size = body_size as usize;
            self.content_type = match content_type {
                2 => match content_sub_type {
                    0 => "image/gif".to_string(),
                    1 => "image/jpeg".to_string(),
                    2 => "image/bmp".to_string(),
                    3 => "image/png".to_string(),
                    _ => format!("image/unknown-{}", content_sub_type),
                },
                _ => format!("application/octet-stream"),
            };

            debug!(
                "SLS object: type={}, size={} bytes",
                self.content_type, self.expected_size
            );
        }

        // Append body data
        let body_offset = if segment_number == 0 { 12 } else { 4 };
        if body_offset < data.len() {
            self.buffer.extend_from_slice(&data[body_offset..]);
        }

        // Check if complete
        if self.expected_size > 0 && self.buffer.len() >= self.expected_size {
            let image_data = self.buffer[..self.expected_size].to_vec();
            let content_type = self.content_type.clone();
            self.buffer.clear();
            self.expected_size = 0;

            debug!("SLS image complete: {} ({} bytes)", content_type, image_data.len());
            return Some((content_type, image_data));
        }

        None
    }
}

/// Simulated DLS text for Australian stations (for development)
pub fn get_simulated_dls(service_id: u32) -> String {
    match service_id {
        0x1001 | 0x4001 | 0x5001 | 0x6001 | 0x7001 => {
            "triple j - Playing the hottest music".to_string()
        }
        0x1002 | 0x4002 => "Double J - Music for grown-ups".to_string(),
        0x1003 | 0x4005 => "ABC NEWS - Breaking news and analysis".to_string(),
        0x1004 | 0x4004 | 0x5003 | 0x6003 | 0x7003 => {
            "ABC Classic - Beautiful music, beautifully played".to_string()
        }
        0x1005 => "ABC Kids Listen - Fun and safe for kids".to_string(),
        0x1006 => "triple j Unearthed - Discovering new Australian music".to_string(),
        0x2001 | 0x400B => "KIIS - Hit music station".to_string(),
        0x2003 | 0x4008 | 0x5004 | 0x6006 | 0x7004 => {
            "Nova - Entertainment and music".to_string()
        }
        0x2005 | 0x400A => "Talk radio - News, sport, and opinion".to_string(),
        _ => "DAB+ Digital Radio".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dls_parser_complete_label() {
        let mut parser = DlsParser::new();

        // Simulate a single-segment DLS with 'last' flag set
        let data = vec![
            0b1010_0000, // toggle=1, first=0, last=1
            b'H', b'e', b'l', b'l', b'o',
        ];

        let result = parser.process_segment(&data);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Hello");
    }

    #[test]
    fn test_simulated_dls() {
        let dls = get_simulated_dls(0x1001);
        assert!(dls.contains("triple j"));
    }
}
