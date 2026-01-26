#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPrimaries {
    Reserved0,
    BT709,
    Unspecified,
    Reserved3,
    BT470M,
    BT470BG,
    BT601,
    SMPTE240M,
    GenericFilm,
    BT2020,
    ST428,
    DciP3,
    DisplayP3,
    Other(u8),
}

impl From<u8> for ColorPrimaries {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Reserved0,
            1 => Self::BT709,
            2 => Self::Unspecified,
            3 => Self::Reserved3,
            4 => Self::BT470M,
            5 => Self::BT470BG,
            6 => Self::BT601,
            7 => Self::SMPTE240M,
            8 => Self::GenericFilm,
            9 => Self::BT2020,
            10 => Self::ST428,
            11 => Self::DciP3,
            12 => Self::DisplayP3,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferCharacteristics {
    Reserved0,
    BT709,
    Unspecified,
    Reserved3,
    Gamma22,
    Gamma28,
    BT601,
    SMPTE240M,
    Linear,
    Log100,
    Log316,
    IEC61966_2_4,
    BT1361,
    SRGB,
    BT2020_10bit,
    BT2020_12bit,
    ST2084,
    ST428,
    HLG,
    Other(u8),
}

impl From<u8> for TransferCharacteristics {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Reserved0,
            1 => Self::BT709,
            2 => Self::Unspecified,
            3 => Self::Reserved3,
            4 => Self::Gamma22,
            5 => Self::Gamma28,
            6 => Self::BT601,
            7 => Self::SMPTE240M,
            8 => Self::Linear,
            9 => Self::Log100,
            10 => Self::Log316,
            11 => Self::IEC61966_2_4,
            12 => Self::BT1361,
            13 => Self::SRGB,
            14 => Self::BT2020_10bit,
            15 => Self::BT2020_12bit,
            16 => Self::ST2084,
            17 => Self::ST428,
            18 => Self::HLG,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixCoefficients {
    Identity,
    BT709,
    Unspecified,
    Reserved3,
    FCC,
    BT470BG,
    BT601,
    SMPTE240M,
    YCgCo,
    BT2020NonConst,
    BT2020Const,
    SMPTE2085,
    ChromaDerivedNonConst,
    ChromaDerivedConst,
    ICtCp,
    Other(u8),
}

impl From<u8> for MatrixCoefficients {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Identity,
            1 => Self::BT709,
            2 => Self::Unspecified,
            3 => Self::Reserved3,
            4 => Self::FCC,
            5 => Self::BT470BG,
            6 => Self::BT601,
            7 => Self::SMPTE240M,
            8 => Self::YCgCo,
            9 => Self::BT2020NonConst,
            10 => Self::BT2020Const,
            11 => Self::SMPTE2085,
            12 => Self::ChromaDerivedNonConst,
            13 => Self::ChromaDerivedConst,
            14 => Self::ICtCp,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaFormat {
    Monochrome,
    YUV420,
    YUV422,
    YUV444,
}

impl TryFrom<u32> for ChromaFormat {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Monochrome),
            1 => Ok(Self::YUV420),
            2 => Ok(Self::YUV422),
            3 => Ok(Self::YUV444),
            other => Err(anyhow::anyhow!("invalid chroma_format_idc: {}", other)),
        }
    }
}

#[derive(Debug)]
pub struct HEVCDecoderConfigurationRecord {
    pub configuration_version: u8,
    pub(crate) general_profile_byte: u8,
    pub general_profile_compatibility_flags: u32,
    pub general_constraint_indicator_flags: u64,
    pub general_level_idc: u8,
    pub(crate) min_spatial_segmentation: u16,
    pub(crate) parallelism_byte: u8,
    pub(crate) chroma_format_byte: u8,
    pub(crate) bit_depth_luma_byte: u8,
    pub(crate) bit_depth_chroma_byte: u8,
    pub avg_frame_rate: u16,
    pub(crate) frame_rate_byte: u8,
    pub arrays: Box<[NalArray]>,
}

impl HEVCDecoderConfigurationRecord {
    pub const fn general_profile_space(&self) -> u8 {
        (self.general_profile_byte >> 6) & 0x03
    }

    pub const fn general_tier_flag(&self) -> bool {
        (self.general_profile_byte & 0x20) != 0
    }

    pub const fn general_profile_idc(&self) -> u8 {
        self.general_profile_byte & 0x1F
    }

    pub const fn min_spatial_segmentation_idc(&self) -> u16 {
        self.min_spatial_segmentation & 0x0FFF
    }

    pub const fn parallelism_type(&self) -> u8 {
        self.parallelism_byte & 0x03
    }

    pub const fn chroma_format_idc(&self) -> u8 {
        self.chroma_format_byte & 0x03
    }

    pub const fn bit_depth_luma_minus8(&self) -> u8 {
        self.bit_depth_luma_byte & 0x07
    }

    pub const fn bit_depth_chroma_minus8(&self) -> u8 {
        self.bit_depth_chroma_byte & 0x07
    }

    pub const fn constant_frame_rate(&self) -> u8 {
        (self.frame_rate_byte >> 6) & 0x03
    }

    pub const fn num_temporal_layers(&self) -> u8 {
        (self.frame_rate_byte >> 3) & 0x07
    }

    pub const fn temporal_id_nested(&self) -> bool {
        (self.frame_rate_byte & 0x04) != 0
    }

    pub const fn length_size_minus_one(&self) -> u8 {
        self.frame_rate_byte & 0x03
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NalUnitKind {
    // VCL NAL unit types (0-31)
    TrailN = 0,
    TrailR = 1,
    TsaN = 2,
    TsaR = 3,
    StsaN = 4,
    StsaR = 5,
    RadlN = 6,
    RadlR = 7,
    RaslN = 8,
    RaslR = 9,
    BlaWLp = 16,
    BlaWRadl = 17,
    BlaNLp = 18,
    IdrWRadl = 19,
    IdrNLp = 20,
    CraNut = 21,

    // Non-VCL NAL unit types (32+)
    VPS = 32,
    SPS = 33,
    PPS = 34,
    AUD = 35,
    EOS = 36,
    EOB = 37,
    FD = 38,
    PrefixSEI = 39,
    SuffixSEI = 40,

    // Reserved and unspecified
    Reserved(u8),
    Unspecified(u8),
}

impl From<u8> for NalUnitKind {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::TrailN,
            1 => Self::TrailR,
            2 => Self::TsaN,
            3 => Self::TsaR,
            4 => Self::StsaN,
            5 => Self::StsaR,
            6 => Self::RadlN,
            7 => Self::RadlR,
            8 => Self::RaslN,
            9 => Self::RaslR,
            16 => Self::BlaWLp,
            17 => Self::BlaWRadl,
            18 => Self::BlaNLp,
            19 => Self::IdrWRadl,
            20 => Self::IdrNLp,
            21 => Self::CraNut,
            32 => Self::VPS,
            33 => Self::SPS,
            34 => Self::PPS,
            35 => Self::AUD,
            36 => Self::EOS,
            37 => Self::EOB,
            38 => Self::FD,
            39 => Self::PrefixSEI,
            40 => Self::SuffixSEI,
            10 | 11 | 12 | 13 | 14 | 15 | 22 | 23 | 24..=31 | 41..=47 => Self::Reserved(value),
            48..=63 => Self::Unspecified(value),
            _ => unreachable!("NAL unit type is 6 bits, max value is 63"),
        }
    }
}

impl From<NalUnitKind> for u8 {
    fn from(value: NalUnitKind) -> Self {
        match value {
            NalUnitKind::TrailN => 0,
            NalUnitKind::TrailR => 1,
            NalUnitKind::TsaN => 2,
            NalUnitKind::TsaR => 3,
            NalUnitKind::StsaN => 4,
            NalUnitKind::StsaR => 5,
            NalUnitKind::RadlN => 6,
            NalUnitKind::RadlR => 7,
            NalUnitKind::RaslN => 8,
            NalUnitKind::RaslR => 9,
            NalUnitKind::BlaWLp => 16,
            NalUnitKind::BlaWRadl => 17,
            NalUnitKind::BlaNLp => 18,
            NalUnitKind::IdrWRadl => 19,
            NalUnitKind::IdrNLp => 20,
            NalUnitKind::CraNut => 21,
            NalUnitKind::VPS => 32,
            NalUnitKind::SPS => 33,
            NalUnitKind::PPS => 34,
            NalUnitKind::AUD => 35,
            NalUnitKind::EOS => 36,
            NalUnitKind::EOB => 37,
            NalUnitKind::FD => 38,
            NalUnitKind::PrefixSEI => 39,
            NalUnitKind::SuffixSEI => 40,
            NalUnitKind::Reserved(v) | NalUnitKind::Unspecified(v) => v,
        }
    }
}

#[derive(Debug)]
pub struct NalArray {
    pub(crate) type_byte: u8,
    pub nal_units: Box<[RawNalUnit]>,
}

impl NalArray {
    pub const fn array_completeness(&self) -> bool {
        (self.type_byte & 0x80) != 0
    }

    pub fn nal_unit_type(&self) -> NalUnitKind {
        NalUnitKind::from(self.type_byte & 0x3F)
    }
}

#[derive(Debug)]
pub struct RawNalUnit {
    pub data: Box<[u8]>,
}

#[derive(Debug, Clone, Copy)]
pub struct NalUnitHeader(pub u16);

impl NalUnitHeader {
    pub const fn forbidden_zero_bit(&self) -> bool {
        (self.0 & 0x8000) != 0
    }

    pub fn nal_unit_type(&self) -> NalUnitKind {
        let type_bits = ((self.0 >> 9) & 0x3F) as u8;
        NalUnitKind::from(type_bits)
    }

    pub const fn nuh_layer_id(&self) -> u8 {
        ((self.0 >> 3) & 0x3F) as u8
    }

    pub const fn nuh_temporal_id_plus1(&self) -> u8 {
        (self.0 & 0x07) as u8
    }
}

#[derive(Debug)]
pub struct VideoParameterSet {
    pub vps_video_parameter_set_id: u8,
    pub vps_base_layer_internal_flag: bool,
    pub vps_base_layer_available_flag: bool,
    pub vps_max_layers_minus1: u8,
    pub vps_max_sub_layers_minus1: u8,
    pub vps_temporal_id_nesting_flag: bool,
    pub vps_max_layer_id: u8,
    pub vps_num_layer_sets_minus1: u32,
    pub vps_timing_info_present_flag: bool,
    // optional timing info
    pub vps_num_units_in_tick: Option<u32>,
    pub vps_time_scale: Option<u32>,
}

#[derive(Debug)]
pub struct SequenceParameterSet {
    pub sps_video_parameter_set_id: u8,
    pub sps_max_sub_layers_minus1: u8,
    pub sps_temporal_id_nesting_flag: bool,
    pub sps_seq_parameter_set_id: u32,
    pub chroma_format: ChromaFormat,
    pub separate_color_plane_flag: bool,
    pub pic_width_in_luma_samples: u32,
    pub pic_height_in_luma_samples: u32,
    pub conformance_window_flag: bool,
    pub conf_win_left_offset: u32,
    pub conf_win_right_offset: u32,
    pub conf_win_top_offset: u32,
    pub conf_win_bottom_offset: u32,
    pub bit_depth_luma_minus8: u32,
    pub bit_depth_chroma_minus8: u32,
    pub log2_max_pic_order_cnt_lsb_minus4: u32,
    pub log2_min_luma_coding_block_size_minus3: u32,
    pub log2_diff_max_min_luma_coding_block_size: u32,
    pub log2_min_luma_transform_block_size_minus2: u32,
    pub log2_diff_max_min_luma_transform_block_size: u32,
    pub max_transform_hierarchy_depth_inter: u32,
    pub max_transform_hierarchy_depth_intra: u32,
    pub scaling_list_enabled_flag: bool,
    pub amp_enabled_flag: bool,
    pub sample_adaptive_offset_enabled_flag: bool,
    pub pcm_enabled_flag: bool,
    pub pcm_sample_bit_depth_luma_minus1: Option<u8>,
    pub pcm_sample_bit_depth_chroma_minus1: Option<u8>,
    pub log2_min_pcm_luma_coding_block_size_minus3: Option<u32>,
    pub log2_diff_max_min_pcm_luma_coding_block_size: Option<u32>,
    pub pcm_loop_filter_disabled_flag: Option<bool>,
    pub num_short_term_ref_pic_sets: u32,
    pub long_term_ref_pics_present_flag: bool,
    pub sps_temporal_mvp_enabled_flag: bool,
    pub strong_intra_smoothing_enabled_flag: bool,
    pub vui_parameters_present_flag: bool,

    // Color space (from VUI)
    pub color_primaries: Option<ColorPrimaries>,
    pub transfer_characteristics: Option<TransferCharacteristics>,
    pub matrix_coeffs: Option<MatrixCoefficients>,
}

#[derive(Debug)]
pub struct PictureParameterSet {
    pub pps_pic_parameter_set_id: u32,
    pub pps_seq_parameter_set_id: u32,
    pub dependent_slice_segments_enabled_flag: bool,
    pub output_flag_present_flag: bool,
    pub num_extra_slice_header_bits: u8,
    pub sign_data_hiding_enabled_flag: bool,
    pub cabac_init_present_flag: bool,
    pub num_ref_idx_l0_default_active_minus1: u32,
    pub num_ref_idx_l1_default_active_minus1: u32,
    pub init_qp_minus26: i32,
    pub constrained_intra_pred_flag: bool,
    pub transform_skip_enabled_flag: bool,
    pub cu_qp_delta_enabled_flag: bool,
    pub diff_cu_qp_delta_depth: Option<u32>,
    pub pps_cb_qp_offset: i32,
    pub pps_cr_qp_offset: i32,
    pub pps_slice_chroma_qp_offsets_present_flag: bool,
    pub weighted_pred_flag: bool,
    pub weighted_bipred_flag: bool,
    pub transquant_bypass_enabled_flag: bool,
    pub tiles_enabled_flag: bool,
    pub entropy_coding_sync_enabled_flag: bool,
    pub num_tile_columns_minus1: Option<u32>,
    pub num_tile_rows_minus1: Option<u32>,
    pub uniform_spacing_flag: Option<bool>,
    pub loop_filter_across_tiles_enabled_flag: Option<bool>,
    pub pps_loop_filter_across_slices_enabled_flag: bool,
    pub deblocking_filter_control_present_flag: bool,
    pub deblocking_filter_override_enabled_flag: Option<bool>,
    pub pps_deblocking_filter_disabled_flag: Option<bool>,
    pub pps_beta_offset_div2: Option<i32>,
    pub pps_tc_offset_div2: Option<i32>,
    pub pps_scaling_list_data_present_flag: bool,
    pub lists_modification_present_flag: bool,
    pub log2_parallel_merge_level_minus2: u32,
    pub slice_segment_header_extension_present_flag: bool,
}

#[derive(Debug)]
pub struct SliceSegmentHeader {
    pub first_slice_segment_in_pic_flag: bool,
    pub no_output_of_prior_pics_flag: Option<bool>,
    pub slice_pic_parameter_set_id: u32,
    pub slice_segment_address: Option<u32>,
    pub slice_type: SliceKind,
    pub pic_output_flag: Option<bool>,
    pub colour_plane_id: Option<u8>,
    pub slice_pic_order_cnt_lsb: Option<u32>,
    pub slice_sao_luma_flag: Option<bool>,
    pub slice_sao_chroma_flag: Option<bool>,
    pub slice_qp_delta: i32,
    pub slice_cb_qp_offset: Option<i32>,
    pub slice_cr_qp_offset: Option<i32>,
    pub deblocking_filter_override_flag: Option<bool>,
    pub slice_deblocking_filter_disabled_flag: Option<bool>,
    pub slice_beta_offset_div2: Option<i32>,
    pub slice_tc_offset_div2: Option<i32>,
    pub slice_loop_filter_across_slices_enabled_flag: Option<bool>,
    pub num_entry_point_offsets: u32,
    pub entry_point_offsets: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceKind {
    B = 0,
    P = 1,
    I = 2,
}

impl TryFrom<u32> for SliceKind {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::B),
            1 => Ok(Self::P),
            2 => Ok(Self::I),
            other => Err(anyhow::anyhow!("invalid slice_type: {}", other)),
        }
    }
}
