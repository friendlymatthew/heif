use crate::hevc::{RbspReader, VideoParameterSet};
use anyhow::{Result, ensure};

pub struct VideoParameterSetReader;

impl VideoParameterSetReader {
    pub fn read(data: &[u8]) -> Result<VideoParameterSet> {
        let mut reader = RbspReader::new(data);

        let vps_video_parameter_set_id = reader.read_u8(4)?;
        let vps_base_layer_internal_flag = reader.read_flag()?;
        let vps_base_layer_available_flag = reader.read_flag()?;
        let vps_max_layers_minus1 = reader.read_u8(6)?;
        let vps_max_sub_layers_minus1 = reader.read_u8(3)?;
        let vps_temporal_id_nesting_flag = reader.read_flag()?;
        let _reserved = reader.read_u32(16)?;

        // skip profile_tier_level(1, vps_max_sub_layers_minus1), it's a lot of faff
        Self::skip_profile_tier_level(&mut reader, true, vps_max_sub_layers_minus1)?;

        Ok(VideoParameterSet {
            vps_video_parameter_set_id,
            vps_base_layer_internal_flag,
            vps_base_layer_available_flag,
            vps_max_layers_minus1,
            vps_max_sub_layers_minus1,
            vps_temporal_id_nesting_flag,
            vps_max_layer_id: 0,
            vps_num_layer_sets_minus1: 0,
            vps_timing_info_present_flag: false,
            vps_num_units_in_tick: None,
            vps_time_scale: None,
        })
    }

    fn skip_profile_tier_level(
        reader: &mut RbspReader,
        profile_present: bool,
        max_sub_layers_minus1: u8,
    ) -> Result<()> {
        if profile_present {
            let _general_profile_space = reader.read_u8(2)?;
            let _general_tier_flag = reader.read_flag()?;
            let _general_profile_idc = reader.read_u8(5)?;

            // skip 32 flags
            for _ in 0..32 {
                let _flag = reader.read_flag()?;
            }

            // skip 48 flags
            for _ in 0..48 {
                let _constraint_flag = reader.read_flag()?;
            }
        }

        let _general_level_idc = reader.read_u8(8)?;

        ensure!(
            max_sub_layers_minus1 == 0,
            "heif images should typically have no sub-layers..."
        );

        Ok(())
    }
}
