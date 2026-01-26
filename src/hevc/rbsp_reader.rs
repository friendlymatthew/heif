use anyhow::{Result, ensure};

#[derive(Debug)]
pub struct RbspReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> RbspReader<'a> {
    pub fn remove_emulation_prevention(data: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            if let Some(zero_offset) = data[i..].iter().position(|&b| b == 0x00) {
                let zero_pos = i + zero_offset;

                output.extend_from_slice(&data[i..zero_pos]);
                output.push(0x00);

                if zero_pos + 2 < data.len()
                    && data[zero_pos + 1] == 0x00
                    && data[zero_pos + 2] == 0x03
                    && (zero_pos + 3 >= data.len() || data[zero_pos + 3] <= 0x03)
                {
                    output.push(0x00);
                    i = zero_pos + 3;
                } else {
                    i = zero_pos + 1;
                }
            } else {
                output.extend_from_slice(&data[i..]);
                break;
            }
        }

        output
    }

    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    pub const fn is_byte_aligned(&self) -> bool {
        self.bit_pos == 0
    }

    pub fn read_flag(&mut self) -> Result<bool> {
        Ok(self.read_bit()? == 1)
    }

    pub fn read_u8(&mut self, n: usize) -> Result<u8> {
        ensure!(n <= 8, "cannot read {} bits into u8", n);
        Ok(self.read_bits(n)? as u8)
    }

    pub fn read_u32(&mut self, n: usize) -> Result<u32> {
        ensure!(n <= 32, "cannot read {} bits into u32", n);
        Ok(self.read_bits(n)? as u32)
    }

    pub fn read_ue(&mut self) -> Result<u32> {
        let mut leading_zero_bits = 0;
        while !self.read_flag()? {
            leading_zero_bits += 1;
        }

        if leading_zero_bits == 0 {
            return Ok(0);
        }

        let suffix = self.read_bits(leading_zero_bits)? as u32;
        Ok((1 << leading_zero_bits) - 1 + suffix)
    }

    pub fn read_se(&mut self) -> Result<i32> {
        let code_num = self.read_ue()?;

        if code_num == 0 {
            return Ok(0);
        }

        if code_num & 1 == 1 {
            return Ok(code_num.div_ceil(2) as i32);
        }

        Ok(-((code_num / 2) as i32))
    }

    pub fn read_bits(&mut self, n: usize) -> Result<usize> {
        let mut out = 0;
        for _ in 0..n {
            out = (out << 1) | (self.read_bit()? as usize);
        }
        Ok(out)
    }

    fn read_bit(&mut self) -> Result<u8> {
        ensure!(self.byte_pos < self.data.len(), "unexpected EOF");

        let byte = self.data[self.byte_pos];
        let bit = (byte >> (7 - self.bit_pos)) & 1;

        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }

        Ok(bit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ue_values() {
        // table 9-2 from spec h265_iso 23008-2
        let test_cases = vec![
            (vec![0b10000000], 0),
            (vec![0b01000000], 1),
            (vec![0b01100000], 2),
            (vec![0b00100000], 3),
            (vec![0b00101000], 4),
            (vec![0b00110000], 5),
            (vec![0b00111000], 6),
            (vec![0b00010000], 7),
            (vec![0b00010010], 8),
            (vec![0b00010100], 9),
        ];

        for (data, expected) in test_cases {
            let mut reader = RbspReader::new(&data);
            let result = reader.read_ue().unwrap();
            assert_eq!(result, expected, "Failed for codeNum {}", expected);
        }
    }

    #[test]
    fn test_se_values() {
        // table 9-3
        let test_cases = vec![
            (vec![0b10000000], 0),
            (vec![0b01000000], 1),
            (vec![0b01100000], -1),
            (vec![0b00100000], 2),
            (vec![0b00101000], -2),
            (vec![0b00110000], 3),
            (vec![0b00111000], -3),
        ];

        for (data, expected) in test_cases {
            let mut reader = RbspReader::new(&data);
            let result = reader.read_se().unwrap();
            assert_eq!(result, expected, "Failed for expected value {}", expected);
        }
    }

    #[test]
    fn test_emulation_prevention_no_pattern() {
        // No emulation prevention bytes - should be unchanged
        let input = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_emulation_prevention_single_zero() {
        // Single 0x00 is fine, not emulation prevention
        let input = vec![0x01, 0x00, 0x02];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_emulation_prevention_double_zero() {
        // 0x00 0x00 without 0x03 is fine
        let input = vec![0x01, 0x00, 0x00, 0x04];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_emulation_prevention_basic() {
        // 0x00 0x00 0x03 0x00 -> 0x00 0x00 (skip 0x03)
        let input = vec![0x00, 0x00, 0x03, 0x00];
        let expected = vec![0x00, 0x00, 0x00];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_emulation_prevention_all_valid_following_bytes() {
        // 0x00 0x00 0x03 followed by 0x00, 0x01, 0x02, or 0x03 should remove 0x03
        for following_byte in 0x00..=0x03 {
            let input = vec![0x00, 0x00, 0x03, following_byte];
            let expected = vec![0x00, 0x00, following_byte];
            let output = RbspReader::remove_emulation_prevention(&input);
            assert_eq!(
                output, expected,
                "Failed for following byte 0x{:02x}",
                following_byte
            );
        }
    }

    #[test]
    fn test_emulation_prevention_invalid_following_byte() {
        // 0x00 0x00 0x03 0x04 -> not emulation prevention, keep 0x03
        let input = vec![0x00, 0x00, 0x03, 0x04];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_emulation_prevention_at_end() {
        // 0x00 0x00 0x03 at end of stream should be removed
        let input = vec![0x01, 0x00, 0x00, 0x03];
        let expected = vec![0x01, 0x00, 0x00];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_emulation_prevention_multiple() {
        // Multiple emulation prevention bytes
        let input = vec![0x00, 0x00, 0x03, 0x00, 0xFF, 0x00, 0x00, 0x03, 0x01];
        let expected = vec![0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x01];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_emulation_prevention_with_prefix() {
        // Data before emulation prevention pattern
        let input = vec![0x42, 0x01, 0x01, 0x03, 0x70, 0x00, 0x00, 0x03, 0x00];
        let expected = vec![0x42, 0x01, 0x01, 0x03, 0x70, 0x00, 0x00, 0x00];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_emulation_prevention_real_sps_data() {
        // Real data from sample.heic SPS (after NAL header)
        let input = vec![
            0x01, 0x03, 0x70, 0x00, 0x00, 0x03, 0x00, 0xb0, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03,
            0x00, 0x5a, 0xa0, 0x04,
        ];
        // Patterns found:
        // - Position 3-6: 00 00 03 00 -> remove 0x03 at index 5
        // - Position 8-11: 00 00 03 00 -> remove 0x03 at index 10
        // - Position 11-14: 00 00 03 00 -> remove 0x03 at index 13 (overlapping pattern!)
        let expected = vec![
            0x01, 0x03, 0x70, 0x00, 0x00, 0x00, 0xb0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5a, 0xa0,
            0x04,
        ];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_emulation_prevention_consecutive_patterns() {
        // Consecutive emulation prevention patterns
        let input = vec![0x00, 0x00, 0x03, 0x00, 0x00, 0x03, 0x01];
        let expected = vec![0x00, 0x00, 0x00, 0x00, 0x01];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_emulation_prevention_empty() {
        // Empty input
        let input = vec![];
        let output = RbspReader::remove_emulation_prevention(&input);
        assert_eq!(output, input);
    }
}
