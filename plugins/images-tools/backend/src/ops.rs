use image::imageops::FilterType;
use image::{RgbaImage, imageops};

use crate::raster::check_size;

pub fn filter_named(name: Option<&str>) -> Result<FilterType, String> {
    match name.unwrap_or("lanczos3") {
        "nearest" => Ok(FilterType::Nearest),
        "triangle" | "bilinear" => Ok(FilterType::Triangle),
        "catmullrom" | "bicubic" => Ok(FilterType::CatmullRom),
        "gaussian" => Ok(FilterType::Gaussian),
        "lanczos3" | "lanczos" => Ok(FilterType::Lanczos3),
        other => Err(format!(
            "unknown filter '{other}': use nearest, bilinear, bicubic, gaussian or lanczos3"
        )),
    }
}

pub fn target_size(
    current: (u32, u32),
    width: Option<u32>,
    height: Option<u32>,
    keep_aspect: bool,
) -> Result<(u32, u32), String> {
    let (cw, ch) = (u64::from(current.0), u64::from(current.1));
    let scaled = |value: u64, numerator: u64, denominator: u64| -> u32 {
        ((value * numerator + denominator / 2) / denominator).clamp(1, u64::from(u32::MAX)) as u32
    };
    let (w, h) = match (width, height) {
        (None, None) => return Err("give a width, a height, or both".into()),
        (Some(w), None) => (w, scaled(ch, u64::from(w), cw)),
        (None, Some(h)) => (scaled(cw, u64::from(h), ch), h),
        (Some(w), Some(h)) if keep_aspect => {
            let by_width = scaled(ch, u64::from(w), cw);
            if by_width <= h {
                (w, by_width)
            } else {
                (scaled(cw, u64::from(h), ch), h)
            }
        }
        (Some(w), Some(h)) => (w, h),
    };
    check_size(u64::from(w), u64::from(h))?;
    Ok((w, h))
}

pub fn resize(image: &RgbaImage, size: (u32, u32), filter: FilterType) -> RgbaImage {
    imageops::resize(image, size.0, size.1, filter)
}

pub fn round_corners(image: &mut RgbaImage, radius: f32) {
    let (width, height) = image.dimensions();
    let radius = radius.clamp(0.0, width.min(height) as f32 / 2.0);
    if radius <= 0.0 {
        return;
    }
    let reach = radius.ceil() as u32;
    for dy in 0..reach.min(height) {
        for dx in 0..reach.min(width) {
            let from_x = radius - (dx as f32 + 0.5);
            let from_y = radius - (dy as f32 + 0.5);
            if from_x <= 0.0 || from_y <= 0.0 {
                continue;
            }
            let distance = (from_x * from_x + from_y * from_y).sqrt();
            let coverage = (radius - distance + 0.5).clamp(0.0, 1.0);
            if coverage >= 1.0 {
                continue;
            }
            for (x, y) in [
                (dx, dy),
                (width - 1 - dx, dy),
                (dx, height - 1 - dy),
                (width - 1 - dx, height - 1 - dy),
            ] {
                let pixel = image.get_pixel_mut(x, y);
                pixel.0[3] = (f32::from(pixel.0[3]) * coverage).round() as u8;
            }
        }
    }
}

pub fn minify_svg(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    let mut collapsed = String::with_capacity(out.len());
    for line in out.lines() {
        let line = line.trim();
        if !line.is_empty() {
            if !collapsed.is_empty() && !collapsed.ends_with('>') {
                collapsed.push(' ');
            }
            collapsed.push_str(line);
        }
    }
    collapsed
}
