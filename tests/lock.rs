use ntfyer::lock::{acquire, open};

#[test]
fn acquire_fresh_creates_file_and_dirs() {
    let d = tempfile::tempdir().expect("tempdir");
    let path = d.path().join("app/build.lock");
    let g = acquire(&path).expect("fresh lock");
    assert!(path.is_file());
    drop(g);
    assert!(acquire(&path).is_some(), "released on drop");
}

#[test]
fn acquire_respects_a_live_holder() {
    let d = tempfile::tempdir().expect("tempdir");
    let path = d.path().join("build.lock");
    let held = open(&path).expect("open");
    held.try_lock().expect("hold it");
    assert!(acquire(&path).is_none(), "someone else holds it");
    held.unlock().expect("unlock");
    assert!(acquire(&path).is_some());
}

#[test]
fn acquire_ignores_a_leftover_file() {
    let d = tempfile::tempdir().expect("tempdir");
    let path = d.path().join("build.lock");
    std::fs::write(&path, "999999").expect("leftover");
    assert!(acquire(&path).is_some(), "a file nobody holds is no lock");
}

#[test]
fn acquire_two_guards_cannot_coexist() {
    let d = tempfile::tempdir().expect("tempdir");
    let path = d.path().join("build.lock");
    let first = acquire(&path).expect("first");
    assert!(acquire(&path).is_none());
    drop(first);
}
