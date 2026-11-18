//! Images embedded in copied HTML (`data:` in Thunderbird/Betterbird, `file:` temp files in
//! Outlook and Word), saved to a folder so the Markdown can point at them.
use base64::Engine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// Saved images older than this are deleted the next time an image is saved.
const RETENTION: Duration = Duration::from_secs(24 * 3600);
/// Larger payloads are not images worth keeping from a mail selection.
const MAX_BYTES: usize = 25 * 1024 * 1024;

pub struct ImageSaver {
    dir: PathBuf,
    stamp: String,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    count: usize,
    /// Same image twice in one selection (a repeated logo): one file.
    by_hash: HashMap<u64, PathBuf>,
    cleaned: bool,
}

impl ImageSaver {
    /// Files are named after the conversion time (UTC), so one copy's images sort together.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, stamp: timestamp(SystemTime::now()), state: Mutex::default() }
    }

    /// Writes the image behind `src` and returns its path, or `None` if `src` is not a
    /// readable `data:` or `file:` image.
    pub fn save(&self, src: &str) -> Option<PathBuf> {
        let (bytes, ext) = read_image(src)?;
        let mut state = self.state.lock().ok()?;
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut h);
            h.finish()
        };
        if let Some(path) = state.by_hash.get(&hash) {
            return Some(path.clone());
        }
        std::fs::create_dir_all(&self.dir).ok()?;
        if !state.cleaned {
            state.cleaned = true;
            remove_old(&self.dir, SystemTime::now());
        }
        state.count += 1;
        let path = self.dir.join(format!("{}-{}.{ext}", self.stamp, state.count));
        std::fs::write(&path, &bytes).ok()?;
        state.by_hash.insert(hash, path.clone());
        Some(path)
    }
}

fn read_image(src: &str) -> Option<(Vec<u8>, &'static str)> {
    let src = src.trim();
    let scheme = src.split_once(':').map(|(s, _)| s.to_ascii_lowercase())?;
    match scheme.as_str() {
        "data" => decode_data_uri(src),
        "file" => {
            let path = file_uri_path(src)?;
            let ext = image_extension(path.extension()?.to_str()?)?;
            let len = std::fs::metadata(&path).ok()?.len();
            if len as usize > MAX_BYTES {
                return None;
            }
            Some((std::fs::read(&path).ok()?, ext))
        }
        _ => None,
    }
}

/// `data:image/png;base64,....` only: other encodings do not occur for mail images.
fn decode_data_uri(src: &str) -> Option<(Vec<u8>, &'static str)> {
    let (meta, payload) = src.get(5..)?.split_once(',')?;
    let mut parts = meta.split(';');
    let mime = parts.next()?.trim().to_ascii_lowercase();
    if !parts.any(|p| p.trim().eq_ignore_ascii_case("base64")) {
        return None;
    }
    let ext = image_extension(mime.strip_prefix("image/")?)?;
    let clean: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.len() / 4 * 3 > MAX_BYTES {
        return None;
    }
    let engine = base64::engine::GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        base64::engine::GeneralPurposeConfig::new()
            .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent)
            .with_decode_allow_trailing_bits(true),
    );
    let bytes = engine.decode(clean.as_bytes()).ok()?;
    (!bytes.is_empty()).then_some((bytes, ext))
}

fn image_extension(kind: &str) -> Option<&'static str> {
    Some(match kind.to_ascii_lowercase().as_str() {
        "png" => "png",
        "jpeg" | "jpg" | "pjpeg" => "jpg",
        "gif" => "gif",
        "webp" => "webp",
        "bmp" => "bmp",
        "svg+xml" | "svg" => "svg",
        _ => return None,
    })
}

/// `file:///C:/Users/x/a%20b.png` -> `C:/Users/x/a b.png`; `file:///home/x/a.png` -> `/home/x/a.png`.
fn file_uri_path(src: &str) -> Option<PathBuf> {
    let rest = src.get(5..)?;
    let rest = rest.strip_prefix("//").unwrap_or(rest);
    // Skip an authority such as `localhost`; a Windows drive keeps its colon.
    let rest = match rest.find('/') {
        Some(0) => rest,
        Some(i) if !rest[..i].contains(':') => &rest[i..],
        _ => rest,
    };
    let decoded = percent_decode(rest)?;
    let b = decoded.as_bytes();
    let path = if b.len() > 2 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' { &decoded[1..] } else { &decoded };
    Some(PathBuf::from(path))
}

fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hex = [bytes.next()?, bytes.next()?];
            out.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            out.push(b);
        }
    }
    String::from_utf8(out).ok()
}

/// `YYYYMMDD-HHMMSS` in UTC.
fn timestamp(t: SystemTime) -> String {
    let secs = t.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Only files named by this module (`YYYYMMDD-HHMMSS-N.ext`) are candidates, so anything else
/// the user put in the folder stays.
fn is_ours(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() > 17
        && b[..8].iter().all(u8::is_ascii_digit)
        && b[8] == b'-'
        && b[9..15].iter().all(u8::is_ascii_digit)
        && b[15] == b'-'
        && b[16].is_ascii_digit()
}

fn remove_old(dir: &Path, now: SystemTime) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_str().is_some_and(is_ours) {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|age| age > RETENTION);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1x1 transparent PNG.
    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("clip-md-html-img-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn saves_data_uri_once_per_content() {
        let dir = scratch("data");
        let saver = ImageSaver::new(dir.clone());
        let src = format!("data:image/png;base64,{}\r\n{}", &PNG[..20], &PNG[20..]);
        let a = saver.save(&src).unwrap();
        let b = saver.save(&format!("data:image/png;base64,{PNG}")).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.extension().unwrap(), "png");
        assert!(is_ours(a.file_name().unwrap().to_str().unwrap()));
        assert_eq!(&std::fs::read(&a).unwrap()[1..4], b"PNG");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn copies_file_uri_images() {
        let dir = scratch("file");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("clip image001.png");
        std::fs::write(&source, b"\x89PNG fake").unwrap();
        let uri = format!("file:///{}", source.to_str().unwrap().replace('\\', "/").trim_start_matches('/').replace(' ', "%20"));
        let saver = ImageSaver::new(dir.join("out"));
        let saved = saver.save(&uri).unwrap();
        assert_eq!(std::fs::read(saved).unwrap(), b"\x89PNG fake");
        assert!(saver.save("file:///nowhere/missing.png").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_what_is_not_an_image() {
        let saver = ImageSaver::new(scratch("none"));
        assert!(saver.save("cid:image001.png@01DC").is_none());
        assert!(saver.save("data:text/plain;base64,aGVsbG8=").is_none());
        assert!(saver.save("data:image/png,rawdata").is_none());
        assert!(saver.save("data:image/png;base64,***").is_none());
        assert!(saver.save("https://example.com/a.png").is_none());
    }

    #[test]
    fn file_uri_forms() {
        assert_eq!(file_uri_path("file:///C:/a%20b/c.png").unwrap(), PathBuf::from("C:/a b/c.png"));
        assert_eq!(file_uri_path("file://localhost/C:/x.png").unwrap(), PathBuf::from("C:/x.png"));
        assert_eq!(file_uri_path("file:///home/u/x.png").unwrap(), PathBuf::from("/home/u/x.png"));
    }

    #[test]
    fn timestamp_is_utc_civil_time() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_795_018_216);
        assert_eq!(timestamp(t), "20261118-161016");
        assert_eq!(timestamp(SystemTime::UNIX_EPOCH + Duration::from_secs(951_782_400)), "20000229-000000");
    }

    #[test]
    fn cleanup_only_touches_old_files_it_named() {
        let dir = scratch("clean");
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["20260101-000000-1.png", "mine.png"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        remove_old(&dir, SystemTime::now() + RETENTION + Duration::from_secs(60));
        assert!(!dir.join("20260101-000000-1.png").exists());
        assert!(dir.join("mine.png").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
