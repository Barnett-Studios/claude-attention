//! macOS: a tiny Swift app posts the popup. macOS takes a notification's icon from the posting
//! app's bundle, and Notification Center keeps the first icon it sees for a bundle id for good, so
//! every icon gets its own app identity (`Ntfyer-<digest>.app`, `dev.ntfyer.notifier.<digest>`).
//! Changing the icon builds a new bundle, which the user allows once, and retires the old one.

use crate::config::Icon;
use std::path::{Path, PathBuf};

pub const ID_PREFIX: &str = "dev.ntfyer.notifier";
pub const SWIFT_SOURCE: &str = include_str!("../../notifier/Ntfyer.swift");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// `Ntfyer-<first 12 chars of digest>.app`
    pub bundle_name: String,
    /// `dev.ntfyer.notifier.<first 12 chars of digest>`
    pub bundle_id: String,
}

/// `"none"` when `file` is None or unreadable, else the lowercase hex sha256 of its contents.
pub fn icon_digest(file: Option<&Path>) -> String {
    use sha2::{Digest, Sha256};
    match file.and_then(|f| std::fs::read(f).ok()) {
        Some(bytes) => hex(&Sha256::digest(bytes)),
        None => "none".to_string(),
    }
}

/// The bundle name and id for an icon digest (see `Identity`). A digest shorter than 12 chars is
/// used whole.
pub fn identity(digest: &str) -> Identity {
    let short = if digest.len() >= 12 {
        &digest[..12]
    } else {
        digest
    };
    Identity {
        bundle_name: format!("Ntfyer-{}.app", short),
        bundle_id: format!("{}.{}", ID_PREFIX, short),
    }
}

/// Lowercase hex sha256 over the bytes of `SWIFT_SOURCE` immediately followed (no separator) by the
/// bytes of the crate version — a changed notifier source or a new ntfyer release rebuilds the
/// binary.
pub fn source_digest() -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(SWIFT_SOURCE.as_bytes());
    h.update(env!("CARGO_PKG_VERSION").as_bytes());
    hex(&h.finalize())
}

/// The bundle's Info.plist: XML plist 1.0 with CFBundleIdentifier = `bundle_id`,
/// CFBundleName and CFBundleDisplayName = `ntfyer`, CFBundleExecutable = `Ntfyer`,
/// CFBundlePackageType = `APPL`, CFBundleShortVersionString = the crate version,
/// CFBundleVersion = `1`, LSUIElement = true, and CFBundleIconFile = `AppIcon` only when
/// `with_icon`.
pub fn info_plist(bundle_id: &str, with_icon: bool) -> String {
    let icon = if with_icon {
        "\n<key>CFBundleIconFile</key><string>AppIcon</string>"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{bundle_id}</string>
<key>CFBundleName</key><string>ntfyer</string>
<key>CFBundleDisplayName</key><string>ntfyer</string>
<key>CFBundleExecutable</key><string>Ntfyer</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>{version}</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSUIElement</key><true/>{icon}
</dict></plist>
"#,
        version = env!("CARGO_PKG_VERSION"),
    )
}

/// The icon file to bake in: Default → `default_icon` when `exists`; None → None; File(p) → p
/// when `exists`, else None.
pub fn icon_source(
    icon: &Icon,
    default_icon: &Path,
    exists: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    match icon {
        Icon::Default => {
            if exists(default_icon) {
                Some(default_icon.to_path_buf())
            } else {
                None
            }
        }
        Icon::None => None,
        Icon::File(p) => {
            if exists(p) {
                Some(p.to_path_buf())
            } else {
                None
            }
        }
    }
}

/// `osascript` arguments for the fallback popup: `-e`, then
/// `display notification "<body>" with title "<title>"`, where in both strings every `\` becomes
/// `\\` and then every `"` becomes `\"`.
pub fn osascript_args(title: &str, body: &str) -> Vec<String> {
    let escape = |s: &str| -> String {
        let mut out = String::new();
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                _ => out.push(c),
            }
        }
        out
    };
    vec![
        "-e".to_string(),
        format!(
            "display notification \"{}\" with title \"{}\"",
            escape(body),
            escape(title)
        ),
    ]
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
