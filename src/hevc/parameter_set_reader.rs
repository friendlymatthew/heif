use crate::hevc::{
    ChromaFormat, ColorPrimaries, MatrixCoefficients, PictureParameterSet, RbspReader,
    SequenceParameterSet, TransferCharacteristics, VideoParameterSet,
};
use anyhow::{Result, ensure};

pub fn video_parameter_set_rbsp(data: &[u8]) -> Result<VideoParameterSet> {
    let mut reader = RbspReader::new(data);

    let vps_video_parameter_set_id = reader.read_u8(4)?;
    let vps_base_layer_internal_flag = reader.read_flag()?;
    let vps_base_layer_available_flag = reader.read_flag()?;
    let vps_max_layers_minus1 = reader.read_u8(6)?;
    let vps_max_sub_layers_minus1 = reader.read_u8(3)?;
    let vps_temporal_id_nesting_flag = reader.read_flag()?;
    let _reserved = reader.read_u32(16)?;

    // skip profile_tier_level, because it's a lot of faff that heic don't need
    skip_profile_tier_level(&mut reader, true, vps_max_sub_layers_minus1)?;

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

pub fn sequence_parameter_set_rbsp(data: &[u8]) -> Result<SequenceParameterSet> {
    let mut reader = RbspReader::new(data);

    let sps_video_parameter_set_id = reader.read_u8(4)?;
    let sps_max_sub_layers_minus1 = reader.read_u8(3)?;
    let sps_temporal_id_nesting_flag = reader.read_flag()?;

    // skip profile tier since it's a proper faff
    skip_profile_tier_level(&mut reader, true, sps_max_sub_layers_minus1)?;

    let sps_seq_parameter_set_id = reader.read_ue()?;
    let chroma_format_idc = reader.read_ue()?;
    let chroma_format = ChromaFormat::try_from(chroma_format_idc)?;

    let separate_color_plane_flag = (chroma_format_idc == 3)
        .then(|| reader.read_flag())
        .transpose()?
        .unwrap_or_default();

    let pic_width_in_luma_samples = reader.read_ue()?;
    let pic_height_in_luma_samples = reader.read_ue()?;

    let conformance_window_flag = reader.read_flag()?;

    let (conf_win_left_offset, conf_win_right_offset, conf_win_top_offset, conf_win_bottom_offset) =
        if conformance_window_flag {
            (
                reader.read_ue()?,
                reader.read_ue()?,
                reader.read_ue()?,
                reader.read_ue()?,
            )
        } else {
            (0, 0, 0, 0)
        };

    let bit_depth_luma_minus8 = reader.read_ue()?;
    let bit_depth_chroma_minus8 = reader.read_ue()?;

    let log2_max_pic_order_cnt_lsb_minus4 = reader.read_ue()?;

    let sps_sub_layer_ordering_info_present_flag = reader.read_flag()?;
    let start_layer = if sps_sub_layer_ordering_info_present_flag {
        0
    } else {
        sps_max_sub_layers_minus1
    };

    for _ in start_layer..=sps_max_sub_layers_minus1 {
        let _sps_max_dec_pic_buffering_minus1 = reader.read_ue()?;
        let _sps_max_num_reorder_pics = reader.read_ue()?;
        let _sps_max_latency_increase_plus1 = reader.read_ue()?;
    }

    let log2_min_luma_coding_block_size_minus3 = reader.read_ue()?;
    let log2_diff_max_min_luma_coding_block_size = reader.read_ue()?;
    let log2_min_luma_transform_block_size_minus2 = reader.read_ue()?;
    let log2_diff_max_min_luma_transform_block_size = reader.read_ue()?;
    let max_transform_hierarchy_depth_inter = reader.read_ue()?;
    let max_transform_hierarchy_depth_intra = reader.read_ue()?;

    let scaling_list_enabled_flag = reader.read_flag()?;
    if scaling_list_enabled_flag {
        let sps_scaling_list_data_present_flag = reader.read_flag()?;
        if sps_scaling_list_data_present_flag {
            skip_scaling_list_data(&mut reader)?;
        }
    }

    let amp_enabled_flag = reader.read_flag()?;
    let sample_adaptive_offset_enabled_flag = reader.read_flag()?;
    let pcm_enabled_flag = reader.read_flag()?;

    let (
        pcm_sample_bit_depth_luma_minus1,
        pcm_sample_bit_depth_chroma_minus1,
        log2_min_pcm_luma_coding_block_size_minus3,
        log2_diff_max_min_pcm_luma_coding_block_size,
        pcm_loop_filter_disabled_flag,
    ) = if pcm_enabled_flag {
        (
            Some(reader.read_u8(4)?),
            Some(reader.read_u8(4)?),
            Some(reader.read_ue()?),
            Some(reader.read_ue()?),
            Some(reader.read_flag()?),
        )
    } else {
        (None, None, None, None, None)
    };

    let num_short_term_ref_pic_sets = reader.read_ue()?;
    for _ in 0..num_short_term_ref_pic_sets {
        skip_st_ref_pic_set(&mut reader)?;
    }

    let long_term_ref_pics_present_flag = reader.read_flag()?;
    if long_term_ref_pics_present_flag {
        let num_long_term_ref_pics_sps = reader.read_ue()?;
        for _ in 0..num_long_term_ref_pics_sps {
            let _lt_ref_pic_poc_lsb_sps =
                reader.read_bits(log2_max_pic_order_cnt_lsb_minus4 as usize + 4)?;
            let _used_by_curr_pic_lt_sps_flag = reader.read_flag()?;
        }
    }

    let sps_temporal_mvp_enabled_flag = reader.read_flag()?;
    let strong_intra_smoothing_enabled_flag = reader.read_flag()?;

    let vui_parameters_present_flag = reader.read_flag()?;
    let (color_primaries, transfer_characteristics, matrix_coeffs) = if vui_parameters_present_flag
    {
        parse_vui_parameters(&mut reader)?
    } else {
        (None, None, None)
    };

    let sps_extension_present_flag = reader.read_flag()?;

    ensure!(
        !sps_extension_present_flag,
        "todo: parse out the if( sps_extension_present_flag ) {{"
    );

    Ok(SequenceParameterSet {
        sps_video_parameter_set_id,
        sps_max_sub_layers_minus1,
        sps_temporal_id_nesting_flag,
        sps_seq_parameter_set_id,
        chroma_format,
        separate_color_plane_flag,
        pic_width_in_luma_samples,
        pic_height_in_luma_samples,
        conformance_window_flag,
        conf_win_left_offset,
        conf_win_right_offset,
        conf_win_top_offset,
        conf_win_bottom_offset,
        bit_depth_luma_minus8,
        bit_depth_chroma_minus8,
        log2_max_pic_order_cnt_lsb_minus4,
        log2_min_luma_coding_block_size_minus3,
        log2_diff_max_min_luma_coding_block_size,
        log2_min_luma_transform_block_size_minus2,
        log2_diff_max_min_luma_transform_block_size,
        max_transform_hierarchy_depth_inter,
        max_transform_hierarchy_depth_intra,
        scaling_list_enabled_flag,
        amp_enabled_flag,
        sample_adaptive_offset_enabled_flag,
        pcm_enabled_flag,
        pcm_sample_bit_depth_luma_minus1,
        pcm_sample_bit_depth_chroma_minus1,
        log2_min_pcm_luma_coding_block_size_minus3,
        log2_diff_max_min_pcm_luma_coding_block_size,
        pcm_loop_filter_disabled_flag,
        num_short_term_ref_pic_sets,
        long_term_ref_pics_present_flag,
        sps_temporal_mvp_enabled_flag,
        strong_intra_smoothing_enabled_flag,
        vui_parameters_present_flag,
        color_primaries,
        transfer_characteristics,
        matrix_coeffs,
    })
}

fn skip_scaling_list_data(reader: &mut RbspReader) -> Result<()> {
    for size_id in 0..4 {
        let num_matrices = if size_id == 3 { 2 } else { 6 };
        for _ in 0..num_matrices {
            let scaling_list_pred_mode_flag = reader.read_flag()?;
            if !scaling_list_pred_mode_flag {
                let _scaling_list_pred_matrix_id_delta = reader.read_ue()?;
            } else {
                let coef_num = (1 << (4 + (size_id << 1))).min(64);
                if size_id > 1 {
                    let _scaling_list_dc_coef_minus8 = reader.read_se()?;
                }
                for _ in 0..coef_num {
                    let _scaling_list_delta_coef = reader.read_se()?;
                }
            }
        }
    }
    Ok(())
}

// this is complex faff and for heif we don't need it since images are intra-only
fn skip_st_ref_pic_set(reader: &mut RbspReader) -> Result<()> {
    let inter_ref_pic_set_prediction_flag = reader.read_flag()?;

    if inter_ref_pic_set_prediction_flag {
        let _delta_idx_minus1 = reader.read_ue()?;
        let _delta_rps_sign = reader.read_flag()?;
        let _abs_delta_rps_minus1 = reader.read_ue()?;
        // would need to parse used_by_curr_pic_flag and use_delta_flag based on previous rps
        // for now just bail, heif shouldn't use inter-prediction
    } else {
        let num_negative_pics = reader.read_ue()?;
        let num_positive_pics = reader.read_ue()?;

        for _ in 0..num_negative_pics {
            let _delta_poc_s0_minus1 = reader.read_ue()?;
            let _used_by_curr_pic_s0_flag = reader.read_flag()?;
        }

        for _ in 0..num_positive_pics {
            let _delta_poc_s1_minus1 = reader.read_ue()?;
            let _used_by_curr_pic_s1_flag = reader.read_flag()?;
        }
    }

    Ok(())
}

fn parse_vui_parameters(
    reader: &mut RbspReader,
) -> Result<(
    Option<ColorPrimaries>,
    Option<TransferCharacteristics>,
    Option<MatrixCoefficients>,
)> {
    let aspect_ratio_info_present_flag = reader.read_flag()?;
    if aspect_ratio_info_present_flag {
        let aspect_ratio_idc = reader.read_u8(8)?;
        if aspect_ratio_idc == 255 {
            // extended_sar
            let _sar_width = reader.read_u32(16)?;
            let _sar_height = reader.read_u32(16)?;
        }
    }

    let overscan_info_present_flag = reader.read_flag()?;
    if overscan_info_present_flag {
        let _overscan_appropriate_flag = reader.read_flag()?;
    }

    let mut color_primaries = None;
    let mut transfer_characteristics = None;
    let mut matrix_coeffs = None;

    let video_signal_type_present_flag = reader.read_flag()?;
    if video_signal_type_present_flag {
        let _video_format = reader.read_u8(3)?;
        let _video_full_range_flag = reader.read_flag()?;
        let color_description_present_flag = reader.read_flag()?;
        if color_description_present_flag {
            color_primaries = Some(ColorPrimaries::from(reader.read_u8(8)?));
            transfer_characteristics = Some(TransferCharacteristics::from(reader.read_u8(8)?));
            matrix_coeffs = Some(MatrixCoefficients::from(reader.read_u8(8)?));
        }
    }

    let chroma_loc_info_present_flag = reader.read_flag()?;
    if chroma_loc_info_present_flag {
        let _chroma_sample_loc_type_top_field = reader.read_ue()?;
        let _chroma_sample_loc_type_bottom_field = reader.read_ue()?;
    }

    let _neutral_chroma_indication_flag = reader.read_flag()?;
    let _field_seq_flag = reader.read_flag()?;
    let _frame_field_info_present_flag = reader.read_flag()?;

    let default_display_window_flag = reader.read_flag()?;
    if default_display_window_flag {
        let _def_disp_win_left_offset = reader.read_ue()?;
        let _def_disp_win_right_offset = reader.read_ue()?;
        let _def_disp_win_top_offset = reader.read_ue()?;
        let _def_disp_win_bottom_offset = reader.read_ue()?;
    }

    let vui_timing_info_present_flag = reader.read_flag()?;
    if vui_timing_info_present_flag {
        let _vui_num_units_in_tick = reader.read_u32(32)?;
        let _vui_time_scale = reader.read_u32(32)?;
        let vui_poc_proportional_to_timing_flag = reader.read_flag()?;
        if vui_poc_proportional_to_timing_flag {
            let _vui_num_ticks_poc_diff_one_minus1 = reader.read_ue()?;
        }
        let vui_hrd_parameters_present_flag = reader.read_flag()?;
        if vui_hrd_parameters_present_flag {
            // skip HRD parameters (very complex)
            skip_hrd_parameters(reader)?;
        }
    }

    let bitstream_restriction_flag = reader.read_flag()?;
    if bitstream_restriction_flag {
        let _tiles_fixed_structure_flag = reader.read_flag()?;
        let _motion_vectors_over_pic_boundaries_flag = reader.read_flag()?;
        let _restricted_ref_pic_lists_flag = reader.read_flag()?;
        let _min_spatial_segmentation_idc = reader.read_ue()?;
        let _max_bytes_per_pic_denom = reader.read_ue()?;
        let _max_bits_per_min_cu_denom = reader.read_ue()?;
        let _log2_max_mv_length_horizontal = reader.read_ue()?;
        let _log2_max_mv_length_vertical = reader.read_ue()?;
    }

    Ok((color_primaries, transfer_characteristics, matrix_coeffs))
}

fn skip_hrd_parameters(reader: &mut RbspReader) -> Result<()> {
    let nal_hrd_parameters_present_flag = reader.read_flag()?;
    let vcl_hrd_parameters_present_flag = reader.read_flag()?;

    if nal_hrd_parameters_present_flag || vcl_hrd_parameters_present_flag {
        let _sub_pic_hrd_params_present_flag = reader.read_flag()?;
        // crazy stuff here, skip for now
        // note: does heif files have detailed HRD params?
    }

    Ok(())
}

pub fn picture_parameter_set_rbsp(data: &[u8]) -> Result<PictureParameterSet> {
    let mut reader = RbspReader::new(data);

    let pps_pic_parameter_set_id = reader.read_ue()?;
    let pps_seq_parameter_set_id = reader.read_ue()?;
    let dependent_slice_segments_enabled_flag = reader.read_flag()?;
    let output_flag_present_flag = reader.read_flag()?;
    let num_extra_slice_header_bits = reader.read_u8(3)?;
    let sign_data_hiding_enabled_flag = reader.read_flag()?;
    let cabac_init_present_flag = reader.read_flag()?;
    let num_ref_idx_l0_default_active_minus1 = reader.read_ue()?;
    let num_ref_idx_l1_default_active_minus1 = reader.read_ue()?;
    let init_qp_minus26 = reader.read_se()?;
    let constrained_intra_pred_flag = reader.read_flag()?;
    let transform_skip_enabled_flag = reader.read_flag()?;
    let cu_qp_delta_enabled_flag = reader.read_flag()?;

    let diff_cu_qp_delta_depth = if cu_qp_delta_enabled_flag {
        Some(reader.read_ue()?)
    } else {
        None
    };

    let pps_cb_qp_offset = reader.read_se()?;
    let pps_cr_qp_offset = reader.read_se()?;
    let pps_slice_chroma_qp_offsets_present_flag = reader.read_flag()?;
    let weighted_pred_flag = reader.read_flag()?;
    let weighted_bipred_flag = reader.read_flag()?;
    let transquant_bypass_enabled_flag = reader.read_flag()?;
    let tiles_enabled_flag = reader.read_flag()?;
    let entropy_coding_sync_enabled_flag = reader.read_flag()?;

    let (
        num_tile_columns_minus1,
        num_tile_rows_minus1,
        uniform_spacing_flag,
        loop_filter_across_tiles_enabled_flag,
    ) = if tiles_enabled_flag {
        let num_tile_columns_minus1 = reader.read_ue()?;
        let num_tile_rows_minus1 = reader.read_ue()?;
        let uniform_spacing_flag = reader.read_flag()?;

        if !uniform_spacing_flag {
            for _ in 0..num_tile_columns_minus1 {
                let _column_width_minus1 = reader.read_ue()?;
            }
            for _ in 0..num_tile_rows_minus1 {
                let _row_height_minus1 = reader.read_ue()?;
            }
        }

        let loop_filter_across_tiles_enabled_flag = reader.read_flag()?;

        (
            Some(num_tile_columns_minus1),
            Some(num_tile_rows_minus1),
            Some(uniform_spacing_flag),
            Some(loop_filter_across_tiles_enabled_flag),
        )
    } else {
        (None, None, None, None)
    };

    let pps_loop_filter_across_slices_enabled_flag = reader.read_flag()?;
    let deblocking_filter_control_present_flag = reader.read_flag()?;

    let (
        deblocking_filter_override_enabled_flag,
        pps_deblocking_filter_disabled_flag,
        pps_beta_offset_div2,
        pps_tc_offset_div2,
    ) = if deblocking_filter_control_present_flag {
        let deblocking_filter_override_enabled_flag = reader.read_flag()?;
        let pps_deblocking_filter_disabled_flag = reader.read_flag()?;

        let (beta, tc) = if !pps_deblocking_filter_disabled_flag {
            (Some(reader.read_se()?), Some(reader.read_se()?))
        } else {
            (None, None)
        };

        (
            Some(deblocking_filter_override_enabled_flag),
            Some(pps_deblocking_filter_disabled_flag),
            beta,
            tc,
        )
    } else {
        (None, None, None, None)
    };

    let pps_scaling_list_data_present_flag = reader.read_flag()?;
    if pps_scaling_list_data_present_flag {
        skip_scaling_list_data(&mut reader)?;
    }

    let lists_modification_present_flag = reader.read_flag()?;
    let log2_parallel_merge_level_minus2 = reader.read_ue()?;
    let slice_segment_header_extension_present_flag = reader.read_flag()?;

    let _pps_extension_present_flag = reader.read_flag()?;

    Ok(PictureParameterSet {
        pps_pic_parameter_set_id,
        pps_seq_parameter_set_id,
        dependent_slice_segments_enabled_flag,
        output_flag_present_flag,
        num_extra_slice_header_bits,
        sign_data_hiding_enabled_flag,
        cabac_init_present_flag,
        num_ref_idx_l0_default_active_minus1,
        num_ref_idx_l1_default_active_minus1,
        init_qp_minus26,
        constrained_intra_pred_flag,
        transform_skip_enabled_flag,
        cu_qp_delta_enabled_flag,
        diff_cu_qp_delta_depth,
        pps_cb_qp_offset,
        pps_cr_qp_offset,
        pps_slice_chroma_qp_offsets_present_flag,
        weighted_pred_flag,
        weighted_bipred_flag,
        transquant_bypass_enabled_flag,
        tiles_enabled_flag,
        entropy_coding_sync_enabled_flag,
        num_tile_columns_minus1,
        num_tile_rows_minus1,
        uniform_spacing_flag,
        loop_filter_across_tiles_enabled_flag,
        pps_loop_filter_across_slices_enabled_flag,
        deblocking_filter_control_present_flag,
        deblocking_filter_override_enabled_flag,
        pps_deblocking_filter_disabled_flag,
        pps_beta_offset_div2,
        pps_tc_offset_div2,
        pps_scaling_list_data_present_flag,
        lists_modification_present_flag,
        log2_parallel_merge_level_minus2,
        slice_segment_header_extension_present_flag,
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

    let mut sub_layer_profile_present = vec![false; max_sub_layers_minus1 as usize];
    let mut sub_layer_level_present = vec![false; max_sub_layers_minus1 as usize];

    for i in 0..(max_sub_layers_minus1 as usize) {
        sub_layer_profile_present[i] = reader.read_flag()?;
        sub_layer_level_present[i] = reader.read_flag()?;
    }

    if max_sub_layers_minus1 > 0 {
        for _ in max_sub_layers_minus1..8 {
            let _reserved = reader.read_u8(2)?;
        }
    }

    for i in 0..(max_sub_layers_minus1 as usize) {
        if sub_layer_profile_present[i] {
            let _sub_layer_profile_space = reader.read_u8(2)?;
            let _sub_layer_tier_flag = reader.read_flag()?;
            let _sub_layer_profile_idc = reader.read_u8(5)?;

            for _ in 0..32 {
                let _flag = reader.read_flag()?;
            }

            for _ in 0..48 {
                let _flag = reader.read_flag()?;
            }
        }

        if sub_layer_level_present[i] {
            let _sub_layer_level_idc = reader.read_u8(8)?;
        }
    }

    Ok(())
}
