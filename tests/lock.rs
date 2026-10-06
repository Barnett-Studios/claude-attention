use ntfyer::lock::acquire;

#[test]
fn acquire_fresh_writes_pid_and_releases_on_drop() {
    let d = tempfile::tempdir().expect("tempdir");
    let dir = d.path().join("app/.build.lock");
    let g = acquire(&dir, 4242, &|_| true).expect("fresh lock");
    assert_eq!(std::fs::read_to_string(dir.join("pid")).expect("pid").trim(), "4242");
    assert_eq!(g.path(), dir.as_path());
    drop(g);
    assert!(!dir.exists(), "released");
}

#[test]
fn acquire_respects_live_owner() {
    let d = tempfile::tempdir().expect("tempdir");
    let dir = d.path().join(".build.lock");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("pid"), "77").expect("pid");
    assert!(acquire(&dir, 1, &|p| p == 77).is_none());
    assert!(dir.exists(), "live owner's lock untouched");
}

#[test]
fn acquire_takes_over_dead_owner() {
    let d = tempfile::tempdir().expect("tempdir");
    let dir = d.path().join(".build.lock");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("pid"), "999999").expect("pid");
    let g = acquire(&dir, 5, &|_| false).expect("stale lock taken over");
    assert_eq!(std::fs::read_to_string(dir.join("pid")).expect("pid").trim(), "5");
    drop(g);
}

#[test]
fn acquire_takes_over_lock_without_pid() {
    let d = tempfile::tempdir().expect("tempdir");
    let dir = d.path().join(".build.lock");
    std::fs::create_dir_all(&dir).expect("mkdir");
    assert!(acquire(&dir, 5, &|_| true).is_some(), "no pid file: the owner died before writing it");
}
