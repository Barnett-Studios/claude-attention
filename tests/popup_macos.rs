use ntfyer::config::Icon;
use ntfyer::popup::macos::{icon_digest, icon_source, identity, info_plist, osascript_args, source_digest, SWIFT_SOURCE};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn icon_digest_cases() {
    assert_eq!(icon_digest(None), "none");
    assert_eq!(icon_digest(Some(Path::new("/definitely/missing.png"))), "none");
    let d = tempfile::tempdir().expect("tempdir");
    let f = d.path().join("i.png");
    std::fs::write(&f, b"icon-bytes").expect("write");
    assert_eq!(icon_digest(Some(&f)), hex(b"icon-bytes"));
}

#[test]
fn identity_cases() {
    let id = identity("0123456789abcdef");
    assert_eq!(id.bundle_name, "Ntfyer-0123456789ab.app");
    assert_eq!(id.bundle_id, "dev.ntfyer.notifier.0123456789ab");
    let none = identity("none");
    assert_eq!(none.bundle_name, "Ntfyer-none.app");
    assert_eq!(none.bundle_id, "dev.ntfyer.notifier.none");
}

#[test]
fn source_digest_covers_source_and_version() {
    let mut bytes = SWIFT_SOURCE.as_bytes().to_vec();
    bytes.extend_from_slice(env!("CARGO_PKG_VERSION").as_bytes());
    assert_eq!(source_digest(), hex(&bytes));
}

#[test]
fn info_plist_contents() {
    let with = info_plist("dev.ntfyer.notifier.abc", true);
    assert!(with.starts_with("<?xml"), "xml header");
    for want in [
        "<key>CFBundleIdentifier</key><string>dev.ntfyer.notifier.abc</string>",
        "<key>CFBundleName</key><string>ntfyer</string>",
        "<key>CFBundleDisplayName</key><string>ntfyer</string>",
        "<key>CFBundleExecutable</key><string>Ntfyer</string>",
        "<key>CFBundlePackageType</key><string>APPL</string>",
        &format!("<key>CFBundleShortVersionString</key><string>{}</string>", env!("CARGO_PKG_VERSION")),
        "<key>CFBundleVersion</key><string>1</string>",
        "<key>LSUIElement</key><true/>",
        "<key>CFBundleIconFile</key><string>AppIcon</string>",
    ] {
        assert!(with.contains(want), "missing {want}");
    }
    let without = info_plist("x", false);
    assert!(!without.contains("CFBundleIconFile"));
    assert!(without.trim_end().ends_with("</plist>"));
}

#[test]
fn icon_source_cases() {
    let def = Path::new("/Applications/Claude.app/icon.icns");
    assert_eq!(icon_source(&Icon::Default, def, &|_| true), Some(def.to_path_buf()));
    assert_eq!(icon_source(&Icon::Default, def, &|_| false), None);
    assert_eq!(icon_source(&Icon::None, def, &|_| true), None);
    assert_eq!(icon_source(&Icon::File("/i.png".into()), def, &|_| true), Some(PathBuf::from("/i.png")));
    assert_eq!(icon_source(&Icon::File("/i.png".into()), def, &|_| false), None);
}

#[test]
fn osascript_args_escape() {
    assert_eq!(
        osascript_args("T", "B"),
        vec!["-e".to_string(), r#"display notification "B" with title "T""#.to_string()]
    );
    assert_eq!(
        osascript_args(r#"a"b"#, r"c\d"),
        vec!["-e".to_string(), r#"display notification "c\\d" with title "a\"b""#.to_string()]
    );
}
