use crate::{cabac::ArithmeticDecoderEngine, hevc::RbspReader};
use anyhow::Result;

#[derive(Debug)]
pub struct CabacDecoder<'a, 'b> {
    engine: ArithmeticDecoderEngine<'a, 'b>,
}

impl<'a, 'b> CabacDecoder<'a, 'b> {
    pub fn try_new(
        reader: &'a mut RbspReader<'b>,
        init_qp_minus26: i32,
        slice_qp_delta: i32,
    ) -> Result<Self> {
        let slice_qp = 26 + init_qp_minus26 + slice_qp_delta;

        let mut engine = ArithmeticDecoderEngine::try_new(reader)?;
        engine.init_all_contexts(slice_qp);

        Ok(Self { engine })
    }

    pub fn decode_intra_chroma_pred_mode(
        &mut self,
        ctx_table: usize,
        ctx_idx: usize,
    ) -> Result<u16> {
        let mut bin_idx = 0;
        decode_intra_chroma_pred_mode_bins(|| {
            let bypass = bin_idx > 0;
            bin_idx += 1;

            self.engine.decode_bin(ctx_table, ctx_idx, bypass)
        })
    }

    // table 9-43
    // binIdx 0: ctxInc = 0 (context-coded)
    // binIdx 1-4: ctxInc = 1 (context-coded)
    // binIdx > 4: bypass (EG0 suffix)
    pub fn decode_cu_qp_delta_abs(&mut self, ctx_table: usize) -> Result<u16> {
        let mut bin_idx = 0;
        decode_cu_qp_delta_abs(|| {
            let (ctx_idx, bypass) = match bin_idx {
                0 => (0, false),
                1..=4 => (1, false),
                _ => (0, true), // bypass for EG0 suffix
            };
            bin_idx += 1;

            self.engine.decode_bin(ctx_table, ctx_idx, bypass)
        })
    }

    pub fn decode_coeff_abs_level_remaining(
        &mut self,
        state: &mut CoeffAbsLevelState,
        base_level: u16,
    ) -> Result<u16> {
        decode_coeff_abs_level_remaining(state, base_level, || self.engine.decode_bypass())
    }

    pub fn decode_fl_bypass(&mut self, c_max: u16) -> Result<u16> {
        decode_fixed_length(c_max, || self.engine.decode_bypass())
    }

    pub fn decode_tr_bypass(&mut self, c_max: u16, c_rice_param: u8) -> Result<u16> {
        decode_truncated_rice(c_max, c_rice_param, || self.engine.decode_bypass())
    }

    pub fn decode_bypass(&mut self) -> Result<bool> {
        self.engine.decode_bypass()
    }

    pub fn decode_terminate(&mut self) -> Result<bool> {
        self.engine.decode_terminate()
    }

    // decode single context-coded bin (FL cMax=1)
    // Used for flags: split_cu_flag, pred_mode_flag, cbf_luma, etc.
    pub fn decode_bin_context(&mut self, ctx_table: usize, ctx_idx: usize) -> Result<bool> {
        self.engine.decode_bin(ctx_table, ctx_idx, false)
    }

    // sao_type_idx (Table 9-43)
    // TR cMax=2, cRiceParam=0
    // binIdx 0: context-coded
    // binIdx 1-2: bypass
    pub fn decode_sao_type_idx(&mut self, ctx_table: usize, ctx_idx: usize) -> Result<u16> {
        let mut bin_idx = 0;
        decode_truncated_rice(2, 0, || {
            let bypass = bin_idx > 0;
            bin_idx += 1;
            self.engine.decode_bin(ctx_table, ctx_idx, bypass)
        })
    }

    // last_sig_coeff_x_prefix / last_sig_coeff_y_prefix (Table 9-43)
    // TR with cRiceParam=0, all context-coded
    // ctxIdx derived per Section 9.3.4.2.3:
    //   ctxInc = (binIdx >> ctxShift) + ctxOffset
    //   For luma (cIdx=0): ctxOffset = 3*(log2TrafoSize-2) + ((log2TrafoSize-1)>>2)
    //                      ctxShift = (log2TrafoSize+1)>>2
    //   For chroma:        ctxOffset = 15, ctxShift = log2TrafoSize - 2
    pub fn decode_last_sig_coeff_prefix(
        &mut self,
        c_max: u16,
        ctx_table: usize,
        c_idx: u8,
        log2_trafo_size: u8,
    ) -> Result<u16> {
        let (ctx_offset, ctx_shift) = if c_idx == 0 {
            let offset = 3 * (log2_trafo_size as usize - 2) + ((log2_trafo_size as usize - 1) >> 2);
            let shift = (log2_trafo_size as usize + 1) >> 2;
            (offset, shift)
        } else {
            (15usize, log2_trafo_size as usize - 2)
        };

        let mut bin_idx = 0usize;
        decode_truncated_rice(c_max, 0, || {
            let ctx_inc = (bin_idx >> ctx_shift) + ctx_offset;
            bin_idx += 1;
            self.engine.decode_bin(ctx_table, ctx_inc, false)
        })
    }

    // part_mode (Table 9-40, Table 9-43)
    // For I-slices (MODE_INTRA):
    //   - log2CbSize > MinCbLog2SizeY: only PART_2Nx2N allowed, no bin to decode
    //   - log2CbSize == MinCbLog2SizeY: decode bin - "1" = PART_2Nx2N (0), "0" = PART_NxN (3)
    pub fn decode_part_mode_intra(
        &mut self,
        ctx_table: usize,
        ctx_idx: usize,
        log2_cb_size: u8,
        min_cb_log2_size_y: u8,
    ) -> Result<u8> {
        if log2_cb_size > min_cb_log2_size_y {
            return Ok(0);
        }

        let bin = self.engine.decode_bin(ctx_table, ctx_idx, false)?;
        if bin { Ok(0) } else { Ok(3) }
    }
}

fn decode_fixed_length<F>(c_max: u16, mut get_bin: F) -> Result<u16>
where
    F: FnMut() -> Result<bool>,
{
    let num_bits = u16::BITS - c_max.leading_zeros();

    let mut out = 0u16;
    for _ in 0..num_bits {
        out = (out << 1) | get_bin()? as u16;
    }

    Ok(out)
}

fn decode_truncated_rice<F>(c_max: u16, c_rice_param: u8, mut get_bin: F) -> Result<u16>
where
    F: FnMut() -> Result<bool>,
{
    let prefix_max = c_max >> c_rice_param;

    let mut prefix_val = 0u16;
    while prefix_val < prefix_max {
        let bin = get_bin()?;

        if !bin {
            break;
        }

        prefix_val += 1;
    }

    let suffix_val = if c_rice_param > 0 && prefix_val < prefix_max {
        decode_fixed_length((1 << c_rice_param) - 1, get_bin)?
    } else {
        0
    };

    Ok((prefix_val << c_rice_param) + suffix_val)
}

fn decode_intra_chroma_pred_mode_bins<F>(mut get_bin: F) -> Result<u16>
where
    F: FnMut() -> Result<bool>,
{
    let first_bin = get_bin()?;

    if !first_bin {
        return Ok(4);
    }

    let suffix = decode_fixed_length(3, get_bin)?;
    Ok(suffix)
}

fn decode_egk<F>(k: u8, mut get_bin: F) -> Result<u16>
where
    F: FnMut() -> Result<bool>,
{
    let mut num_ones: u8 = 0;
    while get_bin()? {
        num_ones += 1;
    }

    let suffix_len = num_ones + k;
    let mut suffix: u16 = 0;
    for _ in 0..suffix_len {
        suffix = (suffix << 1) | get_bin()? as u16;
    }

    Ok((((1u16 << num_ones) - 1) << k) + suffix)
}

#[derive(Debug, Clone, Copy)]
pub struct CoeffAbsLevelState {
    pub c_last_abs_level: u16,
    pub c_last_rice_param: u8,
}

impl Default for CoeffAbsLevelState {
    fn default() -> Self {
        Self {
            c_last_abs_level: 0,
            c_last_rice_param: 0,
        }
    }
}

fn decode_coeff_abs_level_remaining<F>(
    state: &mut CoeffAbsLevelState,
    base_level: u16,
    mut get_bin: F,
) -> Result<u16>
where
    F: FnMut() -> Result<bool>,
{
    let threshold = 3 * (1u16 << state.c_last_rice_param);
    let c_rice_param = if state.c_last_abs_level > threshold {
        (state.c_last_rice_param + 1).min(4)
    } else {
        state.c_last_rice_param
    };

    let c_max = 4u16 << c_rice_param;

    let prefix_val = decode_truncated_rice(c_max, c_rice_param, &mut get_bin)?;

    let coeff_abs_level_remaining = if prefix_val == c_max {
        let suffix_val = decode_egk(c_rice_param + 1, &mut get_bin)?;
        c_max + suffix_val
    } else {
        prefix_val
    };

    let c_abs_level = base_level + coeff_abs_level_remaining;
    state.c_last_abs_level = c_abs_level;
    state.c_last_rice_param = c_rice_param;

    Ok(coeff_abs_level_remaining)
}

fn decode_cu_qp_delta_abs<F>(mut get_bin: F) -> Result<u16>
where
    F: FnMut() -> Result<bool>,
{
    let mut prefix_val = 0;

    while prefix_val < 5 {
        let bin = get_bin()?;
        if !bin {
            break;
        }

        prefix_val += 1;
    }

    if prefix_val == 5 {
        let suffix_val = decode_egk(0, &mut get_bin)?;
        return Ok(5 + suffix_val);
    }

    Ok(prefix_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a bin iterator from a slice
    fn bin_iter(bins: &[bool]) -> impl FnMut() -> Result<bool> + '_ {
        let mut idx = 0;
        move || {
            let bin = bins[idx];
            idx += 1;
            Ok(bin)
        }
    }

    // table 9-39 with rice param 0
    // prefixVal | Bin string
    // 0         | 0
    // 1         | 1 0
    // 2         | 1 1 0
    // 3         | 1 1 1 0
    // 4         | 1 1 1 1 0
    // 5         | 1 1 1 1 1 0
    #[test]
    fn test_tr_table_9_39_unary() {
        // cMax=5, cRiceParam=0 (pure unary, no suffix)
        let c_max: u16 = 5;
        let c_rice_param: u8 = 0;

        let val = decode_truncated_rice(c_max, c_rice_param, bin_iter(&[false])).unwrap();
        assert_eq!(val, 0);

        let val = decode_truncated_rice(c_max, c_rice_param, bin_iter(&[true, false])).unwrap();
        assert_eq!(val, 1);

        let val =
            decode_truncated_rice(c_max, c_rice_param, bin_iter(&[true, true, false])).unwrap();
        assert_eq!(val, 2);

        let val = decode_truncated_rice(c_max, c_rice_param, bin_iter(&[true, true, true, false]))
            .unwrap();
        assert_eq!(val, 3);

        let val = decode_truncated_rice(
            c_max,
            c_rice_param,
            bin_iter(&[true, true, true, true, false]),
        )
        .unwrap();
        assert_eq!(val, 4);

        let val = decode_truncated_rice(
            c_max,
            c_rice_param,
            bin_iter(&[true, true, true, true, true]),
        )
        .unwrap();
        assert_eq!(val, 5);
    }

    // Table 9-41: intra_chroma_pred_mode binarization
    // Value | Bin string
    // 4     | 0
    // 0     | 100
    // 1     | 101
    // 2     | 110
    // 3     | 111
    #[test]
    fn test_intra_chroma_pred_mode_table_9_41() {
        // Value 4: "0"
        let val = decode_intra_chroma_pred_mode_bins(bin_iter(&[false])).unwrap();
        assert_eq!(val, 4);

        // Value 0: "100"
        let val = decode_intra_chroma_pred_mode_bins(bin_iter(&[true, false, false])).unwrap();
        assert_eq!(val, 0);

        // Value 1: "101"
        let val = decode_intra_chroma_pred_mode_bins(bin_iter(&[true, false, true])).unwrap();
        assert_eq!(val, 1);

        // Value 2: "110"
        let val = decode_intra_chroma_pred_mode_bins(bin_iter(&[true, true, false])).unwrap();
        assert_eq!(val, 2);

        // Value 3: "111"
        let val = decode_intra_chroma_pred_mode_bins(bin_iter(&[true, true, true])).unwrap();
        assert_eq!(val, 3);
    }
}
