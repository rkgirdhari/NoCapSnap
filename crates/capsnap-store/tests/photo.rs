//! Spec §3 "Client-Side Hardening": 2048 px long edge, EXIF and GPS stripped.
mod common;

use std::io::Cursor;

use capsnap_store::photo::{MAX_LONG_EDGE, THUMB_LONG_EDGE, process};
use common::*;
use image::{GenericImageView, ImageFormat};

fn decode(jpeg: &[u8]) -> image::DynamicImage {
    image::load_from_memory_with_format(jpeg, ImageFormat::Jpeg).unwrap()
}

fn read_exif(bytes: &[u8]) -> Result<exif::Exif, exif::Error> {
    exif::Reader::new().read_from_container(&mut Cursor::new(bytes))
}

#[test]
fn the_fixture_really_carries_orientation_make_and_gps() {
    // Guards the test below: an absent tag afterwards only means something if
    // an independent parser finds it beforehand.
    let source = with_segment(&jpeg(64, 32), &exif_app1());
    let exif = read_exif(&source).unwrap();
    let orientation = exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .unwrap();
    assert_eq!(orientation.value.get_uint(0), Some(6));
    assert!(exif.get_field(exif::Tag::Make, exif::In::PRIMARY).is_some());
    let lat = exif
        .get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)
        .unwrap();
    assert_eq!(lat.display_value().to_string(), "41 deg 52 min 30 sec");
}

#[test]
fn applies_orientation_then_strips_every_metadata_segment() {
    let source = with_segment(&jpeg(64, 32), &exif_app1());

    let out = process(&source, ImageFormat::Jpeg).unwrap();

    // Orientation 6 is baked into the pixels: a 64×32 landscape becomes 32×64,
    // and the red top-left corner lands top-right.
    assert_eq!((out.width, out.height), (32, 64));
    let pixels = decode(&out.jpeg).to_rgb8();
    assert!(
        near(*pixels.get_pixel(28, 3), RED),
        "{:?}",
        pixels.get_pixel(28, 3)
    );
    assert!(
        near(*pixels.get_pixel(3, 3), BLUE),
        "{:?}",
        pixels.get_pixel(3, 3)
    );

    // Independent parser: no EXIF at all, so no GPS, Make or Orientation.
    assert!(matches!(
        read_exif(&out.jpeg),
        Err(exif::Error::NotFound(_))
    ));
    assert!(matches!(
        read_exif(&out.thumb_jpeg),
        Err(exif::Error::NotFound(_))
    ));
    // Byte level: only JFIF (APP0), tables and frame headers before the scan;
    // no APP1–APP15 (EXIF, XMP, ICC, maker notes) and no comments.
    for jpeg in [&out.jpeg, &out.thumb_jpeg] {
        for marker in header_markers(jpeg) {
            assert!(
                matches!(marker, 0xE0 | 0xDB | 0xC0 | 0xC2 | 0xC4 | 0xDD | 0xDA),
                "unexpected segment FF{marker:02X}"
            );
        }
        assert!(!jpeg.windows(6).any(|w| w == b"Exif\0\0"));
    }
}

#[test]
fn downscales_to_2048_on_the_long_edge_keeping_aspect_ratio() {
    // 3000 px takes the area-averaging path, 2400 px the Catmull-Rom path.
    for (w, h) in [(3000, 1000), (1200, 2400)] {
        let out = process(&jpeg(w, h), ImageFormat::Jpeg).unwrap();
        let long = out.width.max(out.height);
        let short = out.width.min(out.height);
        assert_eq!(long, MAX_LONG_EDGE, "{w}x{h}");
        let expected_short = f64::from(w.min(h)) * f64::from(MAX_LONG_EDGE) / f64::from(w.max(h));
        assert!(
            (f64::from(short) - expected_short).abs() <= 1.0,
            "{w}x{h} -> {short}"
        );
        assert_eq!(decode(&out.jpeg).dimensions(), (out.width, out.height));
        assert_eq!(out.height > out.width, h > w, "orientation kept");

        let thumb = decode(&out.thumb_jpeg);
        assert_eq!(thumb.width().max(thumb.height()), THUMB_LONG_EDGE);
    }
}

#[test]
fn never_upscales_a_small_photo() {
    let out = process(&jpeg(800, 600), ImageFormat::Jpeg).unwrap();
    assert_eq!((out.width, out.height), (800, 600));
    let thumb = decode(&out.thumb_jpeg);
    assert_eq!(thumb.dimensions(), (THUMB_LONG_EDGE, 360));

    let tiny = process(&jpeg(40, 30), ImageFormat::Jpeg).unwrap();
    assert_eq!(decode(&tiny.thumb_jpeg).dimensions(), (40, 30));
}

#[test]
fn png_and_webp_become_jpeg_with_transparency_on_white() {
    let out = process(&png_with_transparency(40, 20), ImageFormat::Png).unwrap();
    let pixels = decode(&out.jpeg).to_rgb8();
    assert!(near(*pixels.get_pixel(5, 10), BLUE));
    // Transparent black must come out white, not black.
    assert!(near(*pixels.get_pixel(35, 10), image::Rgb([255, 255, 255])));

    let webp = encode(&marked(60, 40), ImageFormat::WebP);
    let out = process(&webp, ImageFormat::WebP).unwrap();
    assert_eq!((out.width, out.height), (60, 40));
    assert!(near(*decode(&out.jpeg).to_rgb8().get_pixel(3, 3), RED));
}

#[test]
fn truncated_or_fake_images_are_refused() {
    let whole = jpeg(64, 64);
    assert!(process(&whole[..whole.len() / 3], ImageFormat::Jpeg).is_err());
    // A bare JFIF header with no image, as the W1 tests used.
    let header_only = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0xFF, 0xD9,
    ];
    assert!(process(&header_only, ImageFormat::Jpeg).is_err());
}

#[test]
fn processing_is_deterministic_so_digests_deduplicate() {
    let source = jpeg(300, 200);
    let a = process(&source, ImageFormat::Jpeg).unwrap();
    let b = process(&source, ImageFormat::Jpeg).unwrap();
    assert_eq!(a.jpeg, b.jpeg);
}

/// Evidence for the W2 report, not a pass/fail gate:
/// `cargo test --release --test photo -- --ignored --nocapture`.
#[test]
#[ignore]
fn timing_12_megapixel_photo() {
    let source = jpeg(4000, 3000);
    let start = std::time::Instant::now();
    let out = process(&source, ImageFormat::Jpeg).unwrap();
    println!(
        "4000x3000 JPEG ({} KiB) -> {}x{} ({} KiB) + thumb ({} KiB) in {:?}",
        source.len() / 1024,
        out.width,
        out.height,
        out.jpeg.len() / 1024,
        out.thumb_jpeg.len() / 1024,
        start.elapsed()
    );
}
