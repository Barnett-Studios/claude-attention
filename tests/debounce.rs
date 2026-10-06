use ntfyer::debounce::{admit, stamp_path};
use std::path::Path;

#[test]
fn stamp_path_is_per_project_and_stable() {
    let s = Path::new("/state");
    let a = stamp_path(s, Path::new("/p/a"));
    let b = stamp_path(s, Path::new("/p/b"));
    assert_ne!(a, b);
    assert_eq!(a, stamp_path(s, Path::new("/p/a")));
    assert!(a.starts_with("/state/debounce"));
    let name = a.file_name().and_then(|n| n.to_str()).expect("name");
    assert!(
        name.ends_with(".last") && name.len() == 12 + 5,
        "got {name}"
    );
    assert_eq!(name, format!("{}.last", sha12("/p/a")));
}

fn sha12(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let d = Sha256::digest(s.as_bytes());
    d.iter().map(|b| format!("{b:02x}")).collect::<String>()[..12].to_string()
}

#[test]
fn admit_window() {
    let d = tempfile::tempdir().expect("tempdir");
    let stamp = d.path().join("nested/debounce/x.last");
    assert!(
        admit(&stamp, 1000, 8),
        "first signal passes and creates dirs"
    );
    assert_eq!(
        std::fs::read_to_string(&stamp).expect("stamp").trim(),
        "1000"
    );
    assert!(!admit(&stamp, 1007, 8), "inside window");
    assert_eq!(
        std::fs::read_to_string(&stamp).expect("stamp").trim(),
        "1000",
        "untouched"
    );
    assert!(admit(&stamp, 1008, 8), "window elapsed");
}

#[test]
fn admit_garbage_stamp_counts_as_zero() {
    let d = tempfile::tempdir().expect("tempdir");
    let stamp = d.path().join("x.last");
    std::fs::write(&stamp, "garbage").expect("write");
    assert!(!admit(&stamp, 5, 8), "garbage reads as 0, and 5 - 0 < 8");
    assert!(admit(&stamp, 100, 8));
}

#[test]
fn admit_clock_going_backwards_does_not_block_forever() {
    let d = tempfile::tempdir().expect("tempdir");
    let stamp = d.path().join("x.last");
    std::fs::write(&stamp, "5000").expect("write");
    assert!(
        !admit(&stamp, 10, 8),
        "saturating: now < last counts as inside the window"
    );
}
