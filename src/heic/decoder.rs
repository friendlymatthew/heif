use crate::heif::{HeifReader, ItemInfoEntry, ItemType};
use anyhow::{Result, anyhow, bail};

#[derive(Debug)]
pub struct HeicDecoder;

impl HeicDecoder {
    pub fn decode<'a>(data: &'a [u8]) -> Result<()> {
        let mut reader = HeifReader::new(data);
        let heif = reader.read()?;

        let primary_item_id = heif.primary_item_id();
        let primary_item_info = heif
            .get_item_info_by_item_id(primary_item_id)
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
