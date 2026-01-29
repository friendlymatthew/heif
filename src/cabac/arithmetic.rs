use crate::hevc::RbspReader;
use anyhow::{Result, ensure};

#[derive(Debug)]
pub struct ArithmeticDecoderEngine<'a, 'b> {
    // the start is [256..510]
    pub ivl_curr_range: u16,
    pub ivl_offset: u16,

    val_mps: Vec<bool>,
    p_state_idx: Vec<u8>,

    reader: &'a mut RbspReader<'b>,
}

impl<'a, 'b> ArithmeticDecoderEngine<'a, 'b> {
    pub fn try_new(reader: &'a mut RbspReader<'b>) -> Result<Self> {
        let ivl_offset = reader.read_bits(9)? as u16;

        ensure!(ivl_offset != 510 || ivl_offset != 511);

        Ok(Self {
            ivl_curr_range: 510,
            ivl_offset,
            val_mps: vec![false; MAX_CTX_IDX],
            p_state_idx: vec![0u8; MAX_CTX_IDX],
            reader,
        })
    }

    pub fn init_single_context(&mut self, ctx_id: usize, slice_qp: i32, init_value: u8) {
        let slope_idx = (init_value >> 4) as i32;
        let offset_idx = (init_value & 15) as i32;

        let m = slope_idx * 5 - 45;
        let n = (offset_idx << 3) - 16;

        let slice_qp_clamped = slice_qp.clamp(0, 51);
        let pre_ctx_state = (((m * slice_qp_clamped) >> 4) + n).clamp(1, 126) as u8;

        self.val_mps[ctx_id] = pre_ctx_state > 63;
        self.p_state_idx[ctx_id] = if self.val_mps[ctx_id] {
            pre_ctx_state - 64
        } else {
            63 - pre_ctx_state
        };
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

        self.decode_decision(ctx_idx)
    }

    fn decode_decision(&mut self, ctx_idx: usize) -> Result<bool> {
        let q_range_idx = (self.ivl_curr_range >> 6) & 3;
        let p_state_idx = self.p_state_idx[ctx_idx];

        let ivl_lps_range = RANGE_TAB_LPS[p_state_idx as usize][q_range_idx as usize];

        self.ivl_curr_range -= ivl_lps_range as u16;

        let bin_val = if self.ivl_offset >= self.ivl_curr_range {
            let bin_val = !self.val_mps[ctx_idx];
            self.ivl_offset -= self.ivl_curr_range;
            self.ivl_curr_range = ivl_lps_range as u16;

            if p_state_idx == 0 {
                self.val_mps[ctx_idx] = !self.val_mps[ctx_idx];
            }
            self.p_state_idx[ctx_idx] = TRANS_IDX_LPS[p_state_idx as usize];

            bin_val
        } else {
            self.p_state_idx[ctx_idx] = TRANS_IDX_MPS[p_state_idx as usize];
            self.val_mps[ctx_idx]
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

// safe value covering all init types
pub const MAX_CTX_IDX: usize = 160;

// these are all the init value pub constants for init type = 0 (i slices)

// table 9-5: sao_merge_left_flag and sao_merge_up_flag
pub const INIT_SAO_MERGE_FLAG: u8 = 153;

// table 9-6: sao_type_idx_luma and sao_type_idx_chroma
pub const INIT_SAO_TYPE_IDX: u8 = 200;

// table 9-7: split_cu_flag (3 contexts)
pub const INIT_SPLIT_CU_FLAG: [u8; 3] = [139, 141, 157];

// table 9-8: cu_transquant_bypass_flag
pub const INIT_CU_TRANSQUANT_BYPASS_FLAG: u8 = 154;

// table 9-9: cu_skip_flag (3 contexts) - NOT USED in I-slices
// pub const INIT_CU_SKIP_FLAG: [u8; 3] = [197, 185, 201];

// table 9-10: pred_mode_flag - NOT USED in I-slices
// pub const INIT_PRED_MODE_FLAG: u8 = 149;

// table 9-11: part_mode
// For I-slices with log2CbSize == MinCbLog2SizeY (smallest CU)
pub const INIT_PART_MODE_SMALL: [u8; 2] = [184, 154];
// For I-slices with log2CbSize > MinCbLog2SizeY
pub const INIT_PART_MODE_LARGE: u8 = 184;

// table 9-12: prev_intra_luma_pred_flag
pub const INIT_PREV_INTRA_LUMA_PRED_FLAG: u8 = 184;

// table 9-13: intra_chroma_pred_mode
pub const INIT_INTRA_CHROMA_PRED_MODE: u8 = 63;

// table 9-14: rqt_root_cbf - NOT USED in I-slices
// pub const INIT_RQT_ROOT_CBF: u8 = 79;

// table 9-15: merge_flag - NOT USED in I-slices
// pub const INIT_MERGE_FLAG: u8 = 110;

// table 9-16: merge_idx - NOT USED in I-slices
// pub const INIT_MERGE_IDX: u8 = 122;

// table 9-17: inter_pred_idc - NOT USED in I-slices

// table 9-18: ref_idx_l0 and ref_idx_l1 - NOT USED in   I-slices

// table 9-19: mvp_l0_flag and mvp_l1_flag - NOT USED in I-slices

// table 9-20: split_transform_flag (3 contexts)
pub const INIT_SPLIT_TRANSFORM_FLAG: [u8; 3] = [153, 138, 138];

// table 9-21: cbf_luma (2 contexts)
pub const INIT_CBF_LUMA: [u8; 2] = [111, 141];

// table 9-22: cbf_cb and cbf_cr (4 contexts + 1 additional)
pub const INIT_CBF_CHROMA: [u8; 5] = [94, 138, 182, 154, 149];

// table 9-23: abs_mvd_greater0_flag and abs_mvd_greater1_flag - NOT USED in I-slices

// table 9-24: cu_qp_delta_abs (2 contexts)
pub const INIT_CU_QP_DELTA_ABS: [u8; 2] = [154, 154];

// table 9-25: transform_skip_flag
// [0] for luma (cIdx == 0)
// [1] for chroma (cIdx == 1 or 2)
pub const INIT_TRANSFORM_SKIP_FLAG: [u8; 2] = [139, 139];

// table 9-26: last_sig_coeff_x_prefix (18 contexts)
pub const INIT_LAST_SIG_COEFF_X_PREFIX: [u8; 18] = [
    110, 110, 124, 125, 140, 153, 125, 127, 140, 109, 111, 143, 127, 111, 79, 108, 123, 63,
];

// table 9-27: last_sig_coeff_y_prefix (18 contexts)
pub const INIT_LAST_SIG_COEFF_Y_PREFIX: [u8; 18] = [
    110, 110, 124, 125, 140, 153, 125, 127, 140, 109, 111, 143, 127, 111, 79, 108, 123, 63,
];

// table 9-28: coded_sub_block_flag (4 contexts)
pub const INIT_CODED_SUB_BLOCK_FLAG: [u8; 4] = [91, 171, 134, 141];

// table 9-29: sig_coeff_flag (42 contexts for luma, 42 for chroma)
// First 42 are for luma (cIdx == 0)
pub const INIT_SIG_COEFF_FLAG_LUMA: [u8; 42] = [
    111, 111, 125, 110, 110, 94, 124, 108, 124, 107, 125, 141, 179, 153, 125, 107, 125, 141, 179,
    153, 125, 107, 125, 141, 179, 153, 125, 140, 139, 182, 182, 152, 136, 152, 136, 153, 136, 139,
    111, 136, 139, 111,
];
// Next contexts are for chroma (starting at offset 42+)
// But let's keep them separate for clarity
pub const INIT_SIG_COEFF_FLAG_CHROMA: [u8; 27] = [
    155, 154, 139, 153, 139, 123, 123, 63, 153, 166, 183, 140, 136, 153, 154, 166, 183, 140, 136,
    153, 154, 166, 183, 140, 136, 153, 154,
];

// table 9-30: coeff_abs_level_greater1_flag (24 contexts)
pub const INIT_COEFF_ABS_LEVEL_GREATER1_FLAG: [u8; 24] = [
    140, 92, 137, 138, 140, 152, 138, 139, 153, 74, 149, 92, 139, 107, 122, 152, 140, 179, 166,
    182, 140, 227, 122, 197,
];

// table 9-31: coeff_abs_level_greater2_flag (6 contexts)
pub const INIT_COEFF_ABS_LEVEL_GREATER2_FLAG: [u8; 6] = [138, 153, 136, 167, 152, 152];

// table 9-32: explicit_rdpcm_flag (2 contexts)
pub const INIT_EXPLICIT_RDPCM_FLAG: [u8; 2] = [139, 139];

// table 9-33: explicit_rdpcm_dir_flag (2 contexts)
pub const INIT_EXPLICIT_RDPCM_DIR_FLAG: [u8; 2] = [139, 139];

// table 9-34: cu_chroma_qp_offset_flag
pub const INIT_CU_CHROMA_QP_OFFSET_FLAG: u8 = 154;

// table 9-35: cu_chroma_qp_offset_idx
pub const INIT_CU_CHROMA_QP_OFFSET_IDX: u8 = 154;

// table 9-36: log2_res_scale_abs_plus1 (8 contexts for each component)
pub const INIT_LOG2_RES_SCALE_ABS_PLUS1: [u8; 8] = [154, 154, 154, 154, 154, 154, 154, 154];

// table 9-37: res_scale_sign_flag (2 contexts)
pub const INIT_RES_SCALE_SIGN_FLAG: [u8; 2] = [154, 154];
