use std::collections::HashMap;

use crate::{cabac::SyntaxElement, hevc::RbspReader};
use anyhow::{Result, anyhow, ensure};

type CtxKey = (usize, usize);

/// CABAC arithmetic decoding engine implementing Section 9.3.4.3 of the H.265 specification.
///
/// This engine maintains the state of the arithmetic decoder and stores context variables
/// in a 2D structure indexed by (ctxTable, ctxIdx).
#[derive(Debug)]
pub struct ArithmeticDecoderEngine<'a, 'b> {
    pub ivl_curr_range: u16,
    pub ivl_offset: u16,

    val_mps: HashMap<CtxKey, bool>,
    p_state_idx: HashMap<CtxKey, u8>,
    reader: &'a mut RbspReader<'b>,
}

impl<'a, 'b> ArithmeticDecoderEngine<'a, 'b> {
    pub fn try_new(reader: &'a mut RbspReader<'b>) -> Result<Self> {
        let ivl_offset = reader.read_bits(9)? as u16;

        ensure!(
            ivl_offset != 510 && ivl_offset != 511,
            "Invalid ivlOffset value"
        );

        Ok(Self {
            ivl_curr_range: 510,
            ivl_offset,
            val_mps: HashMap::new(),
            p_state_idx: HashMap::new(),
            reader,
        })
    }

    pub fn init_all_contexts(&mut self, slice_qp: i32) {
        for &syntax_element in SyntaxElement::all_i_slice_elements() {
            let ctx_table = syntax_element.ctx_table();
            let init_values = syntax_element.init_values_i_slice();

            for (ctx_idx, &init_value) in init_values.iter().enumerate() {
                self.init_single_context(ctx_table, ctx_idx, slice_qp, init_value);
            }
        }
    }

    fn init_single_context(
        &mut self,
        ctx_table: usize,
        ctx_idx: usize,
        slice_qp: i32,
        init_value: u8,
    ) {
        let slope_idx = (init_value >> 4) as i32;
        let offset_idx = (init_value & 15) as i32;

        let m = slope_idx * 5 - 45;
        let n = (offset_idx << 3) - 16;

        let slice_qp_clamped = slice_qp.clamp(0, 51);
        let pre_ctx_state = (((m * slice_qp_clamped) >> 4) + n).clamp(1, 126) as u8;

        let val_mps = pre_ctx_state > 63;
        let p_state_idx = if val_mps {
            pre_ctx_state - 64
        } else {
            63 - pre_ctx_state
        };

        let key = (ctx_table, ctx_idx);

        let out = self.val_mps.insert(key, val_mps);
        debug_assert!(out.is_none());

        let out = self.p_state_idx.insert(key, p_state_idx);
        debug_assert!(out.is_none());
    }

    pub fn decode_bin(
        &mut self,
        ctx_table: usize,
        ctx_idx: usize,
        bypass_flag: bool,
    ) -> Result<bool> {
        if bypass_flag {
            return self.decode_bypass();
        }

        if ctx_table == 0 && ctx_idx == 0 {
            return self.decode_terminate();
        }

        self.decode_decision(ctx_table, ctx_idx)
    }

    fn decode_decision(&mut self, ctx_table: usize, ctx_idx: usize) -> Result<bool> {
        let q_range_idx = (self.ivl_curr_range >> 6) & 3;

        let ctx_key = (ctx_table, ctx_idx);

        let &p_state_idx = self
            .p_state_idx
            .get(&ctx_key)
            .ok_or_else(|| anyhow!("expect valid ctx key"))?;

        let ivl_lps_range = RANGE_TAB_LPS[p_state_idx as usize][q_range_idx as usize];

        self.ivl_curr_range -= ivl_lps_range as u16;

        let bin_val = if self.ivl_offset >= self.ivl_curr_range {
            let bin_val = !self.val_mps.get(&ctx_key).expect("valid");
            self.ivl_offset -= self.ivl_curr_range;
            self.ivl_curr_range = ivl_lps_range as u16;

            if p_state_idx == 0 {
                let val_mps = self.val_mps.get_mut(&ctx_key).expect("valid");
                *val_mps = !*val_mps;
            }

            self.p_state_idx
                .insert(ctx_key, TRANS_IDX_LPS[p_state_idx as usize]);

            bin_val
        } else {
            self.p_state_idx
                .insert(ctx_key, TRANS_IDX_MPS[p_state_idx as usize]);

            *self.val_mps.get(&ctx_key).expect("valid")
        };

        self.try_renorm()?;

        Ok(bin_val)
    }

    fn try_renorm(&mut self) -> Result<()> {
        if self.ivl_curr_range < 256 {
            self.ivl_curr_range <<= 1;
            self.ivl_offset = (self.ivl_offset << 1) | self.reader.read_bits(1)? as u16;
        }

        Ok(())
    }

    fn decode_bypass(&mut self) -> Result<bool> {
        self.ivl_offset <<= 1;
        self.ivl_offset |= self.reader.read_bits(1)? as u16;

        if self.ivl_offset >= self.ivl_curr_range {
            self.ivl_offset -= self.ivl_curr_range;

            return Ok(true);
        }

        Ok(false)
    }

    fn decode_terminate(&mut self) -> Result<bool> {
        self.ivl_curr_range -= 2;

        if self.ivl_offset >= self.ivl_curr_range {
            return Ok(true);
        }

        self.try_renorm()?;

        Ok(false)
    }
}

// Table 9-45: State transition tables
pub const TRANS_IDX_MPS: [u8; 64] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 62, 63,
];

pub const TRANS_IDX_LPS: [u8; 64] = [
    0, 0, 1, 2, 2, 4, 4, 5, 6, 7, 8, 9, 9, 11, 11, 12, 13, 13, 15, 15, 16, 16, 18, 18, 19, 19, 21,
    21, 22, 22, 23, 24, 24, 25, 26, 26, 27, 27, 28, 29, 29, 30, 30, 30, 31, 32, 32, 33, 33, 33, 34,
    34, 35, 35, 35, 36, 36, 36, 37, 37, 37, 38, 38, 63,
];

// Table 9-46: Range LPS lookup [pstateIdx][rangeIdx]
pub const RANGE_TAB_LPS: [[u8; 4]; 64] = [
    [128, 176, 208, 240],
    [128, 167, 197, 227],
    [128, 158, 187, 216],
    [123, 150, 178, 205],
    [116, 142, 169, 195],
    [111, 135, 160, 185],
    [105, 128, 152, 175],
    [100, 122, 144, 166],
    [95, 116, 137, 158],
    [90, 110, 130, 150],
    [85, 104, 123, 142],
    [81, 99, 117, 135],
    [77, 94, 111, 128],
    [73, 89, 105, 122],
    [69, 85, 100, 116],
    [66, 80, 95, 110],
    [62, 76, 90, 104],
    [59, 72, 86, 99],
    [56, 69, 81, 94],
    [53, 65, 77, 89],
    [51, 62, 73, 85],
    [48, 59, 69, 80],
    [46, 56, 66, 76],
    [43, 53, 63, 72],
    [41, 50, 59, 69],
    [39, 48, 56, 65],
    [37, 45, 54, 62],
    [35, 43, 51, 59],
    [33, 41, 48, 56],
    [32, 39, 46, 53],
    [30, 37, 43, 50],
    [29, 35, 41, 48],
    [27, 33, 39, 45],
    [26, 31, 37, 43],
    [24, 30, 35, 41],
    [23, 28, 33, 39],
    [22, 27, 32, 37],
    [21, 26, 30, 35],
    [20, 24, 29, 33],
    [19, 23, 27, 31],
    [18, 22, 26, 30],
    [17, 21, 25, 28],
    [16, 20, 23, 27],
    [15, 19, 22, 25],
    [14, 18, 21, 24],
    [14, 17, 20, 23],
    [13, 16, 19, 22],
    [12, 15, 18, 21],
    [12, 14, 17, 20],
    [11, 14, 16, 19],
    [11, 13, 15, 18],
    [10, 12, 15, 17],
    [10, 12, 14, 16],
    [9, 11, 13, 15],
    [9, 11, 12, 14],
    [8, 10, 12, 14],
    [8, 9, 11, 13],
    [7, 9, 11, 12],
    [7, 9, 10, 12],
    [7, 8, 10, 11],
    [6, 8, 9, 11],
    [6, 7, 9, 10],
    [6, 7, 8, 9],
    [2, 2, 2, 2],
];
