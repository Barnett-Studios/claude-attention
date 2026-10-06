use ntfyer::config::{expand_home, parse_icon, parse_sound, read_layer, resolve, Config, Icon, Sound};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const HOME: &str = "/h";
const CFG: &str = "/h/.config/ntfyer";

fn h() -> &'static Path {
    Path::new(HOME)
}

#[test]
fn expand_home_cases() {
    assert_eq!(expand_home("~", h()), PathBuf::from("/h"));
    assert_eq!(expand_home("~/a/b.wav", h()), PathBuf::from("/h/a/b.wav"));
    assert_eq!(expand_home("/abs/x", h()), PathBuf::from("/abs/x"));
    assert_eq!(expand_home("rel/x", h()), PathBuf::from("rel/x"));
    assert_eq!(expand_home("~user/x", h()), PathBuf::from("~user/x"));
}

#[test]
fn parse_sound_cases() {
    assert_eq!(parse_sound(&json!(false), h()), Sound::Off);
    assert_eq!(parse_sound(&json!(true), h()), Sound::Default);
    assert_eq!(parse_sound(&json!(""), h()), Sound::Default);
    assert_eq!(parse_sound(&json!("Pop"), h()), Sound::Name("Pop".into()));
    assert_eq!(parse_sound(&json!("/x/y.wav"), h()), Sound::File("/x/y.wav".into()));
    assert_eq!(parse_sound(&json!("~/c.wav"), h()), Sound::File("/h/c.wav".into()));
    assert_eq!(parse_sound(&json!("dir/c.wav"), h()), Sound::File("dir/c.wav".into()));
    assert_eq!(parse_sound(&json!(3), h()), Sound::Default);
    assert_eq!(parse_sound(&Value::Null, h()), Sound::Default);
}

#[test]
fn parse_icon_cases() {
    let c = Path::new(CFG);
    assert_eq!(parse_icon(&json!(false), h(), c), Icon::None);
    assert_eq!(parse_icon(&json!(true), h(), c), Icon::Default);
    assert_eq!(parse_icon(&json!("claude"), h(), c), Icon::Default);
    assert_eq!(parse_icon(&json!(""), h(), c), Icon::Default);
    assert_eq!(parse_icon(&json!("/i/a.png"), h(), c), Icon::File("/i/a.png".into()));
    assert_eq!(parse_icon(&json!("~/a.png"), h(), c), Icon::File("/h/a.png".into()));
    assert_eq!(parse_icon(&json!("rel.png"), h(), c), Icon::File("/h/.config/ntfyer/rel.png".into()));
    assert_eq!(parse_icon(&json!(7), h(), c), Icon::Default);
}

#[test]
fn read_layer_cases() {
    let d = tempfile::tempdir().expect("tempdir");
    let p = d.path().join("c.json");
    assert_eq!(read_layer(&p), Value::Null, "missing file");
    std::fs::write(&p, "not json").expect("write");
    assert_eq!(read_layer(&p), Value::Null, "malformed");
    std::fs::write(&p, "[1,2]").expect("write");
    assert_eq!(read_layer(&p), Value::Null, "not an object");
    std::fs::write(&p, r#"{"sound":"Pop"}"#).expect("write");
    assert_eq!(read_layer(&p), json!({"sound":"Pop"}));
}

fn res(g: Value, p: Value) -> Config {
    resolve(&g, &p, h(), Path::new(CFG))
}

#[test]
fn resolve_defaults() {
    assert_eq!(res(Value::Null, Value::Null), Config::default());
    assert_eq!(res(json!({}), json!({})), Config::default());
}

#[test]
fn resolve_global_false_and_project_override() {
    let c = res(json!({"enabled": false}), Value::Null);
    assert!(!c.enabled);
    let c = res(json!({"enabled": false}), json!({"enabled": true}));
    assert!(c.enabled, "project overrides global");
}

#[test]
fn resolve_null_does_not_override() {
    let c = res(json!({"enabled": false}), json!({"enabled": null}));
    assert!(!c.enabled);
    let c = res(json!({"sound": "Pop"}), json!({"sound": null}));
    assert_eq!(c.sound, Sound::Name("Pop".into()));
}

#[test]
fn resolve_per_channel_switches() {
    let c = res(json!({"bell": false}), json!({"popup": false, "sound": false}));
    assert!(!c.bell && !c.popup && c.enabled);
    assert_eq!(c.sound, Sound::Off);
}

#[test]
fn resolve_non_bool_flag_is_true() {
    let c = res(json!({"bell": "no"}), Value::Null);
    assert!(c.bell, "only JSON false disables");
}

#[test]
fn resolve_icon_is_global_only() {
    let c = res(json!({"icon": "/g.png"}), json!({"icon": "/p.png"}));
    assert_eq!(c.icon, Icon::File("/g.png".into()));
    let c = res(Value::Null, json!({"icon": false}));
    assert_eq!(c.icon, Icon::Default);
}

#[test]
fn resolve_ignores_non_object_layers() {
    let c = res(json!([1]), json!("x"));
    assert_eq!(c, Config::default());
}
