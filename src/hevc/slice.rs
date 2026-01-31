use crate::{
    cabac::CabacDecoder,
    hevc::{
        NalUnitHeader, NalUnitKind, PictureParameterSet, RbspReader, SequenceParameterSet,
        SliceKind, SliceSegmentHeader,
    },
};
use anyhow::Result;

pub struct SliceSegmentReader<'a> {
    cabac_decoder: CabacDecoder<'a>,
    nal_header: NalUnitHeader,
    slice_header: SliceSegmentHeader,
    sps: &'a SequenceParameterSet,
    pps: &'a PictureParameterSet,
}

impl<'a> SliceSegmentReader<'a> {
    pub fn try_new(
        rbsp: &'a [u8],
        nal_header: NalUnitHeader,
        sps: &'a SequenceParameterSet,
        pps: &'a PictureParameterSet,
    ) -> Result<Self> {
        let mut rbsp_reader = RbspReader::new(rbsp);

        let slice_header = Self::read_header(&mut rbsp_reader, nal_header, sps, pps)?;

        let cabac_decoder = CabacDecoder::try_new(
            rbsp_reader,
            pps.init_qp_minus26,
            slice_header.slice_qp_delta,
        )?;

        Ok(Self {
            cabac_decoder,
            nal_header,
            slice_header,
            sps,
            pps,
        })
    }

    fn read_header(
        reader: &mut RbspReader,
        nal_header: NalUnitHeader,
        sps: &SequenceParameterSet,
        pps: &PictureParameterSet,
    ) -> Result<SliceSegmentHeader> {
        let first_slice_segment_in_pic_flag = reader.read_flag()?;
        let nal_unit_type = nal_header.nal_unit_type();
        let no_output_of_prior_pics_flag = if is_irap_nal_unit_type(nal_unit_type) {
            let flag = reader.read_flag()?;
            Some(flag)
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

        let slice_type_ue = reader.read_ue()?;

        let slice_kind = SliceKind::try_from(slice_type_ue)?;

        let pic_output_flag = if pps.output_flag_present_flag {
            Some(reader.read_flag()?)
        } else {
            None
        };

        let color_plane_id = if sps.separate_color_plane_flag {
            Some(reader.read_u8(2)?)
        } else {
            None
        };

        let slice_pic_order_cnt_lsb = None;

        let (slice_sao_luma_flag, slice_sao_chroma_flag) =
            if sps.sample_adaptive_offset_enabled_flag {
                let luma = reader.read_flag()?;

                let chroma_array_type = if sps.separate_color_plane_flag {
                    0
                } else {
                    sps.chroma_format as u8
                };
                let chroma = if chroma_array_type != 0 {
                    let c = reader.read_flag()?;

                    c
                } else {
                    false
                };
                (luma, chroma)
            } else {
                (false, false)
            };

        if !matches!(slice_kind, SliceKind::I) {
            unimplemented!("P/B slice headers not yet implemented");
        }

        let slice_qp_delta = reader.read_se()?;

        let (slice_cb_qp_offset, slice_cr_qp_offset) =
            if pps.pps_slice_chroma_qp_offsets_present_flag {
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
            && (slice_sao_luma_flag
                || slice_sao_chroma_flag
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
                        let offset = reader.read_u32(offset_len_minus1 as usize + 1)?;
                        offsets.push(offset);
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

        reader.byte_alignment()?;

        Ok(SliceSegmentHeader {
            first_slice_segment_in_pic_flag,
            no_output_of_prior_pics_flag,
            slice_pic_parameter_set_id,
            slice_segment_address: None,
            slice_type: slice_kind,
            pic_output_flag,
            color_plane_id,
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

    pub fn read_data(&mut self) -> Result<()> {
        // since we have no HEVC tiles (each HEIC tile is a separate slice starting at 0),
        // CtbAddrInRs == CtbAddrInTs, so we only need one variable
        let mut ctb_addr = self.slice_header.slice_segment_address.unwrap_or(0);

        loop {
            self.read_coding_tree_unit(ctb_addr)?;

            let end_of_slice_segment_flag = self.cabac_decoder.decode_terminate()?;

            if end_of_slice_segment_flag {
                break;
            }

            ctb_addr += 1;

            if self.pps.entropy_coding_sync_enabled_flag
                && ctb_addr % self.sps.pic_width_in_ctbs_y() == 0
            {
                let _end_of_subset_one_bit = self.cabac_decoder.decode_terminate()?;
                self.cabac_decoder.byte_alignment()?;
            }
        }

        Ok(())
    }

    fn read_coding_tree_unit(&mut self, ctb_addr_in_rs: u32) -> Result<()> {
        let pic_width_in_ctbs_y = self.sps.pic_width_in_ctbs_y();
        let ctb_log2_size_y = self.sps.ctb_log2_size_y();

        let rx = ctb_addr_in_rs % pic_width_in_ctbs_y;
        let ry = ctb_addr_in_rs / pic_width_in_ctbs_y;

        if self.slice_header.slice_sao_luma_flag || self.slice_header.slice_sao_chroma_flag {
            let _ = self.sao(rx, ry)?;
        }

        self.coding_quadtree(rx << ctb_log2_size_y, ry << ctb_log2_size_y, 0)?;

        Ok(())
    }

    fn sao(&mut self, rx: u32, ry: u32) -> Result<()> {
        todo!()
    }

    fn coding_quadtree(&mut self, x0: u32, y0: u32, cqt_depth: usize) -> Result<()> {
        todo!()
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
