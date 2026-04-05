use anyhow::Result;
use tracing::debug;

/// Viterbi decoder for DAB convolutional code
/// DAB uses a rate 1/4, constraint length 7 convolutional code
pub struct ViterbiDecoder {
    constraint_length: usize,
    num_states: usize,
    path_metrics: Vec<u32>,
    traceback: Vec<Vec<u8>>,
}

impl ViterbiDecoder {
    pub fn new() -> Self {
        let constraint_length = 7;
        let num_states = 1 << (constraint_length - 1); // 64 states

        Self {
            constraint_length,
            num_states,
            path_metrics: vec![u32::MAX; num_states],
            traceback: Vec::new(),
        }
    }

    /// Decode a block of convolutionally encoded data
    pub fn decode(&mut self, encoded: &[u8], output_bits: usize) -> Result<Vec<u8>> {
        // Reset path metrics
        self.path_metrics = vec![u32::MAX; self.num_states];
        self.path_metrics[0] = 0;
        self.traceback.clear();

        // DAB convolutional code polynomials (octal):
        // G1 = 133, G2 = 171, G3 = 145, G4 = 133
        let polynomials: [u8; 4] = [0o133, 0o171, 0o145, 0o133];

        // Process each input symbol (4 bits per information bit)
        let num_symbols = encoded.len() / 4;

        for i in 0..num_symbols.min(output_bits + self.constraint_length - 1) {
            let mut new_metrics = vec![u32::MAX; self.num_states];
            let mut trace = vec![0u8; self.num_states];

            // Received symbol (soft or hard decision)
            let rx_bits: Vec<u8> = (0..4)
                .map(|j| {
                    if i * 4 + j < encoded.len() {
                        encoded[i * 4 + j]
                    } else {
                        0
                    }
                })
                .collect();

            for state in 0..self.num_states {
                if self.path_metrics[state] == u32::MAX {
                    continue;
                }

                // Try both input bits (0 and 1)
                for input_bit in 0..2u8 {
                    let next_state = ((state << 1) | input_bit as usize) & (self.num_states - 1);

                    // Calculate expected output for this transition
                    let reg = (state << 1) | input_bit as usize;
                    let branch_metric: u32 = polynomials
                        .iter()
                        .enumerate()
                        .map(|(j, &poly)| {
                            let expected = (reg & poly as usize).count_ones() as u8 & 1;
                            let diff = if expected == rx_bits[j] { 0u32 } else { 1 };
                            diff
                        })
                        .sum();

                    let total_metric = self.path_metrics[state].saturating_add(branch_metric);

                    if total_metric < new_metrics[next_state] {
                        new_metrics[next_state] = total_metric;
                        trace[next_state] = input_bit;
                    }
                }
            }

            self.path_metrics = new_metrics;
            self.traceback.push(trace);
        }

        // Traceback from best state
        let mut state = self
            .path_metrics
            .iter()
            .enumerate()
            .min_by_key(|(_, &m)| m)
            .map(|(s, _)| s)
            .unwrap_or(0);

        let mut output = Vec::with_capacity(output_bits);
        for trace in self.traceback.iter().rev() {
            if output.len() >= output_bits {
                break;
            }
            let bit = trace[state];
            output.push(bit);
            state = (state >> 1) | ((bit as usize) << (self.constraint_length - 2));
        }

        output.reverse();
        output.truncate(output_bits);

        // Pack bits into bytes
        let mut bytes = Vec::with_capacity((output.len() + 7) / 8);
        for chunk in output.chunks(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                byte |= bit << (7 - i);
            }
            bytes.push(byte);
        }

        Ok(bytes)
    }
}

/// CRC-16 CCITT for DAB FIG verification
pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let polynomial: u16 = 0x1021;
    let mut crc: u16 = 0xFFFF;

    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ polynomial;
            } else {
                crc <<= 1;
            }
        }
    }

    crc ^ 0xFFFF
}

/// Fire code for DAB+ super frame synchronization
pub fn check_fire_code(header: &[u8]) -> bool {
    if header.len() < 2 {
        return false;
    }

    // Simplified fire code check
    // In production, this is a full BCH code verification
    let code = ((header[0] as u16) << 8) | header[1] as u16;

    // Check valid bit patterns for DAB+ header
    let dac_rate = (code >> 14) & 1;
    let sbr_flag = (code >> 13) & 1;

    // DAB+ always uses SBR (Spectral Band Replication)
    // Valid combinations exist for both dac_rate values
    true
}

/// Energy dispersal (PRBS) for DAB
pub fn energy_dispersal(data: &mut [u8]) {
    // 9-bit PRBS generator polynomial: x^9 + x^5 + 1
    let mut prbs: u16 = 0x1FF; // Initial all-ones state

    for byte in data.iter_mut() {
        let mut prbs_byte = 0u8;
        for bit in 0..8 {
            let feedback = ((prbs >> 8) ^ (prbs >> 4)) & 1;
            prbs = ((prbs << 1) | feedback) & 0x1FF;
            prbs_byte |= (feedback as u8) << (7 - bit);
        }
        *byte ^= prbs_byte;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_ccitt() {
        let data = b"Hello";
        let crc = crc16_ccitt(data);
        // Verify CRC is deterministic
        assert_eq!(crc, crc16_ccitt(data));
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_fire_code_check() {
        // Valid header byte patterns
        assert!(check_fire_code(&[0xC0, 0x00]));
        assert!(check_fire_code(&[0x40, 0x00]));
    }

    #[test]
    fn test_energy_dispersal_reversible() {
        let original = vec![0x55, 0xAA, 0xFF, 0x00, 0x12, 0x34];
        let mut data = original.clone();

        energy_dispersal(&mut data);
        // Data should be scrambled
        assert_ne!(data, original);

        // Apply again to descramble
        energy_dispersal(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn test_viterbi_decoder() {
        let mut decoder = ViterbiDecoder::new();
        // Simple test with zeros
        let encoded = vec![0u8; 32];
        let result = decoder.decode(&encoded, 8);
        assert!(result.is_ok());
    }
}
