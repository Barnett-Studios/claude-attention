use ntfyer::paths::{project_config, Paths};
use std::path::PathBuf;

fn lookup(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |k| {
        pairs
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.to_string())
    }
}

#[test]
fn paths_defaults_under_home() {
    let p = Paths::from_lookup(lookup(&[("HOME", "/h")])).expect("home set");
    assert_eq!(p.home, PathBuf::from("/h"));
    assert_eq!(p.config_dir, PathBuf::from("/h/.config/ntfyer"));
    assert_eq!(p.state_dir, PathBuf::from("/h/.local/state/ntfyer"));
    assert_eq!(
        p.app_dir,
        PathBuf::from("/h/Library/Application Support/ntfyer")
    );
    assert_eq!(
        p.global_config(),
        PathBuf::from("/h/.config/ntfyer/config.json")
    );
    assert_eq!(p.log_file(), PathBuf::from("/h/.local/state/ntfyer/log"));
}

#[test]
fn paths_honour_xdg() {
    let p = Paths::from_lookup(lookup(&[
        ("HOME", "/h"),
        ("XDG_CONFIG_HOME", "/c"),
        ("XDG_STATE_HOME", "/s"),
    ]))
    .expect("home set");
    assert_eq!(p.config_dir, PathBuf::from("/c/ntfyer"));
    assert_eq!(p.state_dir, PathBuf::from("/s/ntfyer"));
}

#[test]
fn paths_empty_xdg_counts_as_unset() {
    let p = Paths::from_lookup(lookup(&[
        ("HOME", "/h"),
        ("XDG_CONFIG_HOME", ""),
        ("XDG_STATE_HOME", ""),
    ]))
    .expect("home set");
    assert_eq!(p.config_dir, PathBuf::from("/h/.config/ntfyer"));
    assert_eq!(p.state_dir, PathBuf::from("/h/.local/state/ntfyer"));
}

#[test]
fn paths_need_home() {
    assert!(Paths::from_lookup(lookup(&[])).is_none());
    assert!(Paths::from_lookup(lookup(&[("HOME", "")])).is_none());
}

#[test]
fn project_config_file_name() {
    assert_eq!(
        project_config(&PathBuf::from("/p")),
        PathBuf::from("/p/.ntfyer.json")
    );
}
