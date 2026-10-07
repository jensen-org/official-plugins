#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Png,
    Jpeg,
    Gif,
    Webp,
    Bmp,
    Ico,
    Tiff,
    Svg,
}

impl Kind {
    pub fn parse(name: &str) -> Result<Kind, String> {
        match name.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "png" => Ok(Kind::Png),
            "jpg" | "jpeg" => Ok(Kind::Jpeg),
            "gif" => Ok(Kind::Gif),
            "webp" => Ok(Kind::Webp),
            "bmp" => Ok(Kind::Bmp),
            "ico" => Ok(Kind::Ico),
            "tif" | "tiff" => Ok(Kind::Tiff),
            "svg" => Ok(Kind::Svg),
            other => Err(format!(
                "'{other}' is not a format this plugin handles: use png, jpeg, webp, gif, bmp, ico, tiff or svg"
            )),
        }
    }

    pub fn from_path(path: &str) -> Option<Kind> {
        let name = path.rsplit('/').next().unwrap_or(path);
        let dot = name.rfind('.').filter(|dot| *dot > 0)?;
        Kind::parse(&name[dot + 1..]).ok()
    }

    pub fn extension(self) -> &'static str {
        match self {
            Kind::Png => "png",
            Kind::Jpeg => "jpg",
            Kind::Gif => "gif",
            Kind::Webp => "webp",
            Kind::Bmp => "bmp",
            Kind::Ico => "ico",
            Kind::Tiff => "tiff",
            Kind::Svg => "svg",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Jpeg => "jpeg",
            other => other.extension(),
        }
    }

    pub fn is_vector(self) -> bool {
        self == Kind::Svg
    }

    pub fn has_alpha(self) -> bool {
        !matches!(self, Kind::Jpeg | Kind::Bmp)
    }
}

pub fn with_extension(path: &str, kind: Kind) -> String {
    let (folder, name) = match path.rfind('/') {
        Some(at) => (&path[..=at], &path[at + 1..]),
        None => ("", path),
    };
    let stem = match name.rfind('.').filter(|dot| *dot > 0) {
        Some(dot) => &name[..dot],
        None => name,
    };
    format!("{folder}{stem}.{}", kind.extension())
}

pub fn sniff(bytes: &[u8]) -> Option<Kind> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some(Kind::Png);
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(Kind::Jpeg);
    }
    if bytes.starts_with(b"GIF8") {
        return Some(Kind::Gif);
    }
    if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(Kind::Webp);
    }
    if bytes.starts_with(b"BM") {
        return Some(Kind::Bmp);
    }
    if bytes.starts_with(&[0, 0, 1, 0]) {
        return Some(Kind::Ico);
    }
    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        return Some(Kind::Tiff);
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
    head.contains("<svg").then_some(Kind::Svg)
}
