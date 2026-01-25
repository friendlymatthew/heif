use anyhow::{Result, ensure};

#[derive(Debug)]
pub struct RbspReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> RbspReader<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
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
            return Ok(((code_num + 1) / 2) as i32);
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
}
