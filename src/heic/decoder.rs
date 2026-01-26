use crate::heif::{HeifReader, ItemInfoEntry, ItemType};
use crate::hevc::{
    NalUnitKind, RbspReader, picture_parameter_set_rbsp, sequence_parameter_set_rbsp,
    video_parameter_set_rbsp,
};
use anyhow::{Result, anyhow, bail};

#[derive(Debug)]
pub struct HeicDecoder;

impl HeicDecoder {
    pub fn decode(data: &[u8]) -> Result<()> {
        let mut reader = HeifReader::new(data);
        let heif = reader.read()?;

        let hevc_config = heif
            .hevc_configuration_record()
            .ok_or_else(|| anyhow!("missing HEVC decoder configuration"))?;

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

            let (_header, bitstream) = read_raw_nal_unit(&b.data)?;
            video_parameter_set_rbsp(&bitstream)?
        };

        dbg!(vps);

        let sps = {
            let b = hevc_config
                .arrays
                .iter()
                .find(|a| matches!(a.nal_unit_type(), NalUnitKind::SPS))
                .ok_or_else(|| anyhow!("no SPS in hvcC"))?
                .nal_units
                .first()
                .ok_or_else(|| anyhow!("sps array is empty"))?;

            let (_header, bitstream) = read_raw_nal_unit(&b.data)?;
            sequence_parameter_set_rbsp(&bitstream)?
        };

        dbg!(sps);

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

            let (_header, bitstream) = read_raw_nal_unit(&b.data)?;
            picture_parameter_set_rbsp(&bitstream)?
        };

        dbg!(pps);

        let primary_item_id = heif.primary_item_id();
        let primary_item_info = heif
            .item_info_by_item_id(primary_item_id)
            .ok_or_else(|| anyhow!("primary item {} not found in item_info", primary_item_id))?;

        match primary_item_info {
            ItemInfoEntry::Fixed { item_type, .. } => match item_type {
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

                    println!("\nGrid image with {} tiles", grid_ref.to_item_ids.len());

                    // every tile is an hevc bitstream
                    let _tiles = grid_ref
                        .to_item_ids
                        .iter()
                        .map(|&tile_id| reader.get_item_data(tile_id, &heif.meta_box.item_location))
                        .collect::<Result<Vec<_>>>()?;
                }
                // ItemType::Hvc1 => {
                //     let tile_data =
                //         reader.get_item_data(primary_item_id, &heif.meta_box.item_location)?;
                // }
                _ => bail!("unsupported primary item type: {:?}", item_type),
            },
        }

        Ok(())
    }
}

fn read_raw_nal_unit(raw_nal_unit: &[u8]) -> Result<(&[u8], Vec<u8>)> {
    let (header, raw) = raw_nal_unit
        .split_at_checked(2)
        .ok_or_else(|| anyhow!("nal unit is too short"))?;

    Ok((header, RbspReader::remove_emulation_prevention(raw)))
}
