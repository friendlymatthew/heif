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
    pub arrays: Box<[NALArray]>,
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
    // video parameter set
    VPS = 32,
    // sequence parameter set
    SPS = 33,
    // picture parameter set
    PPS = 34,
    PrefixSEI = 39,
    SuffixSEI = 40,
    Unknown(u8),
}

impl From<u8> for NalUnitKind {
    fn from(value: u8) -> Self {
        match value {
            32 => Self::VPS,
            33 => Self::SPS,
            34 => Self::PPS,
            39 => Self::PrefixSEI,
            40 => Self::SuffixSEI,
            other => Self::Unknown(other),
        }
    }
}

impl From<NalUnitKind> for u8 {
    fn from(value: NalUnitKind) -> Self {
        match value {
            NalUnitKind::VPS => 32,
            NalUnitKind::SPS => 33,
            NalUnitKind::PPS => 34,
            NalUnitKind::PrefixSEI => 39,
            NalUnitKind::SuffixSEI => 40,
            NalUnitKind::Unknown(v) => v,
        }
    }
}

#[derive(Debug)]
pub struct NALArray {
    pub(crate) type_byte: u8,
    pub nal_units: Box<[NALUnit]>,
}

impl NALArray {
    pub const fn array_completeness(&self) -> bool {
        (self.type_byte & 0x80) != 0
    }

    pub fn nal_unit_type(&self) -> NalUnitKind {
        NalUnitKind::from(self.type_byte & 0x3F)
    }
}

#[derive(Debug)]
pub struct NALUnit {
    pub data: Box<[u8]>,
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
