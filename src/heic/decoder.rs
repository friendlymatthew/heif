use crate::heif::{HeifReader, ItemInfoEntry, ItemType};
use crate::hevc::{
    NalUnitHeader, NalUnitKind, RbspReader, SliceSegmentReader, picture_parameter_set_rbsp,
    sequence_parameter_set_rbsp, video_parameter_set_rbsp,
};
use anyhow::{Result, anyhow, bail, ensure};

#[derive(Debug)]
pub struct HeicDecoder;

impl HeicDecoder {
    pub fn decode(data: &[u8]) -> Result<()> {
        let mut reader = HeifReader::new(data);
        let heif = reader.read()?;

        let hevc_config = heif
            .hevc_configuration_record()
            .ok_or_else(|| anyhow!("missing HEVC decoder configuration"))?;

        debug_assert_eq!(hevc_config.arrays.len(), 3, "more than 3 param sets found");

        // the order should _typically_ be VPS, SPS, PPS
        // note does heif generally have 1 of each?
        let vps = {
            let b = hevc_config
                .arrays
                .iter()
                .find(|a| matches!(a.nal_unit_type(), NalUnitKind::VPS))
                .ok_or_else(|| anyhow!("no VPS in hvcC"))?
                .nal_units
                .first()
                .ok_or_else(|| anyhow!("vps array is empty"))?;

            let (_header, bitstream) = read_hvcc_nal_unit(&b.data)?;
            video_parameter_set_rbsp(&bitstream)?
        };

        dbg!(&vps);

        let sps = {
            let b = hevc_config
                .arrays
                .iter()
                .find(|a| matches!(a.nal_unit_type(), NalUnitKind::SPS))
                .ok_or_else(|| anyhow!("no SPS in hvcC"))?
                .nal_units
                .first()
                .ok_or_else(|| anyhow!("sps array is empty"))?;

            let (_header, bitstream) = read_hvcc_nal_unit(&b.data)?;
            sequence_parameter_set_rbsp(&bitstream)?
        };

        dbg!(&sps);

        // Parse PPS
        let pps = {
            let b = hevc_config
                .arrays
                .iter()
                .find(|a| matches!(a.nal_unit_type(), NalUnitKind::PPS))
                .ok_or_else(|| anyhow!("no PPS in hvcC"))?
                .nal_units
                .first()
                .ok_or_else(|| anyhow!("pps array is empty"))?;

            let (_header, bitstream) = read_hvcc_nal_unit(&b.data)?;
            picture_parameter_set_rbsp(&bitstream)?
        };

        dbg!(&pps);

        let primary_item_id = heif.primary_item_id();
        let primary_item_info = heif
            .item_info_by_item_id(primary_item_id)
            .ok_or_else(|| anyhow!("primary item {} not found in item_info", primary_item_id))?;

        match primary_item_info {
            ItemInfoEntry::Fixed { item_type, .. } => {
                match item_type {
                    ItemType::Grid => {
                        let item_references = heif
                            .meta_box
                            .item_references
                            .as_ref()
                            .ok_or_else(|| anyhow!("missing iref for grid image"))?;

                        let grid_ref = item_references
                            .references
                            .iter()
                            .find(|r| r.from_item_id == primary_item_id)
                            .ok_or_else(|| {
                                anyhow!("grid {} has no tile references", primary_item_id)
                            })?;

                        dbg!("Grid image with {} tiles", grid_ref.to_item_ids.len());

                        let tiles = grid_ref
                            .to_item_ids
                            .iter()
                            .map(|&tile_id| {
                                match reader.get_item_data(tile_id, &heif.meta_box.item_location) {
                                    Ok(bitstream) => read_item_nal_unit(bitstream),
                                    Err(e) => Err(e),
                                }
                            })
                            .collect::<Result<Vec<_>>>()?;

                        ensure!(tiles.iter().all(|(header, _)| matches!(
                            header.nal_unit_type(),
                            NalUnitKind::IdrNLp
                        )));

                        for (header, rbsp) in tiles {
                            let mut reader = SliceSegmentReader::new(&rbsp, header, &sps, &pps);
                            reader.read()?;
                        }
                    }
                    // ItemType::Hvc1 => {
                    //     let tile_data =
                    //         reader.get_item_data(primary_item_id, &heif.meta_box.item_location)?;
                    // }
                    _ => bail!("unsupported primary item type: {:?}", item_type),
                }
            }
        }

        Ok(())
    }
}

// no length prefix here
fn read_hvcc_nal_unit(raw_nal_unit: &[u8]) -> Result<(NalUnitHeader, Vec<u8>)> {
    match raw_nal_unit {
        [header_1, header_2, rbsp @ ..] => {
            let header = NalUnitHeader(u16::from_be_bytes([*header_1, *header_2]));
            Ok((header, RbspReader::remove_emulation_prevention(rbsp)))
        }
        _ => bail!("nal unit is too short"),
    }
}

// nal units from item data (tiles) has 4-byte length prefix
fn read_item_nal_unit(raw_nal_unit: &[u8]) -> Result<(NalUnitHeader, Vec<u8>)> {
    match raw_nal_unit {
        [len_0, len_1, len_2, len_3, header_1, header_2, rbsp @ ..] => {
            let nal_length = u32::from_be_bytes([*len_0, *len_1, *len_2, *len_3]) as usize;
            let actual_nal_size = 2 + rbsp.len(); // header (2 bytes) + rbsp

            ensure!(
                nal_length == actual_nal_size,
                "tile item should contain exactly one NAL unit: length prefix says {} bytes, but item has {} bytes of NAL data",
                nal_length,
                actual_nal_size
            );

            let header = NalUnitHeader(u16::from_be_bytes([*header_1, *header_2]));
            Ok((header, RbspReader::remove_emulation_prevention(rbsp)))
        }
        _ => bail!("nal unit is too short (need at least 6 bytes: 4 length + 2 header)"),
    }
}
