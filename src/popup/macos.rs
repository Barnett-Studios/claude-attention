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
    unimplemented!("delegated: icon-digest")
}

/// The bundle name and id for an icon digest (see `Identity`). A digest shorter than 12 chars is
/// used whole.
pub fn identity(digest: &str) -> Identity {
    unimplemented!("delegated: bundle-identity")
}

/// Lowercase hex sha256 over `SWIFT_SOURCE` followed by the crate version — a changed notifier
/// source or a new ntfyer release rebuilds the binary.
pub fn source_digest() -> String {
    unimplemented!("delegated: source-digest")
}

/// The bundle's Info.plist: XML plist 1.0 with CFBundleIdentifier = `bundle_id`,
/// CFBundleName and CFBundleDisplayName = `ntfyer`, CFBundleExecutable = `Ntfyer`,
/// CFBundlePackageType = `APPL`, CFBundleShortVersionString = the crate version,
/// CFBundleVersion = `1`, LSUIElement = true, and CFBundleIconFile = `AppIcon` only when
/// `with_icon`.
pub fn info_plist(bundle_id: &str, with_icon: bool) -> String {
    unimplemented!("delegated: info-plist")
}

/// The icon file to bake in: Default → `default_icon` when `exists`; None → None; File(p) → p
/// when `exists`, else None.
pub fn icon_source(icon: &Icon, default_icon: &Path, exists: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    unimplemented!("delegated: icon-source")
}

/// `osascript` arguments for the fallback popup: `-e`, then
/// `display notification "<body>" with title "<title>"`, where in both strings every `\` becomes
/// `\\` and then every `"` becomes `\"`.
pub fn osascript_args(title: &str, body: &str) -> Vec<String> {
    unimplemented!("delegated: osascript-args")
}
