//! Windows: embeds VERSIONINFO, icon and an asInvoker manifest. Unsigned binaries without
//! metadata trip more antivirus heuristics.

#[cfg(windows)]
#[path = "src/icon_pixels.rs"]
mod icon_pixels;

#[cfg(windows)]
const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>
"#;

#[cfg(windows)]
fn ico_bytes() -> Vec<u8> {
    let rgba = icon_pixels::rgba();
    let (w, h) = (32usize, 32usize);
    let mut dib = Vec::new();
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(w as i32).to_le_bytes());
    dib.extend_from_slice(&((h * 2) as i32).to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&32u16.to_le_bytes());
    dib.extend_from_slice(&[0u8; 24]);
    for y in (0..h).rev() {
        for x in 0..w {
            let i = (y * w + x) * 4;
            dib.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    dib.extend(std::iter::repeat_n(0u8, h * 4)); // AND mask
    let mut ico = vec![0, 0, 1, 0, 1, 0, w as u8, h as u8, 0, 0, 1, 0, 32, 0];
    ico.extend_from_slice(&(dib.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend(dib);
    ico
}

#[cfg(windows)]
fn embed() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico = out.join("app.ico");
    std::fs::write(&ico, ico_bytes()).unwrap();
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().unwrap())
        .set_manifest(MANIFEST)
        .set_language(0x0409)
        .set("ProductName", "clip-md-html")
        .set("FileDescription", "Converts clipboard Markdown to HTML")
        .set("CompanyName", "shaffe-fr")
        .set("OriginalFilename", "clip-md-html.exe")
        .set("InternalName", "clip-md-html")
        .set("LegalCopyright", "MIT - shaffe-fr");
    res.compile().expect("failed to compile Windows resources");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/icon_pixels.rs");
    #[cfg(windows)]
    embed();
}
