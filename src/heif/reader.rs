use anyhow::{Result, anyhow, bail, ensure};

use crate::{
    BoxKind, ColorInformationBox, DataEntryBaseBox, DataEntryImdaBox, DataEntrySeqNumImdaBox,
    DataEntryUrlBox, DataEntryUrnBox, DataInformationBox, DataReferenceBox, FileTypeBox,
    HandlerBox, ImageRotationBox, ImageSpatialExtentsPropertyBox, IsoBmffBox, ItemInfoBox,
    ItemInfoEntry, ItemLocationBox, ItemLocationBoxReference, ItemPropertiesBox, ItemProperty,
    ItemPropertyAssociationBox, ItemPropertyContainerBox, ItemReferenceBox, ItemType, MetaBox,
    PixelInformationPropertyBox, PrimaryItemBox, RootBox, SingleItemReferenceBox, VersionFlag,
    impl_read_for_datatype,
};

#[derive(Debug)]
pub struct HeifReader<'a> {
    cursor: usize,
    data: &'a [u8],

    // a debug feature that helps point out where we are in the box tree
    box_stack: Vec<BoxKind<'a>>,
}

impl<'a> HeifReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            cursor: 0,
            data,
            box_stack: vec![RootBox::KIND],
        }
    }

    pub fn read(&mut self) -> Result<()> {
        let file_type_box = self.read_file_type_box()?;

        dbg!(&file_type_box);

        loop {
            if self.cursor == self.data.len() {
                break;
            }

            match self.peek_box_kind()? {
                b"meta" => {
                    let meta_box = self.read_meta_box()?;
                    dbg!(&meta_box);
                }
                foreign => self.skip_box(foreign)?,
            }
        }

        Ok(())
    }

    fn read_file_type_box(&mut self) -> Result<FileTypeBox<'a>> {
        self.with_box(&FileTypeBox::KIND, |this, start, box_size| {
            let major_brand = this.read_u32()?;
            let minor_version = this.read_u32()?;

            let elements = this.remaining_bytes_in_box(start, box_size) / 4;

            Ok(FileTypeBox {
                major_brand,
                minor_version,
                compatible_brands: this.read_slice_fn(elements, Self::read_box_kind)?,
            })
        })
    }

    fn read_meta_box(&mut self) -> Result<MetaBox<'a>> {
        self.with_full_box(&MetaBox::KIND, |this, start, box_size, version_flag| {
            ensure!(version_flag.version() == 0);

            let handler = this.read_handler_box()?;

            let mut primary_item = None;
            let mut item_info = None;
            let mut item_location = None;

            // optional boxes
            let mut item_properties = None;
            let mut item_references = None;
            let mut data_information = None;

            loop {
                if this.cursor == start + box_size {
                    break;
                }

                match this.peek_box_kind()? {
                    b"dinf" => {
                        data_information = Some(this.read_data_information_box()?);
                    }
                    b"pitm" => {
                        primary_item = Some(this.read_primary_item_box()?);
                    }
                    b"iinf" => {
                        item_info = Some(this.read_item_info_box()?);
                    }
                    b"iref" => {
                        item_references = Some(this.read_item_reference_box()?);
                    }
                    b"iprp" => {
                        item_properties = Some(this.read_item_properties_box()?);
                    }
                    b"iloc" => {
                        item_location = Some(this.read_item_location_box()?);
                    }
                    foreign => {
                        this.skip_box(foreign)?;
                        continue;
                    }
                }
            }

            Ok(MetaBox {
                handler,
                primary_item: primary_item.ok_or_else(|| anyhow!("missing required pitm box"))?,
                item_info: item_info.ok_or_else(|| anyhow!("missing required iinf box"))?,
                item_location: item_location.ok_or_else(|| anyhow!("missing required iloc box"))?,
                item_properties,
                item_references,
                data_information,
            })
        })
    }

    fn read_handler_box(&mut self) -> Result<HandlerBox<'a>> {
        self.with_full_box(&HandlerBox::KIND, |this, start, box_size, version_flag| {
            ensure!(version_flag.version() == 0);
            ensure!(version_flag.flags() == 0);
            ensure!(this.read_u32()? == 0, "predefined must be 0");
            ensure!(this.read_box_kind()? == BoxKind(b"pict"));
            ensure!(this.read_slice_fn(3, Self::read_u32)?.as_ref() == &[0, 0, 0]);

            let remainder = this.remaining_bytes_in_box(start, box_size);
            let name = str::from_utf8(this.read_slice(remainder)?)?;

            Ok(HandlerBox { kind: "pict", name })
        })
    }

    fn read_data_information_box(&mut self) -> Result<DataInformationBox<'a>> {
        self.with_box(&DataInformationBox::KIND, |this, _start, _box_size| {
            Ok(DataInformationBox(this.read_data_reference_box()?))
        })
    }

    fn read_data_reference_box(&mut self) -> Result<DataReferenceBox<'a>> {
        self.with_full_box(
            &DataReferenceBox::KIND,
            |this, _start, _box_size, version_flag| {
                ensure!(version_flag.version() == 0);

                let entry_count = this.read_u32()?;

                Ok(DataReferenceBox {
                    entries: this
                        .read_slice_fn(entry_count as usize, Self::read_data_entry_base_box)?,
                })
            },
        )
    }

    fn read_data_entry_base_box(&mut self) -> Result<DataEntryBaseBox<'a>> {
        let b = match BoxKind(self.peek_box_kind()?) {
            DataEntryUrlBox::KIND => DataEntryBaseBox::Url(self.read_data_entry_url_box()?),
            DataEntryUrnBox::KIND => DataEntryBaseBox::Urn(self.read_data_entry_urn_box()?),
            DataEntryImdaBox::KIND => DataEntryBaseBox::Imda(self.read_data_entry_imda_box()?),
            DataEntrySeqNumImdaBox::KIND => {
                DataEntryBaseBox::SeqNumImda(self.read_data_entry_seq_num_imda_box()?)
            }
            foreign => todo!("encountered foreign box {:?}", foreign),
        };

        Ok(b)
    }

    fn read_data_entry_url_box(&mut self) -> Result<DataEntryUrlBox<'a>> {
        self.with_full_box(
            &DataEntryUrlBox::KIND,
            |this, start, box_size, version_flag| {
                ensure!(version_flag.version() == 0);

                let remainder = this.remaining_bytes_in_box(start, box_size);

                Ok(DataEntryUrlBox {
                    location: str::from_utf8(this.read_slice(remainder)?)?,
                })
            },
        )
    }

    fn read_data_entry_urn_box(&mut self) -> Result<DataEntryUrnBox<'a>> {
        self.with_full_box(
            &DataEntryUrnBox::KIND,
            |this, start, box_size, version_flag| {
                ensure!(version_flag.version() == 0);

                let remainder = this.remaining_bytes_in_box(start, box_size);
                let bytes = this.read_slice(remainder)?;

                let name_end = bytes
                    .iter()
                    .position(|&b| b == 0x00)
                    .ok_or_else(|| anyhow::anyhow!("Missing null terminator after name"))?;

                Ok(DataEntryUrnBox {
                    name: str::from_utf8(&bytes[..name_end])?,
                    location: str::from_utf8(&bytes[name_end..])?,
                })
            },
        )
    }

    fn read_data_entry_imda_box(&mut self) -> Result<DataEntryImdaBox> {
        self.with_full_box(
            &DataEntryImdaBox::KIND,
            |this, _start, _box_size, version_flag| {
                ensure!(version_flag.version() == 0);

                Ok(DataEntryImdaBox {
                    version_flag,
                    imda_ref_identifier: this.read_u32()?,
                })
            },
        )
    }

    fn read_data_entry_seq_num_imda_box(&mut self) -> Result<DataEntrySeqNumImdaBox> {
        self.with_full_box(
            &DataEntrySeqNumImdaBox::KIND,
            |_this, _start, _box_size, version_flag| {
                ensure!(version_flag.version() == 0);

                Ok(DataEntrySeqNumImdaBox(version_flag))
            },
        )
    }

    fn read_primary_item_box(&mut self) -> Result<PrimaryItemBox> {
        self.with_full_box(
            &PrimaryItemBox::KIND,
            |this, _start, _box_size, version_flag| {
                Ok(PrimaryItemBox {
                    item_id: this.read_versioned_u32(version_flag.version(), 1)?,
                })
            },
        )
    }

    fn read_item_info_box(&mut self) -> Result<ItemInfoBox<'a>> {
        self.with_full_box(
            &ItemInfoBox::KIND,
            |this, _start, _box_size, version_flag| {
                ensure!(version_flag.flags() == 0);

                let len = this.read_versioned_u32(version_flag.version(), 1)?;

                Ok(ItemInfoBox {
                    item_info_entries: this
                        .read_slice_fn(len as usize, Self::read_item_info_entry)?,
                })
            },
        )
    }

    fn read_item_info_entry(&mut self) -> Result<ItemInfoEntry<'a>> {
        self.with_full_box(
            &ItemInfoEntry::KIND,
            |this, start, box_size, version_flag| {
                let item_info_entry = match version_flag.version() {
                    0 | 1 => todo!("how does version 1, 2 parse"),
                    v => {
                        let item_id = if v == 2 {
                            this.read_u16()? as u32
                        } else if v == 3 {
                            this.read_u32()?
                        } else {
                            bail!("unrecognized version {v}");
                        };

                        let item_protection_index = this.read_u16()?;
                        let item_type = this.read_slice(4)?;

                        let remainder_len = this.remaining_bytes_in_box(start, box_size);
                        let mut remainder = this.read_slice(remainder_len)?;

                        let item_name_end = remainder
                            .iter()
                            .position(|&b| b == 0x00)
                            .ok_or_else(|| anyhow!("expected string"))?;

                        let item_name = str::from_utf8(&remainder[..item_name_end])?;

                        remainder = &remainder[item_name_end..];

                        let item_type = match item_type {
                            b"mime" => {
                                let content_type_end = remainder
                                    .iter()
                                    .position(|&b| b == 0x00)
                                    .ok_or_else(|| anyhow!("expected string"))?;

                                let content_type = str::from_utf8(&remainder[..content_type_end])?;
                                let content_encoding =
                                    str::from_utf8(&remainder[content_type_end..])?;

                                ItemType::Mime {
                                    content_type,
                                    content_encoding,
                                }
                            }
                            b"uri " => {
                                let item_uri_type_end =
                                    remainder
                                        .iter()
                                        .position(|&b| b == 0x00)
                                        .ok_or_else(|| anyhow!("expected string"))?;

                                let item_uri_type =
                                    str::from_utf8(&remainder[..item_uri_type_end])?;

                                ItemType::Uri { item_uri_type }
                            }
                            b"hvc1" => ItemType::Hvc1,
                            b"grid" => ItemType::Grid,
                            b"Exif" => ItemType::Exif,
                            _ => bail!("unrecognized item type {:?}", str::from_utf8(item_type)),
                        };

                        ItemInfoEntry::Fixed {
                            item_id,
                            item_name,
                            item_protection_index,
                            item_type,
                        }
                    }
                };

                Ok(item_info_entry)
            },
        )
    }

    fn read_item_reference_box(&mut self) -> Result<ItemReferenceBox<'a>> {
        self.with_full_box(
            &ItemReferenceBox::KIND,
            |this, start, box_size, version_flag| {
                let mut references = Vec::new();

                loop {
                    if this.cursor == start + box_size {
                        break;
                    }

                    references
                        .push(this.read_single_item_reference_box(version_flag.version() > 0)?);
                }

                Ok(ItemReferenceBox {
                    references: references.into_boxed_slice(),
                })
            },
        )
    }

    fn read_single_item_reference_box(
        &mut self,
        large: bool,
    ) -> Result<SingleItemReferenceBox<'a>> {
        self.with_box_unchecked(|this, kind, _start, _box_size| {
            Ok(SingleItemReferenceBox {
                kind,
                from_item_id: if large {
                    this.read_u32()?
                } else {
                    this.read_u16()? as u32
                },
                to_item_ids: {
                    let len = this.read_u16()?;
                    this.read_slice_fn(len as usize, |this| {
                        if large {
                            this.read_u32()
                        } else {
                            Ok(this.read_u16()? as u32)
                        }
                    })?
                },
            })
        })
    }

    fn read_item_properties_box(&mut self) -> Result<ItemPropertiesBox> {
        self.with_box(&ItemPropertiesBox::KIND, |this, _start, _box_size| {
            Ok(ItemPropertiesBox {
                container: this.read_item_property_container_box()?,
                association: this.read_item_property_association_box()?,
            })
        })
    }

    fn read_item_property_container_box(&mut self) -> Result<ItemPropertyContainerBox> {
        self.with_box(&ItemPropertyContainerBox::KIND, |this, start, box_size| {
            let mut properties = Vec::new();

            loop {
                if this.cursor == start + box_size {
                    break;
                }

                let property = match BoxKind(this.peek_box_kind()?) {
                    ColorInformationBox::KIND => {
                        ItemProperty::ColorInformation(this.read_color_information_box()?)
                    }
                    BoxKind(b"hvcC") => {
                        // skip hevc configuration box for now
                        // panic!("need to read");

                        let start = this.cursor;
                        let (_, box_size) = this.read_box_header()?;

                        this.cursor = start + box_size;
                        continue;
                    }
                    ImageSpatialExtentsPropertyBox::KIND => {
                        ItemProperty::ImageSpatialExtentsProperty(
                            this.read_image_spatial_extents_property_box()?,
                        )
                    }
                    ImageRotationBox::KIND => {
                        ItemProperty::ImageRotation(this.read_image_rotation_box()?)
                    }
                    PixelInformationPropertyBox::KIND => ItemProperty::PixelInformationProperty(
                        this.read_pixel_information_property_box()?,
                    ),
                    foreign => {
                        this.skip_box(foreign.0)?;
                        continue;
                    }
                };

                properties.push(property);
            }

            Ok(ItemPropertyContainerBox {
                properties: properties.into_boxed_slice(),
            })
        })
    }

    fn read_item_property_association_box(&mut self) -> Result<ItemPropertyAssociationBox> {
        self.with_full_box(
            &ItemPropertyAssociationBox::KIND,
            |this, _start, _box_size, version_flag| {
                let entry_count = this.read_u32()?;

                let mut out = Vec::with_capacity(entry_count as usize);

                for _ in 0..entry_count {
                    let item_id = this.read_versioned_u32(version_flag.version(), 1)?;

                    let assoc_ct = this.read_u8()?;
                    let mut assocs = Vec::with_capacity(assoc_ct as usize);

                    for _ in 0..assoc_ct {
                        let raw_index = if (version_flag.flags() & 1) == 1 {
                            this.read_u16()?
                        } else {
                            this.read_u8()? as u16
                        };

                        let property_index = if raw_index < 256 {
                            raw_index & 0x7F
                        } else {
                            raw_index & 0x7FFF
                        };

                        assocs.push(property_index);
                    }

                    out.push((item_id, assocs.into_boxed_slice()))
                }

                Ok(ItemPropertyAssociationBox {
                    assoc: out.into_boxed_slice(),
                })
            },
        )
    }

    fn read_color_information_box(&mut self) -> Result<ColorInformationBox> {
        self.with_box(&ColorInformationBox::KIND, |this, start, box_size| {
            match this.read_slice(4)? {
                b"prof" => {
                    let remainder = this.remaining_bytes_in_box(start, box_size);
                    let _icc_profile_data = this.read_slice(remainder)?;

                    // todo: proper faff!
                    // let _profile = ICCProfileReader::new(this.read_slice(remainder)?).read()?;
                }
                foreign => todo!(
                    "encountered todo profile type: {:?}. IsoBMFF 12.1.5.2",
                    str::from_utf8(foreign)
                ),
            };

            Ok(ColorInformationBox {})
        })
    }

    fn read_image_spatial_extents_property_box(
        &mut self,
    ) -> Result<ImageSpatialExtentsPropertyBox> {
        self.with_full_box(
            &ImageSpatialExtentsPropertyBox::KIND,
            |this, _start, _box_size, _version_flag| {
                Ok(ImageSpatialExtentsPropertyBox {
                    image_width: this.read_u32()?,
                    image_height: this.read_u32()?,
                })
            },
        )
    }

    fn read_image_rotation_box(&mut self) -> Result<ImageRotationBox> {
        self.with_box(&ImageRotationBox::KIND, |this, _start, _box_size| {
            Ok(ImageRotationBox {
                angle: this.read_u8()? & 0b11,
            })
        })
    }

    fn read_pixel_information_property_box(&mut self) -> Result<PixelInformationPropertyBox> {
        self.with_full_box(
            &PixelInformationPropertyBox::KIND,
            |this, _start, _box_size, _version_flag| {
                let num_channels = this.read_u8()?;

                Ok(PixelInformationPropertyBox {
                    bits_per_channel: this.read_slice_fn(num_channels as usize, Self::read_u8)?,
                })
            },
        )
    }

    fn read_item_location_box(&mut self) -> Result<ItemLocationBox> {
        self.with_full_box(
            &ItemLocationBox::KIND,
            |this, _start, _box_size, version_flag| {
                let version = version_flag.version();
                ensure!(version <= 2, "Unsupported iloc version: {}", version);

                let b1 = this.read_u8()?;
                let offset_size = (b1 >> 4) & 0x0F;
                let length_size = b1 & 0x0F;

                let b2 = this.read_u8()?;
                let base_offset_size = (b2 >> 4) & 0x0F;
                let index_size = b2 & 0x0F;

                let item_count = this.read_versioned_u32(version, 2)?;

                Ok(ItemLocationBox {
                    offset_size,
                    length_size,
                    base_offset_size,
                    index_size,
                    references: {
                        let mut out = Vec::with_capacity(item_count as usize);

                        for _ in 0..item_count {
                            let item_id = this.read_versioned_u32(version, 2)?;

                            let construction_method = if version == 1 || version == 2 {
                                let packed = this.read_u16()?;
                                // Upper 12 bits are reserved, lower 4 bits are construction_method
                                Some(packed & 0x0F)
                            } else {
                                None
                            };

                            let data_reference_index = this.read_u16()?;

                            let base_offset = this.read_variable_size(base_offset_size)?;
                            let extent_count = this.read_u16()?;

                            let mut extents = Vec::with_capacity(extent_count as usize);

                            for _ in 0..extent_count {
                                // Optional item_reference_index (only in version 1/2 with index_size > 0)
                                let _item_reference_index =
                                    if (version == 1 || version == 2) && index_size > 0 {
                                        Some(this.read_variable_size(index_size)?)
                                    } else {
                                        None
                                    };

                                let extent_offset = this.read_variable_size(offset_size)?;
                                let extent_length = this.read_variable_size(length_size)?;

                                extents.push((extent_offset, extent_length));
                            }

                            out.push(ItemLocationBoxReference {
                                item_id,
                                construction_method,
                                data_reference_index,
                                base_offset,
                                extents: extents.into_boxed_slice(),
                            });
                        }

                        out.into_boxed_slice()
                    },
                })
            },
        )
    }

    fn read_variable_size(&mut self, size: u8) -> Result<u64> {
        match size {
            0 => Ok(0),
            4 => Ok(self.read_u32()? as u64),
            8 => Ok(self.read_u64()?),
            _ => bail!("unsupported size: {}", size),
        }
    }

    // some helper methods to reduce ceremony

    /// read u32 with version-dependent size: u16 if version < threshold, otherwise u32
    fn read_versioned_u32(&mut self, version: u8, threshold: u8) -> Result<u32> {
        let n = if version < threshold {
            self.read_u16()? as u32
        } else {
            self.read_u32()?
        };

        Ok(n)
    }

    const fn remaining_bytes_in_box(&self, start: usize, box_size: usize) -> usize {
        box_size - (self.cursor - start)
    }

    fn skip_box(&mut self, foreign_box_kind: &[u8]) -> Result<()> {
        eprintln!(
            "Skipping unrecognized box: {:?}\n\tbox stack: {:?}",
            str::from_utf8(foreign_box_kind),
            self.box_stack
        );
        let start = self.cursor;
        let (_, box_size) = self.read_box_header()?;
        self.cursor = start + box_size;

        Ok(())
    }

    fn with_box<T>(
        &mut self,
        expected_kind: &BoxKind<'a>,
        f: impl FnOnce(&mut Self, usize, usize) -> Result<T>,
    ) -> Result<T> {
        self.box_stack.push(expected_kind.clone());
        let start = self.cursor;
        let (kind, box_size) = self.read_box_header()?;
        ensure!(kind == *expected_kind);

        let result = f(self, start, box_size)?;

        ensure!(self.cursor == start + box_size);
        self.box_stack.pop();

        Ok(result)
    }

    fn with_full_box<T>(
        &mut self,
        expected_kind: &BoxKind<'a>,
        f: impl FnOnce(&mut Self, usize, usize, VersionFlag) -> Result<T>,
    ) -> Result<T> {
        self.box_stack.push(expected_kind.clone());
        let start = self.cursor;
        let (kind, box_size, version_flag) = self.read_full_box_header()?;
        ensure!(kind == *expected_kind);

        let result = f(self, start, box_size, version_flag)?;

        ensure!(self.cursor == start + box_size);
        self.box_stack.pop();

        Ok(result)
    }

    fn with_box_unchecked<T>(
        &mut self,
        f: impl FnOnce(&mut Self, BoxKind<'a>, usize, usize) -> Result<T>,
    ) -> Result<T> {
        let start = self.cursor;
        let (kind, box_size) = self.read_box_header()?;

        self.box_stack.push(kind.clone());
        let result = f(self, kind, start, box_size)?;

        ensure!(self.cursor == start + box_size);
        self.box_stack.pop();

        Ok(result)
    }

    fn read_version_flag(&mut self) -> Result<VersionFlag> {
        Ok(VersionFlag::from(self.read_u32()?))
    }

    fn read_full_box_header(&mut self) -> Result<(BoxKind<'a>, usize, VersionFlag)> {
        let (kind, box_size) = self.read_box_header()?;
        Ok((kind, box_size, self.read_version_flag()?))
    }

    fn read_box_header(&mut self) -> Result<(BoxKind<'a>, usize)> {
        let mut size = self.read_u32()? as usize;
        let kind = self.read_box_kind()?;

        if size == 1 {
            size = self.read_u64()? as usize;
        }

        if kind == b"uuid".into() {
            let _user_kind = self.read_slice(16)?;
        }

        Ok((kind, size))
    }

    // todo: make a commen tabout where to call this method
    fn peek_box_kind(&self) -> Result<&'a [u8; 4]> {
        self.data
            .get(self.cursor + 4..self.cursor + 8)
            .ok_or_else(|| anyhow!("oob"))?
            .try_into()
            .map_err(|_| anyhow!("should fit"))
    }

    fn read_box_kind(&mut self) -> Result<BoxKind<'a>> {
        Ok(BoxKind(self.read_fixed_slice::<4>()?))
    }

    fn read_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        let s = self
            .data
            .get(self.cursor..self.cursor + len)
            .ok_or_else(|| anyhow!("oob"))?;

        self.cursor += len;
        Ok(s)
    }

    fn read_fixed_slice<const N: usize>(&mut self) -> Result<&'a [u8; N]> {
        self.read_slice(N)?
            .try_into()
            .map_err(|_| anyhow!("should fit"))
    }

    fn read_slice_fn<T>(
        &mut self,
        len: usize,
        f: impl Fn(&mut Self) -> Result<T>,
    ) -> Result<Box<[T]>> {
        (0..len)
            .map(|_| f(self))
            .collect::<Result<Vec<_>>>()
            .map(|v| v.into_boxed_slice())
    }

    impl_read_for_datatype!(read_u8, u8);
    impl_read_for_datatype!(read_u16, u16);
    impl_read_for_datatype!(read_u32, u32);
    impl_read_for_datatype!(read_u64, u64);
}
