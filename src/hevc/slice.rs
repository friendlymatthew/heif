use crate::hevc::{
    NalUnitHeader, NalUnitKind, PictureParameterSet, RbspReader, SequenceParameterSet, SliceKind,
    SliceSegmentHeader, VideoParameterSet,
};
use anyhow::{Result, ensure};

pub fn slice_segment_layer_rbsp(
    reader: &mut RbspReader,
    nal_header: NalUnitHeader,
    vps: &VideoParameterSet,
    sps: &SequenceParameterSet,
    pps: &PictureParameterSet,
) -> Result<SliceSegmentHeader> {
    let header = slice_segment_header(reader, nal_header, vps, sps, pps)?;

    Ok(header)
}

fn slice_segment_header(
    reader: &mut RbspReader,
    nal_header: NalUnitHeader,
    _vps: &VideoParameterSet,
    sps: &SequenceParameterSet,
    pps: &PictureParameterSet,
) -> Result<SliceSegmentHeader> {
    let first_slice_segment_in_pic_flag = reader.read_flag()?;

    let nal_unit_type = nal_header.nal_unit_type();
    let no_output_of_prior_pics_flag = if is_irap_nal_unit_type(nal_unit_type) {
        Some(reader.read_flag()?)
    } else {
        None
    };

    let slice_pic_parameter_set_id = reader.read_ue()?;

    assert!(
        first_slice_segment_in_pic_flag,
        "first slice segement in pic flag should always be true"
    );

    for _ in 0..pps.num_extra_slice_header_bits {
        reader.read_flag()?;
    }

    let slice_kind = SliceKind::try_from(reader.read_ue()?)?;

    let pic_output_flag = pps
        .output_flag_present_flag
        .then_some(reader.read_flag())
        .transpose()?;

    let colour_plane_id = sps
        .separate_color_plane_flag
        .then_some(reader.read_u8(2))
        .transpose()?;

    let slice_pic_order_cnt_lsb = None;

    let (slice_sao_luma_flag, slice_sao_chroma_flag) = if sps.sample_adaptive_offset_enabled_flag {
        let luma = reader.read_flag()?;
        let chroma_array_type = if sps.separate_color_plane_flag {
            0
        } else {
            sps.chroma_format as u8
        };
        let chroma = if chroma_array_type != 0 {
            Some(reader.read_flag()?)
        } else {
            None
        };
        (Some(luma), chroma)
    } else {
        (None, None)
    };

    if !matches!(slice_kind, SliceKind::I) {
        unimplemented!("P/B slice headers not yet implemented");
    }

    let slice_qp_delta = reader.read_se()?;

    let (slice_cb_qp_offset, slice_cr_qp_offset) = if pps.pps_slice_chroma_qp_offsets_present_flag {
        (Some(reader.read_se()?), Some(reader.read_se()?))
    } else {
        (None, None)
    };

    // todo: handle cu_chroma_qp_offset_enabled_flag when chroma_qp_offset_list_enabled_flag is supported

    let deblocking_filter_override_flag =
        if pps.deblocking_filter_override_enabled_flag.unwrap_or(false) {
            Some(reader.read_flag()?)
        } else {
            None
        };

    let (slice_deblocking_filter_disabled_flag, slice_beta_offset_div2, slice_tc_offset_div2) =
        if deblocking_filter_override_flag.unwrap_or(false) {
            let disabled = reader.read_flag()?;
            if !disabled {
                (
                    Some(disabled),
                    Some(reader.read_se()?),
                    Some(reader.read_se()?),
                )
            } else {
                (Some(disabled), None, None)
            }
        } else {
            (pps.pps_deblocking_filter_disabled_flag, None, None)
        };

    let slice_loop_filter_across_slices_enabled_flag = if pps
        .pps_loop_filter_across_slices_enabled_flag
        && (slice_sao_luma_flag.unwrap_or(false)
            || slice_sao_chroma_flag.unwrap_or(false)
            || !slice_deblocking_filter_disabled_flag.unwrap_or(false))
    {
        Some(reader.read_flag()?)
    } else {
        None
    };

    let (num_entry_point_offsets, entry_point_offsets) =
        if pps.tiles_enabled_flag || pps.entropy_coding_sync_enabled_flag {
            let num_offsets = reader.read_ue()?;
            let mut offsets = Vec::new();
            if num_offsets > 0 {
                let offset_len_minus1 = reader.read_ue()?;
                for _ in 0..num_offsets {
                    offsets.push(reader.read_u32(offset_len_minus1 as usize + 1)?);
                }
            }
            (num_offsets, offsets)
        } else {
            (0, vec![])
        };

    if pps.slice_segment_header_extension_present_flag {
        let extension_length = reader.read_ue()?;
        for _ in 0..extension_length {
            reader.read_u8(8)?;
        }
    }

    if !reader.is_byte_aligned() {
        let alignment_bit = reader.read_flag()?;
        ensure!(alignment_bit, "alignment_bit_equal_to_one must be 1");

        while !reader.is_byte_aligned() {
            let bit = reader.read_flag()?;
            ensure!(!bit, "alignment_bit_equal_to_zero must be 0");
        }
    }

    Ok(SliceSegmentHeader {
        first_slice_segment_in_pic_flag,
        no_output_of_prior_pics_flag,
        slice_pic_parameter_set_id,
        slice_segment_address: None,
        slice_type: slice_kind,
        pic_output_flag,
        colour_plane_id,
        slice_pic_order_cnt_lsb,
        slice_sao_luma_flag,
        slice_sao_chroma_flag,
        slice_qp_delta,
        slice_cb_qp_offset,
        slice_cr_qp_offset,
        deblocking_filter_override_flag,
        slice_deblocking_filter_disabled_flag,
        slice_beta_offset_div2,
        slice_tc_offset_div2,
        slice_loop_filter_across_slices_enabled_flag,
        num_entry_point_offsets,
        entry_point_offsets,
    })
}

const fn is_irap_nal_unit_type(nal_type: NalUnitKind) -> bool {
    matches!(
        nal_type,
        NalUnitKind::BlaWLp
            | NalUnitKind::BlaWRadl
            | NalUnitKind::BlaNLp
            | NalUnitKind::IdrWRadl
            | NalUnitKind::IdrNLp
            | NalUnitKind::CraNut
            | NalUnitKind::Reserved(22)
            | NalUnitKind::Reserved(23)
    )
}
