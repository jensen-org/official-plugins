mod format;
mod ops;
mod raster;

use jensen_plugin_backend::{Backend, Value, fs, json};
use serde::Deserialize;

use format::{Kind, with_extension};
use raster::{Loaded, decode, encode};

const DEFAULT_QUALITY: u8 = 80;

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct PathArgs {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResizeArgs {
    path: String,
    width: Option<u32>,
    height: Option<u32>,
    #[serde(default = "yes")]
    keep_aspect: bool,
    filter: Option<String>,
    out: Option<String>,
    keep_original: Option<bool>,
    #[serde(default)]
    overwrite: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompressArgs {
    path: String,
    quality: Option<u8>,
    out: Option<String>,
    #[serde(default)]
    overwrite: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoundArgs {
    path: String,
    radius: f32,
    #[serde(default)]
    percent: bool,
    out: Option<String>,
    keep_original: Option<bool>,
    #[serde(default)]
    overwrite: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConvertArgs {
    path: String,
    to: String,
    quality: Option<u8>,
    width: Option<u32>,
    out: Option<String>,
    keep_original: Option<bool>,
    #[serde(default)]
    overwrite: bool,
}

fn args<T: serde::de::DeserializeOwned>(input: Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("bad arguments: {error}"))
}

fn open(path: &str, scale: Option<(u32, u32)>) -> Result<Loaded, String> {
    let bytes = fs::read(path)?;
    decode(path, &bytes, scale)
}

fn before(path: &str, loaded: &Loaded) -> Value {
    json!({
        "path": path,
        "format": loaded.kind.name(),
        "width": loaded.image.width(),
        "height": loaded.image.height(),
        "bytes": loaded.bytes,
    })
}

struct Write<'a> {
    replaces_original: bool,
    source_path: &'a str,
    source: &'a Loaded,
    image: &'a image::RgbaImage,
    target: Kind,
    bytes: Vec<u8>,
    out: Option<String>,
    keep_original: Option<bool>,
    overwrite: bool,
}

fn refuse_to_clobber(destination: &str, source: &str, overwrite: bool) -> Result<(), String> {
    if destination != source && !overwrite && fs::read(destination).is_ok() {
        return Err(format!(
            "{destination} already exists: pick another name or allow overwriting"
        ));
    }
    Ok(())
}

fn commit(request: Write) -> Result<Value, String> {
    let explicit = request.out.is_some();
    let destination = request.out.unwrap_or_else(|| {
        if request.target == request.source.kind && !request.source.kind.is_vector() {
            request.source_path.to_string()
        } else {
            with_extension(request.source_path, request.target)
        }
    });
    let moved = destination != request.source_path;
    refuse_to_clobber(&destination, request.source_path, request.overwrite)?;
    fs::write(&destination, &request.bytes)?;
    let keep = request
        .keep_original
        .unwrap_or(explicit || request.source.kind.is_vector() || !request.replaces_original);
    let removed = if moved && !keep {
        fs::remove(request.source_path)?;
        Some(request.source_path.to_string())
    } else {
        None
    };
    Ok(json!({
        "path": destination,
        "format": request.target.name(),
        "width": request.image.width(),
        "height": request.image.height(),
        "bytes": request.bytes.len(),
        "before": before(request.source_path, request.source),
        "removed": removed,
        "unchanged": false,
    }))
}

fn info(input: Value) -> Result<Value, String> {
    let PathArgs { path } = args(input)?;
    let loaded = open(&path, None)?;
    let has_alpha = loaded.image.pixels().any(|pixel| pixel.0[3] != 255);
    let mut report = before(&path, &loaded);
    report["hasAlpha"] = json!(has_alpha);
    Ok(report)
}

fn resize(input: Value) -> Result<Value, String> {
    let request: ResizeArgs = args(input)?;
    let filter = ops::filter_named(request.filter.as_deref())?;
    let source = open(&request.path, None)?;
    let size = ops::target_size(
        source.image.dimensions(),
        request.width,
        request.height,
        request.keep_aspect,
    )?;
    let image = if source.kind.is_vector() {
        open(&request.path, Some(size))?.image
    } else {
        ops::resize(&source.image, size, filter)
    };
    let target = if source.kind.is_vector() {
        Kind::Png
    } else {
        source.kind
    };
    let bytes = encode(&image, target, DEFAULT_QUALITY)?;
    commit(Write {
        replaces_original: false,
        source_path: &request.path,
        source: &source,
        image: &image,
        target,
        bytes,
        out: request.out,
        keep_original: request.keep_original,
        overwrite: request.overwrite,
    })
}

fn compress(input: Value) -> Result<Value, String> {
    let request: CompressArgs = args(input)?;
    let raw = fs::read(&request.path)?;
    let source = decode(&request.path, &raw, None)?;
    let quality = request.quality.unwrap_or(DEFAULT_QUALITY);
    let bytes = if source.kind.is_vector() {
        ops::minify_svg(&String::from_utf8_lossy(&raw)).into_bytes()
    } else {
        encode(&source.image, source.kind, quality)?
    };
    let destination = request.out.unwrap_or_else(|| request.path.clone());
    refuse_to_clobber(&destination, &request.path, request.overwrite)?;
    let smaller = bytes.len() < raw.len();
    if !smaller && destination == request.path {
        return Ok(json!({
            "path": request.path,
            "format": source.kind.name(),
            "width": source.image.width(),
            "height": source.image.height(),
            "bytes": raw.len(),
            "before": before(&request.path, &source),
            "removed": null,
            "unchanged": true,
        }));
    }
    fs::write(&destination, &bytes)?;
    Ok(json!({
        "path": destination,
        "format": source.kind.name(),
        "width": source.image.width(),
        "height": source.image.height(),
        "bytes": bytes.len(),
        "before": before(&request.path, &source),
        "removed": null,
        "unchanged": false,
    }))
}

fn round_corners(input: Value) -> Result<Value, String> {
    let request: RoundArgs = args(input)?;
    if request.radius < 0.0 {
        return Err("the radius cannot be negative".into());
    }
    let source = open(&request.path, None)?;
    let shortest = source.image.width().min(source.image.height()) as f32;
    let radius = if request.percent {
        shortest * request.radius.min(50.0) / 100.0
    } else {
        request.radius
    };
    let mut image = source.image.clone();
    ops::round_corners(&mut image, radius);
    let target = if source.kind.has_alpha() && source.kind != Kind::Gif && !source.kind.is_vector()
    {
        source.kind
    } else {
        Kind::Png
    };
    let bytes = encode(&image, target, DEFAULT_QUALITY)?;
    commit(Write {
        replaces_original: false,
        source_path: &request.path,
        source: &source,
        image: &image,
        target,
        bytes,
        out: request.out,
        keep_original: request.keep_original,
        overwrite: request.overwrite,
    })
}

fn convert(input: Value) -> Result<Value, String> {
    let request: ConvertArgs = args(input)?;
    let target = Kind::parse(&request.to)?;
    if target.is_vector() {
        return Err("a raster image cannot be converted to svg".into());
    }
    let first = open(&request.path, None)?;
    if first.kind == target && request.out.is_none() {
        return Err(format!("{} is already {}", request.path, target.name()));
    }
    let source = match (first.kind.is_vector(), request.width) {
        (true, Some(width)) => {
            let size = ops::target_size(first.image.dimensions(), Some(width), None, true)?;
            open(&request.path, Some(size))?
        }
        _ => first,
    };
    let bytes = encode(
        &source.image,
        target,
        request.quality.unwrap_or(DEFAULT_QUALITY),
    )?;
    commit(Write {
        replaces_original: true,
        source_path: &request.path,
        source: &source,
        image: &source.image,
        target,
        bytes,
        out: request.out,
        keep_original: request.keep_original,
        overwrite: request.overwrite,
    })
}

fn register(backend: &mut Backend) {
    backend
        .method("info", info)
        .method("resize", resize)
        .method("compress", compress)
        .method("round_corners", round_corners)
        .method("convert", convert);
}

jensen_plugin_backend::export!(register);

#[cfg(test)]
mod tests;
