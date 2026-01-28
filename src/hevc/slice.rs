use crate::hevc::{
    NalUnitHeader, NalUnitKind, PictureParameterSet, RbspReader, SequenceParameterSet, SliceKind,
    SliceSegmentHeader,
};
use anyhow::Result;

pub struct SliceSegmentReader<'a> {
    reader: RbspReader<'a>,
    nal_header: NalUnitHeader,
    sps: &'a SequenceParameterSet,
    pps: &'a PictureParameterSet,
}

impl<'a> SliceSegmentReader<'a> {
    pub fn new(
        rbsp: &'a [u8],
        nal_header: NalUnitHeader,
        sps: &'a SequenceParameterSet,
        pps: &'a PictureParameterSet,
    ) -> Self {
        Self {
            reader: RbspReader::new(rbsp),
            nal_header,
            sps,
            pps,
        }
    }

    pub fn read(&mut self) -> Result<()> {
        let _header = self.read_slice_header()?;

        Ok(())
    }

    fn read_slice_header(&mut self) -> Result<SliceSegmentHeader> {
        let first_slice_segment_in_pic_flag = self.reader.read_flag()?;
        let nal_unit_type = self.nal_header.nal_unit_type();
        let no_output_of_prior_pics_flag = if is_irap_nal_unit_type(nal_unit_type) {
            let flag = self.reader.read_flag()?;
            Some(flag)
        } else {
            None
        };

        let slice_pic_parameter_set_id = self.reader.read_ue()?;
        assert!(
            first_slice_segment_in_pic_flag,
            "first slice segement in pic flag should always be true"
        );

        for _ in 0..self.pps.num_extra_slice_header_bits {
            self.reader.read_flag()?;
        }

        let slice_type_ue = self.reader.read_ue()?;

        let slice_kind = SliceKind::try_from(slice_type_ue)?;

        let pic_output_flag = self
            .pps
            .output_flag_present_flag
            .then_some(self.reader.read_flag())
            .transpose()?;

        let colour_plane_id = self
            .sps
            .separate_color_plane_flag
            .then_some(self.reader.read_u8(2))
            .transpose()?;

        let slice_pic_order_cnt_lsb = None;

        let (slice_sao_luma_flag, slice_sao_chroma_flag) =
            if self.sps.sample_adaptive_offset_enabled_flag {
                let luma = self.reader.read_flag()?;

                let chroma_array_type = if self.sps.separate_color_plane_flag {
                    0
                } else {
                    self.sps.chroma_format as u8
                };
                let chroma = if chroma_array_type != 0 {
                    let c = self.reader.read_flag()?;

                    Some(c)
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

        let slice_qp_delta = self.reader.read_se()?;

        let (slice_cb_qp_offset, slice_cr_qp_offset) =
            if self.pps.pps_slice_chroma_qp_offsets_present_flag {
                (Some(self.reader.read_se()?), Some(self.reader.read_se()?))
            } else {
                (None, None)
            };

        // todo: handle cu_chroma_qp_offset_enabled_flag when chroma_qp_offset_list_enabled_flag is supported

        let deblocking_filter_override_flag = if self
            .pps
            .deblocking_filter_override_enabled_flag
            .unwrap_or(false)
        {
            Some(self.reader.read_flag()?)
        } else {
            None
        };

        let (slice_deblocking_filter_disabled_flag, slice_beta_offset_div2, slice_tc_offset_div2) =
            if deblocking_filter_override_flag.unwrap_or(false) {
                let disabled = self.reader.read_flag()?;
                if !disabled {
                    (
                        Some(disabled),
                        Some(self.reader.read_se()?),
                        Some(self.reader.read_se()?),
                    )
                } else {
                    (Some(disabled), None, None)
                }
            } else {
                (self.pps.pps_deblocking_filter_disabled_flag, None, None)
            };

        let slice_loop_filter_across_slices_enabled_flag =
            if self.pps.pps_loop_filter_across_slices_enabled_flag
                && (slice_sao_luma_flag.unwrap_or(false)
                    || slice_sao_chroma_flag.unwrap_or(false)
                    || !slice_deblocking_filter_disabled_flag.unwrap_or(false))
            {
                Some(self.reader.read_flag()?)
            } else {
                None
            };

        let (num_entry_point_offsets, entry_point_offsets) =
            if self.pps.tiles_enabled_flag || self.pps.entropy_coding_sync_enabled_flag {
                let num_offsets = self.reader.read_ue()?;

                let mut offsets = Vec::new();
                if num_offsets > 0 {
                    let offset_len_minus1 = self.reader.read_ue()?;

                    for i in 0..num_offsets {
                        let offset = self.reader.read_u32(offset_len_minus1 as usize + 1)?;
                        offsets.push(offset);
                    }
                }
                (num_offsets, offsets)
            } else {
                (0, vec![])
            };

        if self.pps.slice_segment_header_extension_present_flag {
            let extension_length = self.reader.read_ue()?;
            for _ in 0..extension_length {
                self.reader.read_u8(8)?;
            }
        }

        // byte_alignment()
        if !self.reader.is_byte_aligned() {
            let _alignment_bit = self.reader.read_flag()?;
            while !self.reader.is_byte_aligned() {
                let _bit = self.reader.read_flag()?;
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
