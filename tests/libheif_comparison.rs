use std::path::{Path, PathBuf};

const TEST_FILE: &str = "halfmoonbay.heic";

fn get_test_file_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(TEST_FILE)
}

#[test]
fn cross_check_with_libheif() {
    let path = get_test_file_path();
    let data = std::fs::read(&path).expect("failed to read file");

    let ctx = libheif_rs::HeifContext::read_from_file(path.to_str().unwrap())
        .expect("failed to read HEIC");
    let handle = ctx.primary_image_handle().expect("failed to get handle");

    let libheif_ispe_width = handle.ispe_width();
    let libheif_ispe_height = handle.ispe_height();
    let libheif_width = handle.width();
    let libheif_height = handle.height();
    let libheif_luma_bits = handle.luma_bits_per_pixel();
    let libheif_chroma_bits = handle.chroma_bits_per_pixel();
    // let libheif_has_alpha = handle.has_alpha_channel();
    let libheif_is_primary = handle.is_primary();
    let libheif_num_thumbnails = handle.number_of_thumbnails();
    // let libheif_has_depth = handle.has_depth_image();

    // ----

    let mut reader = heif::HeifReader::new(&data);
    let heif = reader.read().expect("failed to parse HEIF");

    let iprp = heif
        .meta_box
        .item_properties
        .as_ref()
        .expect("No properties");
    let primary_id = heif.primary_item_id();

    let primary_assoc = iprp
        .association
        .assoc
        .iter()
        .find(|(id, _)| *id == primary_id)
        .expect("No association for primary item");

    let mut our_ispe_width = 0i32;
    let mut our_ispe_height = 0i32;
    let mut our_rotation = 0u8;

    for &prop_idx in primary_assoc.1.iter() {
        if prop_idx == 0 {
            continue;
        }
        let prop = &iprp.container.properties[(prop_idx - 1) as usize];
        match prop {
            heif::heif::ItemProperty::ImageSpatialExtentsProperty(ispe) => {
                our_ispe_width = ispe.image_width as i32;
                our_ispe_height = ispe.image_height as i32;
            }
            heif::heif::ItemProperty::ImageRotation(irot) => {
                our_rotation = irot.angle;
            }
            _ => {}
        }
    }

    let (our_width, our_height) = if our_rotation == 1 || our_rotation == 3 {
        // 90 or 270 degree rotation swaps dimensions
        (our_ispe_height as u32, our_ispe_width as u32)
    } else {
        (our_ispe_width as u32, our_ispe_height as u32)
    };

    let our_num_thumbnails = heif
        .meta_box
        .item_references
        .as_ref()
        .map(|iref| {
            iref.references
                .iter()
                .filter(|r| r.kind.0 == b"thmb" && r.to_item_ids.contains(&primary_id))
                .count()
        })
        .unwrap_or(0);

    let hevc_config = heif.hevc_configuration_record().expect("No HEVC config");
    let sps_array = hevc_config
        .arrays
        .iter()
        .find(|a| matches!(a.nal_unit_type(), heif::hevc::NalUnitKind::SPS))
        .expect("No SPS");

    let sps_rbsp =
        heif::hevc::RbspReader::remove_emulation_prevention(&sps_array.nal_units[0].data[2..]);
    let sps = heif::hevc::sequence_parameter_set_rbsp(&sps_rbsp).expect("failed to parse SPS");

    let our_luma_bits = (sps.bit_depth_luma_minus8 + 8) as u8;
    let our_chroma_bits = (sps.bit_depth_chroma_minus8 + 8) as u8;

    assert_eq!(libheif_ispe_width, our_ispe_width);
    assert_eq!(libheif_ispe_height, our_ispe_height);
    assert_eq!(libheif_width, our_width);
    assert_eq!(libheif_height, our_height);

    assert_eq!(libheif_luma_bits, our_luma_bits);
    assert_eq!(libheif_chroma_bits, our_chroma_bits);
    assert!(libheif_is_primary);

    assert_eq!(libheif_num_thumbnails, our_num_thumbnails,);
}
