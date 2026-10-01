//! Test fixtures: real encoded images, built in memory so no binary fixtures
//! live in the repo.
#![allow(dead_code)]

use std::io::Cursor;

use image::{ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};

pub const RED: Rgb<u8> = Rgb([220, 20, 20]);
pub const BLUE: Rgb<u8> = Rgb([20, 20, 220]);

/// `w`×`h`, blue, with a red square in the top-left corner so rotations are visible.
pub fn marked(w: u32, h: u32) -> RgbImage {
    let mark = w.min(h) / 3;
    RgbImage::from_fn(w, h, |x, y| if x < mark && y < mark { RED } else { BLUE })
}

pub fn encode(image: &RgbImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, format).unwrap();
    out.into_inner()
}

pub fn jpeg(w: u32, h: u32) -> Vec<u8> {
    encode(&marked(w, h), ImageFormat::Jpeg)
}

/// A PNG whose right half is fully transparent *black*.
pub fn png_with_transparency(w: u32, h: u32) -> Vec<u8> {
    let image = RgbaImage::from_fn(w, h, |x, _| {
        if x < w / 2 {
            Rgba([20, 20, 220, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    });
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

/// Big-endian TIFF/EXIF block as a phone would write it: IFD0 with Make
/// "Pix", Orientation = 6 (rotate 90° clockwise to display) and a GPS IFD
/// holding latitude 41° 52' 30" N.
pub fn exif_app1() -> Vec<u8> {
    fn entry(out: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: [u8; 4]) {
        out.extend_from_slice(&tag.to_be_bytes());
        out.extend_from_slice(&kind.to_be_bytes());
        out.extend_from_slice(&count.to_be_bytes());
        out.extend_from_slice(&value);
    }
    const ASCII: u16 = 2;
    const SHORT: u16 = 3;
    const LONG: u16 = 4;
    const RATIONAL: u16 = 5;
    const BYTE: u16 = 1;
    let ifd0_at: u32 = 8;
    let gps_at: u32 = ifd0_at + 2 + 3 * 12 + 4; // 50
    let lat_at: u32 = gps_at + 2 + 3 * 12 + 4; // 92

    let mut tiff = b"MM\x00\x2A".to_vec();
    tiff.extend_from_slice(&ifd0_at.to_be_bytes());
    tiff.extend_from_slice(&3u16.to_be_bytes());
    entry(&mut tiff, 0x010F, ASCII, 4, *b"Pix\0"); // Make
    entry(&mut tiff, 0x0112, SHORT, 1, [0, 6, 0, 0]); // Orientation
    entry(&mut tiff, 0x8825, LONG, 1, gps_at.to_be_bytes()); // GPS IFD pointer
    tiff.extend_from_slice(&0u32.to_be_bytes());
    assert_eq!(tiff.len() as u32, gps_at);

    tiff.extend_from_slice(&3u16.to_be_bytes());
    entry(&mut tiff, 0x0000, BYTE, 4, [2, 2, 0, 0]); // GPSVersionID
    entry(&mut tiff, 0x0001, ASCII, 2, *b"N\0\0\0"); // GPSLatitudeRef
    entry(&mut tiff, 0x0002, RATIONAL, 3, lat_at.to_be_bytes()); // GPSLatitude
    tiff.extend_from_slice(&0u32.to_be_bytes());
    assert_eq!(tiff.len() as u32, lat_at);
    for (num, den) in [(41u32, 1u32), (52, 1), (3000, 100)] {
        tiff.extend_from_slice(&num.to_be_bytes());
        tiff.extend_from_slice(&den.to_be_bytes());
    }

    let mut segment = vec![0xFF, 0xE1];
    segment.extend_from_slice(&((2 + 6 + tiff.len()) as u16).to_be_bytes());
    segment.extend_from_slice(b"Exif\0\0");
    segment.extend_from_slice(&tiff);
    segment
}

/// Inserts `segment` right after SOI.
pub fn with_segment(jpeg: &[u8], segment: &[u8]) -> Vec<u8> {
    assert_eq!(&jpeg[..2], [0xFF, 0xD8]);
    [&jpeg[..2], segment, &jpeg[2..]].concat()
}

/// Marker bytes of every segment before the scan data (SOS).
pub fn header_markers(jpeg: &[u8]) -> Vec<u8> {
    assert_eq!(&jpeg[..2], [0xFF, 0xD8], "not a JPEG");
    let mut markers = Vec::new();
    let mut at = 2;
    loop {
        assert_eq!(jpeg[at], 0xFF, "lost sync at byte {at}");
        let marker = jpeg[at + 1];
        markers.push(marker);
        if marker == 0xDA {
            return markers;
        }
        let len = u16::from_be_bytes([jpeg[at + 2], jpeg[at + 3]]) as usize;
        at += 2 + len;
    }
}

pub fn near(actual: Rgb<u8>, expected: Rgb<u8>) -> bool {
    actual
        .0
        .iter()
        .zip(expected.0)
        .all(|(&a, e)| a.abs_diff(e) <= 40)
}
