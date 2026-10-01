//! On-device photo processing (Spec §3 "Client-Side Hardening"): every photo is
//! re-encoded before it is stored, so nothing the camera embedded (EXIF, GPS,
//! maker notes, thumbnails, colour profiles) survives into the outbox.

use std::fmt;
use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbImage};

/// Spec §3: images are resized to 2048 px on the longest edge on-device.
pub const MAX_LONG_EDGE: u32 = 2048;
/// Long edge of the list/grid thumbnail (`<sha>.thumb.jpg`).
pub const THUMB_LONG_EDGE: u32 = 480;
/// Largest source side we will decode; anything bigger is refused rather than
/// risking an out-of-memory kill mid-service.
pub const MAX_SOURCE_SIDE: u32 = 16_384;

const PHOTO_QUALITY: u8 = 85;
const THUMB_QUALITY: u8 = 80;

/// A processed photo: baseline JPEG with no metadata segments, orientation
/// already applied to the pixels.
pub struct Processed {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub thumb_jpeg: Vec<u8>,
}

#[derive(Debug)]
pub struct PhotoError(String);

impl fmt::Display for PhotoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PhotoError {}

impl From<image::ImageError> for PhotoError {
    fn from(e: image::ImageError) -> Self {
        PhotoError(e.to_string())
    }
}

/// Decodes `bytes` (already sniffed as `format`), applies the EXIF orientation,
/// downscales to [`MAX_LONG_EDGE`] and re-encodes. CPU-heavy: call it from a
/// blocking thread.
pub fn process(bytes: &[u8], format: ImageFormat) -> Result<Processed, PhotoError> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_SIDE);
    limits.max_image_height = Some(MAX_SOURCE_SIDE);

    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(format);
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    // A damaged EXIF block must not lose the photo: fall back to "as stored".
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);

    let photo = flatten(fit_within(&image, MAX_LONG_EDGE));
    let thumb = flatten(fit_within(&image, THUMB_LONG_EDGE));
    Ok(Processed {
        width: photo.width(),
        height: photo.height(),
        jpeg: encode_jpeg(&photo, PHOTO_QUALITY)?,
        thumb_jpeg: encode_jpeg(&thumb, THUMB_QUALITY)?,
    })
}

/// Downscale only, preserving aspect ratio. Large reductions use area
/// averaging, which also keeps the float scratch buffer of a filtered resize
/// (4 × f32 per pixel) off a 12–50 MP source; small ones use Catmull-Rom,
/// where area averaging would alias.
fn fit_within(image: &DynamicImage, long_edge: u32) -> DynamicImage {
    let longest = image.width().max(image.height());
    if longest <= long_edge {
        image.clone()
    } else if longest >= long_edge.saturating_mul(3) / 2 {
        image.thumbnail(long_edge, long_edge)
    } else {
        image.resize(
            long_edge,
            long_edge,
            image::imageops::FilterType::CatmullRom,
        )
    }
}

/// JPEG has no alpha: composite transparent pixels onto white instead of
/// letting whatever colour hides behind alpha = 0 show through.
fn flatten(image: DynamicImage) -> RgbImage {
    if !image.color().has_alpha() {
        return image.into_rgb8();
    }
    let rgba = image.into_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let over_white = |c: u8| {
            let (c, a) = (u32::from(c), u32::from(a));
            ((c * a + 255 * (255 - a) + 127) / 255) as u8
        };
        image::Rgb([over_white(r), over_white(g), over_white(b)])
    })
}

/// The encoder writes only JFIF + image data: no EXIF, XMP or ICC segments
/// unless asked, and we never ask.
fn encode_jpeg(image: &RgbImage, quality: u8) -> Result<Vec<u8>, image::ImageError> {
    let mut out = Vec::with_capacity(image.as_raw().len() / 8);
    JpegEncoder::new_with_quality(&mut out, quality).encode_image(image)?;
    Ok(out)
}
