use std::fmt::Debug;

use crate::hevc::HEVCDecoderConfigurationRecord;

macro_rules! impl_box {
    ($box_struct:ident<$lifetime:lifetime>, $box_kind:expr) => {
        impl<$lifetime> IsoBmffBox<$lifetime> for $box_struct<$lifetime> {
            const KIND: BoxKind<'a> = BoxKind($box_kind);
        }
    };

    ($box_struct:ident, $box_kind:expr) => {
        impl<'a> IsoBmffBox<'a> for $box_struct {
            const KIND: BoxKind<'a> = BoxKind($box_kind);
        }
    };
}

#[derive(Debug)]
pub struct Heif<'a> {
    pub file_type_box: FileTypeBox<'a>,
    pub meta_box: MetaBox<'a>,
}

impl<'a> Heif<'a> {
    pub const fn primary_item_id(&self) -> u32 {
        self.meta_box.primary_item.item_id
    }

    pub fn get_item_info_by_item_id(&self, target_item_id: u32) -> Option<&ItemInfoEntry<'a>> {
        self.meta_box
            .item_info
            .item_info_entries
            .iter()
            .find(|ItemInfoEntry::Fixed { item_id, .. }| *item_id == target_item_id)
    }
}

// not a real box. but to indicate we're in the root
#[derive(Debug)]
pub struct RootBox;

impl_box!(RootBox, b"root");

#[derive(PartialEq, Eq, Clone)]
pub struct BoxKind<'a>(pub &'a [u8; 4]);

impl<'a> From<&'a [u8; 4]> for BoxKind<'a> {
    fn from(value: &'a [u8; 4]) -> Self {
        Self(value)
    }
}

impl<'a> TryFrom<&'a [u8]> for BoxKind<'a> {
    type Error = anyhow::Error;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl<'a> Debug for BoxKind<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(str::from_utf8(self.0).unwrap())
    }
}

#[derive(Debug)]
pub struct VersionFlag(u32);

impl From<u32> for VersionFlag {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl VersionFlag {
    pub const fn version(&self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub const fn flags(&self) -> u32 {
        self.0 & 0xFFFFFF
    }
}

#[derive(Debug)]
pub struct HandlerBox<'a> {
    pub kind: &'a str,
    pub name: &'a str,
}

impl_box!(HandlerBox<'a>, b"hdlr");

#[derive(Debug)]
pub struct DataInformationBox<'a>(pub DataReferenceBox<'a>);

impl_box!(DataInformationBox<'a>, b"dinf");

#[derive(Debug)]
pub struct DataReferenceBox<'a> {
    pub entries: Box<[DataEntryBaseBox<'a>]>,
}

impl_box!(DataReferenceBox<'a>, b"dref");

#[derive(Debug)]
pub enum DataEntryBaseBox<'a> {
    Url(DataEntryUrlBox<'a>),
    Urn(DataEntryUrnBox<'a>),
    Imda(DataEntryImdaBox),
    SeqNumImda(DataEntrySeqNumImdaBox),
}

#[derive(Debug)]
pub struct DataEntryUrlBox<'a> {
    pub location: &'a str,
}

impl_box!(DataEntryUrlBox<'a>, b"url ");

#[derive(Debug)]
pub struct DataEntryUrnBox<'a> {
    pub name: &'a str,
    pub location: &'a str,
}

impl_box!(DataEntryUrnBox<'a>, b"urn ");

#[derive(Debug)]
pub struct DataEntryImdaBox {
    pub version_flag: VersionFlag,
    pub imda_ref_identifier: u32,
}

impl_box!(DataEntryImdaBox, b"imdt");

#[derive(Debug)]
pub struct DataEntrySeqNumImdaBox(pub VersionFlag);

impl_box!(DataEntrySeqNumImdaBox, b"snim");

#[derive(Debug)]
pub struct PrimaryItemBox {
    pub item_id: u32,
}

impl_box!(PrimaryItemBox, b"pitm");

#[derive(Debug)]
pub struct ItemInfoBox<'a> {
    pub item_info_entries: Box<[ItemInfoEntry<'a>]>,
}

impl_box!(ItemInfoBox<'a>, b"iinf");

#[derive(Debug)]
pub enum ItemType<'a> {
    Mime {
        content_type: &'a str,
        content_encoding: &'a str,
    },
    Uri {
        item_uri_type: &'a str,
    },
    Hvc1,
    Grid,
    Exif,
}

#[derive(Debug)]
pub enum ItemInfoEntry<'a> {
    Fixed {
        item_id: u32,
        item_name: &'a str,
        item_protection_index: u16,
        item_type: ItemType<'a>,
    },
}

impl_box!(ItemInfoEntry<'a>, b"infe");

#[derive(Debug)]
pub struct ItemReferenceBox<'a> {
    pub references: Box<[SingleItemReferenceBox<'a>]>,
}

impl_box!(ItemReferenceBox<'a>, b"iref");

#[derive(Debug)]
pub struct SingleItemReferenceBox<'a> {
    pub kind: BoxKind<'a>,
    pub from_item_id: u32,
    pub to_item_ids: Box<[u32]>,
}

#[derive(Debug)]
pub struct ItemPropertiesBox {
    pub container: ItemPropertyContainerBox,
    pub association: ItemPropertyAssociationBox,
}

impl_box!(ItemPropertiesBox, b"iprp");

#[derive(Debug)]
pub struct ItemPropertyContainerBox {
    pub properties: Box<[ItemProperty]>,
}

impl_box!(ItemPropertyContainerBox, b"ipco");

#[derive(Debug)]
pub enum ItemProperty {
    ColorInformation(ColorInformationBox),
    HevcDecoderConfiguration(HEVCDecoderConfigurationRecord),
    ImageSpatialExtentsProperty(ImageSpatialExtentsPropertyBox),
    ImageRotation(ImageRotationBox),
    PixelInformationProperty(PixelInformationPropertyBox),
}

#[derive(Debug)]
pub struct ItemPropertyAssociationBox {
    pub assoc: Box<[(u32, Box<[u16]>)]>,
}

impl_box!(ItemPropertyAssociationBox, b"ipma");

#[derive(Debug)]
pub struct ColorInformationBox {}

impl_box!(ColorInformationBox, b"colr");

#[derive(Debug)]
pub struct ImageSpatialExtentsPropertyBox {
    pub image_width: u32,
    pub image_height: u32,
}

impl_box!(ImageSpatialExtentsPropertyBox, b"ispe");

#[derive(Debug)]
pub struct ImageRotationBox {
    pub angle: u8,
}

impl_box!(ImageRotationBox, b"irot");

#[derive(Debug)]
pub struct PixelInformationPropertyBox {
    pub bits_per_channel: Box<[u8]>,
}

impl_box!(PixelInformationPropertyBox, b"pixi");

//

pub trait IsoBmffBox<'a> {
    const KIND: BoxKind<'a>;
}

#[derive(Debug)]
pub struct FileTypeBox<'a> {
    pub major_brand: u32,
    pub minor_version: u32,
    pub compatible_brands: Box<[BoxKind<'a>]>,
}

impl_box!(FileTypeBox<'a>, b"ftyp");

#[derive(Debug)]
pub struct MetaBox<'a> {
    // Required boxes
    pub handler: HandlerBox<'a>,
    pub primary_item: PrimaryItemBox,
    pub item_info: ItemInfoBox<'a>,
    pub item_location: ItemLocationBox,

    // Optional but common boxes
    pub item_properties: Option<ItemPropertiesBox>,
    pub item_references: Option<ItemReferenceBox<'a>>,

    // Optional boxes
    pub data_information: Option<DataInformationBox<'a>>,
}

impl_box!(MetaBox<'a>, b"meta");

#[derive(Debug)]
pub struct ItemLocationBox {
    pub offset_size: u8,
    pub length_size: u8,
    pub base_offset_size: u8,
    pub index_size: u8,
    pub references: Box<[ItemLocationBoxReference]>,
}

impl_box!(ItemLocationBox, b"iloc");

#[derive(Debug)]
pub struct ItemLocationBoxReference {
    pub item_id: u32,
    pub construction_method: u16,
    pub data_reference_index: u16,
    pub base_offset: u64,
    /// (offset, length)
    pub extents: Box<[(u64, u64)]>,
}
