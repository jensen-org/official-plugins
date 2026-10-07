#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use image::{ImageEncoder, Rgba, RgbaImage};
use jensen_plugin_backend::{Value, json};

use super::*;

static NEXT: AtomicU32 = AtomicU32::new(0);

fn workdir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "images-tools-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn at(dir: &std::path::Path, name: &str) -> String {
    dir.join(name).to_str().unwrap().to_string()
}

fn gradient(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        Rgba([
            (x * 255 / width.max(1)) as u8,
            (y * 255 / height.max(1)) as u8,
            128,
            255,
        ])
    })
}

fn flat(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_pixel(width, height, Rgba([200, 30, 30, 255]))
}

fn save(path: &str, image: &RgbaImage, kind: Kind) {
    std::fs::write(path, encode(image, kind, 90).unwrap()).unwrap();
}

fn size_of(path: &str) -> (u32, u32) {
    let loaded = open(path, None).unwrap();
    loaded.image.dimensions()
}

const SVG: &str = r##"<?xml version="1.0"?>
<!-- a comment that should disappear -->
<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20">
  <rect width="40" height="20" fill="#336699"/>
</svg>
"##;

#[test]
fn info_reports_format_size_and_alpha() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    let mut image = flat(8, 4);
    image.put_pixel(0, 0, Rgba([0, 0, 0, 0]));
    save(&path, &image, Kind::Png);

    let report = info(json!({ "path": path })).unwrap();

    assert_eq!(report["format"], "png");
    assert_eq!(report["width"], 8);
    assert_eq!(report["height"], 4);
    assert_eq!(report["hasAlpha"], true);
}

#[test]
fn resize_by_width_keeps_the_aspect_ratio_and_rewrites_the_file_in_place() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    save(&path, &gradient(100, 50), Kind::Png);

    let report = resize(json!({ "path": path, "width": 40 })).unwrap();

    assert_eq!(report["path"], json!(path));
    assert_eq!(
        (report["width"].clone(), report["height"].clone()),
        (json!(40), json!(20))
    );
    assert_eq!(size_of(&path), (40, 20));
    assert_eq!(report["before"]["width"], 100);
}

#[test]
fn resize_into_a_box_fits_inside_it_unless_the_aspect_is_released() {
    let dir = workdir();
    let path = at(&dir, "a.jpg");
    save(&path, &gradient(100, 50), Kind::Jpeg);

    resize(json!({ "path": path, "width": 30, "height": 30 })).unwrap();
    assert_eq!(size_of(&path), (30, 15));

    resize(json!({ "path": path, "width": 30, "height": 30, "keepAspect": false })).unwrap();
    assert_eq!(size_of(&path), (30, 30));
}

#[test]
fn resize_needs_a_size_and_refuses_absurd_ones() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    save(&path, &flat(4, 4), Kind::Png);

    assert!(resize(json!({ "path": path })).is_err());
    assert!(resize(json!({ "path": path, "width": 60000, "height": 60000 })).is_err());
    assert_eq!(size_of(&path), (4, 4));
}

#[test]
fn resizing_an_svg_draws_it_sharp_at_the_new_size_and_keeps_the_vector() {
    let dir = workdir();
    let path = at(&dir, "logo.svg");
    std::fs::write(&path, SVG).unwrap();

    let report = resize(json!({ "path": path, "width": 160 })).unwrap();

    let png = at(&dir, "logo.png");
    assert_eq!(report["path"], json!(png));
    assert_eq!(report["removed"], Value::Null);
    assert_eq!(size_of(&png), (160, 80));
    assert!(std::path::Path::new(&path).exists());
}

#[test]
fn rounding_makes_the_corners_transparent_and_leaves_the_middle_alone() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    save(&path, &flat(40, 40), Kind::Png);

    round_corners(json!({ "path": path, "radius": 10 })).unwrap();

    let image = open(&path, None).unwrap().image;
    assert_eq!(image.get_pixel(0, 0).0[3], 0);
    assert_eq!(image.get_pixel(39, 39).0[3], 0);
    assert_eq!(image.get_pixel(20, 20).0, [200, 30, 30, 255]);
    assert_eq!(image.get_pixel(20, 0).0[3], 255);
}

#[test]
fn a_full_radius_makes_a_circle_and_a_percent_is_of_the_shortest_side() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    save(&path, &flat(40, 40), Kind::Png);

    round_corners(json!({ "path": path, "radius": 50, "percent": true })).unwrap();

    let image = open(&path, None).unwrap().image;
    assert_eq!(image.get_pixel(1, 1).0[3], 0);
    assert_eq!(image.get_pixel(20, 20).0[3], 255);
}

#[test]
fn rounding_a_jpeg_writes_a_png_next_to_it_and_keeps_the_jpeg() {
    let dir = workdir();
    let path = at(&dir, "a.jpg");
    save(&path, &flat(40, 40), Kind::Jpeg);

    let report = round_corners(json!({ "path": path, "radius": 8 })).unwrap();

    assert_eq!(report["path"], json!(at(&dir, "a.png")));
    assert_eq!(report["removed"], Value::Null);
    assert!(std::path::Path::new(&path).exists());
    assert_eq!(
        open(&at(&dir, "a.png"), None)
            .unwrap()
            .image
            .get_pixel(0, 0)
            .0[3],
        0
    );
}

#[test]
fn rounding_a_jpeg_can_replace_it_when_asked() {
    let dir = workdir();
    let path = at(&dir, "a.jpg");
    save(&path, &flat(40, 40), Kind::Jpeg);

    let report =
        round_corners(json!({ "path": path, "radius": 8, "keepOriginal": false })).unwrap();

    assert_eq!(report["removed"], json!(path));
    assert!(!std::path::Path::new(&path).exists());
}

#[test]
fn writing_to_an_explicit_path_never_deletes_the_source() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    let copy = at(&dir, "small.png");
    save(&path, &gradient(40, 40), Kind::Png);

    let resized = resize(json!({ "path": path, "width": 10, "out": copy })).unwrap();
    assert_eq!(resized["removed"], Value::Null);
    assert_eq!(size_of(&path), (40, 40));
    assert_eq!(size_of(&copy), (10, 10));

    let converted =
        convert(json!({ "path": path, "to": "webp", "out": at(&dir, "b.webp") })).unwrap();
    assert_eq!(converted["removed"], Value::Null);
    assert!(std::path::Path::new(&path).exists());
}

#[test]
fn compressing_into_another_path_will_not_overwrite_a_file_that_is_there() {
    let dir = workdir();
    let path = at(&dir, "a.jpg");
    let target = at(&dir, "keep.jpg");
    std::fs::write(&path, encode(&gradient(96, 96), Kind::Jpeg, 100).unwrap()).unwrap();
    std::fs::write(&target, b"someone else's file").unwrap();

    let error = compress(json!({ "path": path, "quality": 30, "out": target })).unwrap_err();

    assert!(error.contains("already exists"));
    assert_eq!(std::fs::read(&target).unwrap(), b"someone else's file");

    compress(json!({ "path": path, "quality": 30, "out": target, "overwrite": true })).unwrap();
    assert_ne!(std::fs::read(&target).unwrap(), b"someone else's file");
}

#[test]
fn compressing_a_flat_png_shrinks_it_and_keeps_every_pixel() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    let image = flat(64, 64);
    let loose = {
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new_with_quality(
            &mut out,
            image::codecs::png::CompressionType::Fast,
            image::codecs::png::FilterType::NoFilter,
        )
        .write_image(image.as_raw(), 64, 64, image::ExtendedColorType::Rgba8)
        .unwrap();
        out
    };
    std::fs::write(&path, &loose).unwrap();

    let report = compress(json!({ "path": path })).unwrap();

    assert_eq!(report["unchanged"], false);
    assert!(report["bytes"].as_u64().unwrap() < loose.len() as u64);
    assert_eq!(open(&path, None).unwrap().image, image);
}

#[test]
fn compressing_a_jpeg_lowers_the_size_with_the_quality() {
    let dir = workdir();
    let path = at(&dir, "a.jpg");
    std::fs::write(&path, encode(&gradient(96, 96), Kind::Jpeg, 100).unwrap()).unwrap();
    let before = std::fs::metadata(&path).unwrap().len();

    let report = compress(json!({ "path": path, "quality": 30 })).unwrap();

    assert!(report["bytes"].as_u64().unwrap() < before);
    assert_eq!(
        std::fs::metadata(&path).unwrap().len(),
        report["bytes"].as_u64().unwrap()
    );
}

#[test]
fn compressing_what_cannot_shrink_leaves_the_file_untouched() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    save(&path, &flat(4, 4), Kind::Png);
    let bytes = std::fs::read(&path).unwrap();

    let report = compress(json!({ "path": path })).unwrap();

    assert_eq!(report["unchanged"], true);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn compressing_an_svg_strips_comments_and_whitespace() {
    let dir = workdir();
    let path = at(&dir, "a.svg");
    std::fs::write(&path, SVG).unwrap();

    let report = compress(json!({ "path": path })).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(report["unchanged"], false);
    assert!(!text.contains("<!--"));
    assert!(text.contains("<rect"));
    assert!(text.len() < SVG.len());
}

#[test]
fn converting_changes_the_extension_and_removes_the_original() {
    let dir = workdir();
    let path = at(&dir, "photo.png");
    save(&path, &gradient(32, 32), Kind::Png);

    let report = convert(json!({ "path": path, "to": "jpg" })).unwrap();

    assert_eq!(report["path"], json!(at(&dir, "photo.jpg")));
    assert_eq!(report["format"], "jpeg");
    assert_eq!(report["removed"], json!(path));
    assert!(!std::path::Path::new(&path).exists());
    assert_eq!(size_of(&at(&dir, "photo.jpg")), (32, 32));
}

#[test]
fn every_raster_format_converts_to_every_other_and_decodes_again() {
    let dir = workdir();
    let source = at(&dir, "a.png");
    save(&source, &gradient(24, 24), Kind::Png);

    for target in ["jpeg", "webp", "gif", "bmp", "ico", "tiff", "png"] {
        let from = at(&dir, "a.png");
        let report =
            convert(json!({ "path": from, "to": target, "keepOriginal": true, "overwrite": true }));
        if target == "png" {
            assert!(report.is_err(), "converting to the same format is refused");
            continue;
        }
        let report = report.unwrap_or_else(|error| panic!("{target}: {error}"));
        let written = report["path"].as_str().unwrap().to_string();
        assert_eq!(
            size_of(&written),
            (24, 24),
            "{target} decodes back to the same size"
        );
    }
}

#[test]
fn converting_keeps_transparency_where_the_target_allows_it_and_flattens_on_white_where_not() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    let mut image = flat(8, 8);
    image.put_pixel(0, 0, Rgba([0, 0, 0, 0]));
    save(&path, &image, Kind::Png);

    convert(json!({ "path": path, "to": "webp", "keepOriginal": true })).unwrap();
    convert(json!({ "path": path, "to": "jpg", "keepOriginal": true })).unwrap();

    assert_eq!(
        open(&at(&dir, "a.webp"), None)
            .unwrap()
            .image
            .get_pixel(0, 0)
            .0[3],
        0
    );
    let jpeg = open(&at(&dir, "a.jpg"), None).unwrap().image;
    assert!(
        jpeg.get_pixel(0, 0).0[0] > 240,
        "a transparent pixel becomes white in a jpeg"
    );
}

#[test]
fn converting_never_overwrites_another_file_unless_told_to() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    save(&path, &flat(8, 8), Kind::Png);
    std::fs::write(at(&dir, "a.jpg"), b"someone else's file").unwrap();

    let error = convert(json!({ "path": path, "to": "jpg" })).unwrap_err();

    assert!(error.contains("already exists"));
    assert_eq!(
        std::fs::read(at(&dir, "a.jpg")).unwrap(),
        b"someone else's file"
    );
    assert!(std::path::Path::new(&path).exists());
    convert(json!({ "path": path, "to": "jpg", "overwrite": true })).unwrap();
    assert!(!std::path::Path::new(&path).exists());
}

#[test]
fn an_svg_converts_to_a_raster_and_keeps_the_vector_but_a_raster_cannot_become_an_svg() {
    let dir = workdir();
    let path = at(&dir, "logo.svg");
    std::fs::write(&path, SVG).unwrap();

    let report = convert(json!({ "path": path, "to": "webp", "width": 80 })).unwrap();

    assert_eq!(report["path"], json!(at(&dir, "logo.webp")));
    assert_eq!(size_of(&at(&dir, "logo.webp")), (80, 40));
    assert!(std::path::Path::new(&path).exists());
    assert!(convert(json!({ "path": at(&dir, "logo.webp"), "to": "svg" })).is_err());
}

#[test]
fn an_ico_larger_than_256_is_refused_with_the_way_forward() {
    let dir = workdir();
    let path = at(&dir, "a.png");
    save(&path, &flat(300, 300), Kind::Png);

    let error = convert(json!({ "path": path, "to": "ico" })).unwrap_err();

    assert!(error.contains("resize first"));
}

#[test]
fn a_file_that_is_not_an_image_is_refused_clearly() {
    let dir = workdir();
    let path = at(&dir, "notes.png");
    std::fs::write(&path, b"plain text").unwrap();

    assert!(
        info(json!({ "path": path }))
            .unwrap_err()
            .contains("could not be decoded")
    );
    assert!(info(json!({ "path": at(&dir, "missing.png") })).is_err());
    assert!(info(json!({})).unwrap_err().contains("bad arguments"));
}
