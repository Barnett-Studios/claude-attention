use ntfyer::log::{append, sanitize, KEEP_LINES};

#[test]
fn sanitize_strips_line_breaks() {
    assert_eq!(sanitize("a\nb\rc"), "a b c");
    assert_eq!(sanitize("plain"), "plain");
}

#[test]
fn append_creates_and_appends() {
    let d = tempfile::tempdir().expect("tempdir");
    let log = d.path().join("state/ntfyer/log");
    append(&log, "one");
    append(&log, "two");
    assert_eq!(std::fs::read_to_string(&log).expect("log"), "one\ntwo\n");
}

#[test]
fn append_keeps_last_lines() {
    let d = tempfile::tempdir().expect("tempdir");
    let log = d.path().join("log");
    for i in 0..(KEEP_LINES + 25) {
        append(&log, &format!("line {i}"));
    }
    let text = std::fs::read_to_string(&log).expect("log");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), KEEP_LINES);
    assert_eq!(lines[0], "line 25");
    assert_eq!(*lines.last().expect("last"), format!("line {}", KEEP_LINES + 24));
    let leftovers = std::fs::read_dir(d.path()).expect("dir").count();
    assert_eq!(leftovers, 1, "temp file renamed away");
}

#[test]
fn append_ignores_unwritable_location() {
    append(std::path::Path::new("/dev/null/cannot/log"), "x");
}
