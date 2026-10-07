use std::io::Cursor;

use image::codecs::bmp::BmpEncoder;
use image::codecs::gif::GifEncoder;
use image::codecs::ico::IcoEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::tiff::TiffEncoder;
use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageFormat, RgbaImage};

use crate::format::{Kind, sniff};

pub const MAX_PIXELS: u64 = 120_000_000;

pub struct Loaded {
    pub image: RgbaImage,
    pub kind: Kind,
    pub bytes: usize,
}

pub fn decode(path: &str, bytes: &[u8], scale: Option<(u32, u32)>) -> Result<Loaded, String> {
    let kind = Kind::from_path(path)
        .or_else(|| sniff(bytes))
        .ok_or_else(|| format!("{path} is not an image this plugin can read"))?;
    let image = if kind.is_vector() {
        render_svg(bytes, scale)?
    } else {
        let format = match kind {
            Kind::Png => ImageFormat::Png,
            Kind::Jpeg => ImageFormat::Jpeg,
            Kind::Gif => ImageFormat::Gif,
            Kind::Webp => ImageFormat::WebP,
            Kind::Bmp => ImageFormat::Bmp,
            Kind::Ico => ImageFormat::Ico,
            Kind::Tiff => ImageFormat::Tiff,
            Kind::Svg => return Err("svg is decoded by render_svg".into()),
        };
        image::load_from_memory_with_format(bytes, format)
            .map_err(|error| format!("{path} could not be decoded: {error}"))?
            .to_rgba8()
    };
    Ok(Loaded {
        image,
        kind,
        bytes: bytes.len(),
    })
}

pub fn check_size(width: u64, height: u64) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("an image cannot be zero pixels wide or tall".into());
    }
    if width * height > MAX_PIXELS {
        return Err(format!(
            "{width} x {height} is larger than the {MAX_PIXELS} pixel limit"
        ));
    }
    Ok(())
}

fn render_svg(bytes: &[u8], scale: Option<(u32, u32)>) -> Result<RgbaImage, String> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(bytes, &options)
        .map_err(|error| format!("the svg could not be read: {error}"))?;
    let size = tree.size();
    let (width, height) = match scale {
        Some((width, height)) => (width, height),
        None => (size.width().ceil() as u32, size.height().ceil() as u32),
    };
    check_size(u64::from(width), u64::from(height))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "the svg is too large to draw".to_string())?;
    let transform = resvg::tiny_skia::Transform::from_scale(
        width as f32 / size.width(),
        height as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut raw = Vec::with_capacity(pixmap.pixels().len() * 4);
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        raw.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    RgbaImage::from_raw(width, height, raw).ok_or_else(|| "the svg raster is malformed".into())
}

fn is_opaque(image: &RgbaImage) -> bool {
    image.pixels().all(|pixel| pixel.0[3] == 255)
}

fn flatten_on_white(image: &RgbaImage) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(image.pixels().len() * 3);
    for pixel in image.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        let alpha = u32::from(alpha);
        for channel in [red, green, blue] {
            let blended = (u32::from(channel) * alpha + 255 * (255 - alpha) + 127) / 255;
            rgb.push(blended as u8);
        }
    }
    rgb
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let (width, height) = image.dimensions();
    let mut palette: Vec<[u8; 4]> = Vec::new();
    let mut indices: Vec<u8> = Vec::with_capacity((width * height) as usize);
    let mut indexed = true;
    for pixel in image.pixels() {
        let color = pixel.0;
        match palette.iter().position(|known| *known == color) {
            Some(at) => indices.push(at as u8),
            None if palette.len() < 256 => {
                indices.push(palette.len() as u8);
                palette.push(color);
            }
            None => {
                indexed = false;
                break;
            }
        }
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_compression(png::Compression::Best);
        encoder.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);
        let data = if indexed {
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(
                palette
                    .iter()
                    .flat_map(|c| [c[0], c[1], c[2]])
                    .collect::<Vec<u8>>(),
            );
            if palette.iter().any(|c| c[3] != 255) {
                encoder.set_trns(palette.iter().map(|c| c[3]).collect::<Vec<u8>>());
            }
            indices
        } else if is_opaque(image) {
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            image
                .pixels()
                .flat_map(|p| [p.0[0], p.0[1], p.0[2]])
                .collect()
        } else {
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            image.as_raw().clone()
        };
        let mut writer = encoder
            .write_header()
            .map_err(|error| format!("png header: {error}"))?;
        writer
            .write_image_data(&data)
            .map_err(|error| format!("png data: {error}"))?;
    }
    Ok(out)
}

pub fn encode(image: &RgbaImage, kind: Kind, quality: u8) -> Result<Vec<u8>, String> {
    let (width, height) = image.dimensions();
    let mut out = Cursor::new(Vec::new());
    let fail = |error: image::ImageError| format!("encoding {} failed: {error}", kind.name());
    match kind {
        Kind::Png => return encode_png(image),
        Kind::Jpeg => JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100))
            .write_image(
                &flatten_on_white(image),
                width,
                height,
                ExtendedColorType::Rgb8,
            )
            .map_err(fail)?,
        Kind::Webp => WebPEncoder::new_lossless(&mut out)
            .write_image(image.as_raw(), width, height, ExtendedColorType::Rgba8)
            .map_err(fail)?,
        Kind::Gif => GifEncoder::new(&mut out)
            .encode(image.as_raw(), width, height, ExtendedColorType::Rgba8)
            .map_err(fail)?,
        Kind::Bmp => BmpEncoder::new(&mut out)
            .write_image(image.as_raw(), width, height, ExtendedColorType::Rgba8)
            .map_err(fail)?,
        Kind::Ico => {
            if width > 256 || height > 256 {
                return Err("an ico holds at most 256 x 256 pixels: resize first".into());
            }
            IcoEncoder::new(&mut out)
                .write_image(image.as_raw(), width, height, ExtendedColorType::Rgba8)
                .map_err(fail)?
        }
        Kind::Tiff => TiffEncoder::new(&mut out)
            .write_image(image.as_raw(), width, height, ExtendedColorType::Rgba8)
            .map_err(fail)?,
        Kind::Svg => return Err("a raster image cannot be written as svg".into()),
    }
    Ok(out.into_inner())
}
