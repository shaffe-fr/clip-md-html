//! Where the Markdown should point for an image of copied HTML: images embedded in the copy
//! (`data:`) are saved to a folder, and image files of this machine named in the address
//! (`file:`, an editor's Markdown preview, Obsidian) are linked where they are.
//!
//! The HTML may come from any web page or mail, so it is untrusted: only files with an image
//! extension are ever linked or read (a page could otherwise name `~/.aws/credentials` and have
//! an agent open it from the pasted Markdown), no network path is probed, and the work and the
//! disk space one conversion can use are capped.
use base64::Engine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// Saved images older than this are deleted the next time an image is saved.
const RETENTION: Duration = Duration::from_secs(24 * 3600);
/// Larger payloads are not images worth keeping from a mail selection.
const MAX_BYTES: usize = 25 * 1024 * 1024;
/// Saved by one conversion, all images together.
const MAX_TOTAL_BYTES: usize = 100 * 1024 * 1024;
/// Images looked at by one conversion; the rest stay `[image]` or web links.
const MAX_IMAGES: usize = 200;
/// Longer non-`data:` addresses are not real image addresses.
const MAX_ADDRESS: usize = 4096;

pub struct ImageSaver {
    dir: PathBuf,
    stamp: String,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    count: usize,
    attempts: usize,
    stored_bytes: usize,
    /// Same image twice in one selection (a repeated logo): one file.
    by_hash: HashMap<u64, PathBuf>,
    cleaned: bool,
}

impl ImageSaver {
    /// Files are named after the conversion time (UTC), so one copy's images sort together.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, stamp: timestamp(SystemTime::now()), state: Mutex::default() }
    }

    /// Path to link for the image behind `src`, or `None` when it cannot be read from this machine.
    ///
    /// A local file is linked as is, unless it lives in the temp directory (Outlook and Word copy
    /// images there and delete them later): then it is copied to the folder like a `data:` image.
    pub fn resolve(&self, src: &str) -> Option<PathBuf> {
        {
            let mut state = self.state.lock().ok()?;
            state.attempts += 1;
            if state.attempts > MAX_IMAGES {
                return None;
            }
        }
        let src = src.trim();
        if src.get(..5).is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:")) {
            let (bytes, ext) = decode_data_uri(src)?;
            return self.store(&bytes, ext);
        }
        if src.len() > MAX_ADDRESS {
            return None;
        }
        let path = local_file(src)?;
        if !is_temporary(&path) {
            return Some(path);
        }
        let ext = image_extension(path.extension()?.to_str()?)?;
        if std::fs::metadata(&path).ok()?.len() as usize > MAX_BYTES {
            return None;
        }
        self.store(&std::fs::read(&path).ok()?, ext)
    }

    fn store(&self, bytes: &[u8], ext: &str) -> Option<PathBuf> {
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
        if state.stored_bytes + bytes.len() > MAX_TOTAL_BYTES {
            return None;
        }
        std::fs::create_dir_all(&self.dir).ok()?;
        if !state.cleaned {
            state.cleaned = true;
            remove_old(&self.dir, SystemTime::now());
        }
        state.count += 1;
        let path = self.dir.join(format!("{}-{}.{ext}", self.stamp, state.count));
        std::fs::write(&path, bytes).ok()?;
        state.stored_bytes += bytes.len();
        state.by_hash.insert(hash, path.clone());
        Some(path)
    }
}

/// The first existing image file among the paths found in the address. Requiring an image
/// extension and an existing file is what makes the guessing safe: a wrong guess would have to
/// be a real image on this disk.
fn local_file(src: &str) -> Option<PathBuf> {
    candidates(src).into_iter().filter(|p| has_image_extension(p)).find(|p| p.is_file())
}

fn has_image_extension(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).and_then(image_extension).is_some()
}

/// Paths an image address may stand for, most specific first. Query and fragment are dropped
/// (Obsidian appends `?<timestamp>`), then the address is percent-decoded:
/// - Windows: a drive (`C:/`) at the start or right after a `/`, whatever the scheme:
///   `file:///C:/a.png`, `https://file+.vscode-resource.vscode-cdn.net/c%3A/a.png`,
///   `app://<id>/C:/a.png`. Never a network path (`\\server\share`), which would send the
///   Windows credentials to whoever the page names.
/// - Elsewhere: every absolute tail of the address (`/a/b/c.png`, `/b/c.png`, ...).
fn candidates(src: &str) -> Vec<PathBuf> {
    let address = src.split(['?', '#']).next().unwrap_or(src);
    let Some(decoded) = percent_decode(address) else { return Vec::new() };
    let decoded = decoded.replace('\\', "/");
    let b = decoded.as_bytes();
    let mut out = Vec::new();
    if cfg!(windows) {
        for i in 0..b.len().saturating_sub(2) {
            if b[i].is_ascii_alphabetic() && b[i + 1] == b':' && b[i + 2] == b'/' && (i == 0 || b[i - 1] == b'/') {
                out.push(PathBuf::from(format!("{}{}", decoded[i..=i].to_ascii_uppercase(), &decoded[i + 1..])));
            }
        }
    } else {
        for i in 0..b.len() {
            if b[i] == b'/' && b.get(i + 1).is_some_and(|&c| c != b'/') && (i == 0 || b[i - 1] != b'/') {
                out.push(PathBuf::from(&decoded[i..]));
            }
        }
    }
    out
}

fn is_temporary(path: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy().replace('\\', "/");
        let s = s.trim_end_matches('/').to_string();
        if cfg!(windows) { s.to_lowercase() } else { s }
    };
    let temp = norm(&std::env::temp_dir());
    !temp.is_empty() && norm(path).starts_with(&format!("{temp}/"))
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
        _ => return None,
    })
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
        let a = saver.resolve(&src).unwrap();
        let b = saver.resolve(&format!("data:image/png;base64,{PNG}")).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.extension().unwrap(), "png");
        assert!(is_ours(a.file_name().unwrap().to_str().unwrap()));
        assert_eq!(&std::fs::read(&a).unwrap()[1..4], b"PNG");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn file_uri(path: &Path) -> String {
        let p = path.to_str().unwrap().replace('\\', "/").replace(' ', "%20");
        format!("file:///{}", p.trim_start_matches('/'))
    }

    #[test]
    fn temporary_files_are_copied() {
        let dir = scratch("temp");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("clip image001.png");
        std::fs::write(&source, b"\x89PNG fake").unwrap();
        let out = dir.join("out");
        let saved = ImageSaver::new(out.clone()).resolve(&file_uri(&source)).unwrap();
        assert!(saved.starts_with(&out), "{saved:?}");
        assert_eq!(std::fs::read(saved).unwrap(), b"\x89PNG fake");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn other_local_files_are_linked_where_they_are() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("img-link-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("05 offre.png");
        std::fs::write(&file, b"x").unwrap();
        let out = scratch("linked");
        let saver = ImageSaver::new(out.clone());
        let path = file.to_str().unwrap().replace('\\', "/");
        let encoded = path.trim_start_matches('/').replace(':', "%3A").replace(' ', "%20");
        for src in [
            file_uri(&file),
            format!("https://file+.vscode-resource.vscode-cdn.net/{encoded}?version%3D1"),
            format!("app://49a1b2/{}?1700000000", path.trim_start_matches('/').replace(' ', "%20")),
        ] {
            let got = saver.resolve(&src).unwrap_or_else(|| panic!("{src}"));
            assert_eq!(got.to_str().unwrap().replace('\\', "/").to_lowercase(), path.to_lowercase(), "{src}");
        }
        assert!(!out.exists(), "nothing is copied for a file outside the temp directory");
        assert!(saver.resolve(&format!("https://example.com/{encoded}x")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_image_files_are_linked() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("img-ext-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let saver = ImageSaver::new(scratch("ext"));
        for name in ["credentials", "id_rsa.pub", "notes.txt", "logo.svg"] {
            let file = dir.join(name);
            std::fs::write(&file, b"secret").unwrap();
            assert!(saver.resolve(&file_uri(&file)).is_none(), "{name}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn work_per_conversion_is_capped() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("img-cap-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.png");
        std::fs::write(&file, b"x").unwrap();
        let saver = ImageSaver::new(scratch("cap"));
        let uri = file_uri(&file);
        for _ in 0..MAX_IMAGES {
            assert!(saver.resolve(&uri).is_some());
        }
        assert!(saver.resolve(&uri).is_none());
        let long = format!("{uri}?{}", "/".repeat(MAX_ADDRESS));
        assert!(ImageSaver::new(scratch("cap")).resolve(&long).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_what_cannot_be_read() {
        let saver = ImageSaver::new(scratch("none"));
        for src in [
            "cid:image001.png@01DC",
            "data:text/plain;base64,aGVsbG8=",
            "data:image/png,rawdata",
            "data:image/png;base64,***",
            "https://example.com/a.png",
            "file:///nowhere/missing.png",
            "imap://host/INBOX?part=1.2",
        ] {
            assert!(saver.resolve(src).is_none(), "{src}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_candidates() {
        let has = |src: &str, want: &str| candidates(src).iter().any(|p| p == Path::new(want));
        assert!(has("https://file+.vscode-resource.vscode-cdn.net/c%3A/Users/K/a%20b.png?v=1", "C:/Users/K/a b.png"));
        assert!(has("app://49a1/C:/Users/K/Vault/img.png?1700000000", "C:/Users/K/Vault/img.png"));
        assert!(has("file:///C:/x.png", "C:/x.png"));
        assert!(has("vscode-resource:/c:/x.png", "C:/x.png"));
        assert!(!candidates("file://server/share/a.png").iter().chain(&candidates("file:////server/share/a.png")).any(|p| p.to_string_lossy().starts_with(r"\\")));
        assert!(!candidates("file:///x.png").iter().any(|p| p.to_string_lossy().starts_with("E:")), "the scheme is not a drive");
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_candidates() {
        let c = candidates("app://49a1/home/k/a%20b.png?1700000000");
        assert!(c.contains(&PathBuf::from("/home/k/a b.png")) && c.contains(&PathBuf::from("/k/a b.png")), "{c:?}");
        assert!(candidates("file:///home/k/a.png").first() == Some(&PathBuf::from("/home/k/a.png")));
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
