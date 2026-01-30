// these correspond to the syntax elements listed in Table 9-4.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SyntaxElement {
    SaoMergeLeftFlag,
    SaoMergeUpFlag,
    SaoTypeIdxLuma,
    SaoTypeIdxChroma,
    SplitCuFlag,
    CuTransquantBypassFlag,
    CuSkipFlag,
    PredModeFlag,
    PartMode,
    PrevIntraLumaPredFlag,
    IntraChromaPredMode,
    RqtRootCbf,
    MergeFlag,
    MergeIdx,
    InterPredIdc,
    RefIdxL0,
    RefIdxL1,
    MvpL0Flag,
    MvpL1Flag,
    SplitTransformFlag,
    CbfLuma,
    CbfCb,
    CbfCr,
    AbsMvdGreater0Flag,
    AbsMvdGreater1Flag,
    CuQpDeltaAbs,
    CuChromaQpOffsetFlag,
    CuChromaQpOffsetIdx,
    Log2ResScaleAbsPlus1,
    ResScaleSignFlag,
    TransformSkipFlag,
    ExplicitRdpcmFlag,
    ExplicitRdpcmDirFlag,
    LastSigCoeffXPrefix,
    LastSigCoeffYPrefix,
    CodedSubBlockFlag,
    SigCoeffFlag,
    CoeffAbsLevelGreater1Flag,
    CoeffAbsLevelGreater2Flag,
}

impl SyntaxElement {
    pub const fn ctx_table(self) -> usize {
        match self {
            Self::SaoMergeLeftFlag => 5,           // Table 9-5
            Self::SaoMergeUpFlag => 5,             // Table 9-5 (same as SaoMergeLeftFlag)
            Self::SaoTypeIdxLuma => 6,             // Table 9-6
            Self::SaoTypeIdxChroma => 6,           // Table 9-6
            Self::SplitCuFlag => 7,                // Table 9-7
            Self::CuTransquantBypassFlag => 8,     // Table 9-8
            Self::CuSkipFlag => 9,                 // Table 9-9
            Self::PredModeFlag => 10,              // Table 9-10
            Self::PartMode => 11,                  // Table 9-11
            Self::PrevIntraLumaPredFlag => 12,     // Table 9-12
            Self::IntraChromaPredMode => 13,       // Table 9-13
            Self::RqtRootCbf => 14,                // Table 9-14
            Self::MergeFlag => 15,                 // Table 9-15
            Self::MergeIdx => 16,                  // Table 9-16
            Self::InterPredIdc => 17,              // Table 9-17
            Self::RefIdxL0 => 18,                  // Table 9-18
            Self::RefIdxL1 => 18,                  // Table 9-18 (same table)
            Self::MvpL0Flag => 19,                 // Table 9-19
            Self::MvpL1Flag => 19,                 // Table 9-19
            Self::SplitTransformFlag => 20,        // Table 9-20
            Self::CbfLuma => 21,                   // Table 9-21
            Self::CbfCb => 22,                     // Table 9-22
            Self::CbfCr => 22,                     // Table 9-22
            Self::AbsMvdGreater0Flag => 23,        // Table 9-23
            Self::AbsMvdGreater1Flag => 23,        // Table 9-23
            Self::CuQpDeltaAbs => 24,              // Table 9-24
            Self::TransformSkipFlag => 25,         // Table 9-25
            Self::LastSigCoeffXPrefix => 26,       // Table 9-26
            Self::LastSigCoeffYPrefix => 27,       // Table 9-27
            Self::CodedSubBlockFlag => 28,         // Table 9-28
            Self::SigCoeffFlag => 29,              // Table 9-29
            Self::CoeffAbsLevelGreater1Flag => 30, // Table 9-30
            Self::CoeffAbsLevelGreater2Flag => 31, // Table 9-31
            Self::ExplicitRdpcmFlag => 32,         // Table 9-32
            Self::ExplicitRdpcmDirFlag => 33,      // Table 9-33
            Self::CuChromaQpOffsetFlag => 34,      // Table 9-34
            Self::CuChromaQpOffsetIdx => 35,       // Table 9-35
            Self::Log2ResScaleAbsPlus1 => 36,      // Table 9-36
            Self::ResScaleSignFlag => 37,          // Table 9-37
        }
    }

    pub const fn init_values_i_slice(self) -> &'static [u8] {
        match self {
            // Table 9-5: sao_merge_left_flag and sao_merge_up_flag
            Self::SaoMergeLeftFlag | Self::SaoMergeUpFlag => &[153],

            // Table 9-6: sao_type_idx_luma and sao_type_idx_chroma
            Self::SaoTypeIdxLuma | Self::SaoTypeIdxChroma => &[200],

            // Table 9-7: split_cu_flag
            Self::SplitCuFlag => &[139, 141, 157],

            // Table 9-8: cu_transquant_bypass_flag
            Self::CuTransquantBypassFlag => &[154],

            // Table 9-9: cu_skip_flag
            // not used in i slices
            Self::CuSkipFlag => &[197, 185, 201],

            // Table 9-10: pred_mode_flag
            Self::PredModeFlag => &[149],

            // Table 9-11: part_mode
            Self::PartMode => &[184],

            // Table 9-12: prev_intra_luma_pred_flag
            Self::PrevIntraLumaPredFlag => &[184],

            // Table 9-13: intra_chroma_pred_mode
            Self::IntraChromaPredMode => &[63],

            // Table 9-14: rqt_root_cbf
            // not used in i slices
            Self::RqtRootCbf => &[79],

            // Table 9-15: merge_flag
            // not used in i slices
            Self::MergeFlag => &[110],

            // Table 9-16: merge_idx
            // not used in i slices
            Self::MergeIdx => &[122],

            // Table 9-17: inter_pred_idc
            // not used in i slices
            Self::InterPredIdc => &[95, 79, 63, 31, 31],

            // Table 9-18: ref_idx_l0 and ref_idx_l1
            // not used in i slices
            Self::RefIdxL0 | Self::RefIdxL1 => &[153, 153],

            // Table 9-19: mvp_l0_flag and mvp_l1_flag
            // not used in i slices
            Self::MvpL0Flag | Self::MvpL1Flag => &[168],

            // Table 9-20: split_transform_flag
            Self::SplitTransformFlag => &[153, 138, 138],

            // Table 9-21: cbf_luma
            Self::CbfLuma => &[111, 141],

            // Table 9-22: cbf_cb and cbf_cr
            // initType 0: ctxIdx 0..3 and 12
            // Indices 4-11 are for P/B slices (initType 1 and 2), filled with 154 as placeholder
            Self::CbfCb | Self::CbfCr => &[94, 138, 182, 154, 154, 154, 154, 154, 154, 154, 154, 154, 149],

            // Table 9-23: abs_mvd_greater0_flag and abs_mvd_greater1_flag
            // initType 0: ctxIdx 0 for greater0, ctxIdx 1 for greater1
            // We store both: [greater0_ctx0, greater1_ctx1]
            Self::AbsMvdGreater0Flag => &[140],
            Self::AbsMvdGreater1Flag => &[198],

            // Table 9-24: cu_qp_delta_abs
            // initType 0: ctxIdx 0..1, initValues [154, 154]
            Self::CuQpDeltaAbs => &[154, 154],

            // Table 9-25: transform_skip_flag
            // initType 0: ctxIdx 0, initValue 139
            Self::TransformSkipFlag => &[139],

            // Table 9-26: last_sig_coeff_x_prefix
            // initType 0: ctxIdx 0..17, 18 values
            Self::LastSigCoeffXPrefix => &[
                110, 110, 124, 125, 140, 153, 125, 127, 140, 109, 111, 143, 127, 111, 79, 108, 123,
                63,
            ],

            // Table 9-27: last_sig_coeff_y_prefix
            // initType 0: ctxIdx 0..17, 18 values (same as x_prefix)
            Self::LastSigCoeffYPrefix => &[
                110, 110, 124, 125, 140, 153, 125, 127, 140, 109, 111, 143, 127, 111, 79, 108, 123,
                63,
            ],

            // Table 9-28: coded_sub_block_flag
            // initType 0: ctxIdx 0..3, initValues [91, 171, 134, 141]
            Self::CodedSubBlockFlag => &[91, 171, 134, 141],

            // Table 9-29: sig_coeff_flag
            // initType 0: ctxIdx 0..41 and 126..127
            // Indices 42-125 are for P/B slices, filled with 154 as placeholder
            Self::SigCoeffFlag => &[
                // ctxIdx 0-41
                111, 111, 125, 110, 110, 94, 124, 108, 124, 107, 125, 141, 179, 153, 125, 107, 125,
                141, 179, 153, 125, 107, 125, 141, 179, 153, 125, 140, 139, 182, 182, 152, 136,
                152, 136, 153, 136, 139, 111, 136, 139, 111,
                // ctxIdx 42-125 (P/B slice contexts, placeholder)
                154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154,
                154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154,
                154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154,
                154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154,
                154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154, 154,
                154, 154, 154, 154,
                // ctxIdx 126-127
                111, 111,
            ],

            // Table 9-30: coeff_abs_level_greater1_flag
            // initType 0: ctxIdx 0..23, 24 values
            Self::CoeffAbsLevelGreater1Flag => &[
                140, 92, 137, 138, 140, 152, 138, 139, 153, 74, 149, 92, 139, 107, 122, 152, 140,
                179, 166, 182, 140, 227, 122, 197,
            ],

            // Table 9-31: coeff_abs_level_greater2_flag
            // initType 0: ctxIdx 0..5, 6 values
            Self::CoeffAbsLevelGreater2Flag => &[138, 153, 136, 167, 152, 152],

            // Table 9-32: explicit_rdpcm_flag
            // initType 0: ctxIdx 0, initValue 139
            Self::ExplicitRdpcmFlag => &[139],

            // Table 9-33: explicit_rdpcm_dir_flag
            // initType 0: ctxIdx 0, initValue 139
            Self::ExplicitRdpcmDirFlag => &[139],

            // Table 9-34: cu_chroma_qp_offset_flag
            // initType 0: ctxIdx 0, initValue 154
            Self::CuChromaQpOffsetFlag => &[154],

            // Table 9-35: cu_chroma_qp_offset_idx
            // initType 0: ctxIdx 0, initValue 154
            Self::CuChromaQpOffsetIdx => &[154],

            // Table 9-36: log2_res_scale_abs_plus1
            // initType 0: ctxIdx 0..7, 8 values (all 154)
            Self::Log2ResScaleAbsPlus1 => &[154, 154, 154, 154, 154, 154, 154, 154],

            // Table 9-37: res_scale_sign_flag
            // initType 0: ctxIdx 0..1, 2 values (all 154)
            Self::ResScaleSignFlag => &[154, 154],
        }
    }

    /// Returns all syntax elements that need to be initialized for I-slices.
    pub const fn all_i_slice_elements() -> &'static [Self] {
        &[
            Self::SaoMergeLeftFlag,
            Self::SaoMergeUpFlag,
            Self::SaoTypeIdxLuma,
            Self::SaoTypeIdxChroma,
            Self::SplitCuFlag,
            Self::CuTransquantBypassFlag,
            Self::PredModeFlag,
            Self::PartMode,
            Self::PrevIntraLumaPredFlag,
            Self::IntraChromaPredMode,
            Self::SplitTransformFlag,
            Self::CbfLuma,
            Self::CbfCb,
            Self::CbfCr,
            Self::AbsMvdGreater0Flag,
            Self::AbsMvdGreater1Flag,
            Self::CuQpDeltaAbs,
            Self::TransformSkipFlag,
            Self::LastSigCoeffXPrefix,
            Self::LastSigCoeffYPrefix,
            Self::CodedSubBlockFlag,
            Self::SigCoeffFlag,
            Self::CoeffAbsLevelGreater1Flag,
            Self::CoeffAbsLevelGreater2Flag,
            Self::ExplicitRdpcmFlag,
            Self::ExplicitRdpcmDirFlag,
            Self::CuChromaQpOffsetFlag,
            Self::CuChromaQpOffsetIdx,
            Self::Log2ResScaleAbsPlus1,
            Self::ResScaleSignFlag,
        ]
    }
}
