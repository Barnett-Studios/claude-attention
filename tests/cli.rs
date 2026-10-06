//! `ntfyer signal` behaviour through the real binary, in dry-run mode (NTFYER_DRY_RUN=1 prints the
//! channels it would fire: `bell`, `sound:<file>`, `popup:<title>|<message>`). Ported from the
//! claude-attention shell suite.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Case {
    _dir: tempfile::TempDir,
    home: PathBuf,
    proj: PathBuf,
}

impl Case {
    fn new() -> Case {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("home");
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(home.join(".config/ntfyer")).expect("config dir");
        std::fs::create_dir_all(&proj).expect("proj");
        Case {
            _dir: dir,
            home,
            proj,
        }
    }

    fn global(&self, json: &str) {
        std::fs::write(self.home.join(".config/ntfyer/config.json"), json).expect("global config");
    }

    fn project(&self, json: &str) {
        std::fs::write(self.proj.join(".ntfyer.json"), json).expect("project config");
    }

    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_ntfyer"));
        c.args(args)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", &self.home)
            .env("NTFYER_DRY_RUN", "1")
            .env("TMPDIR", self.home.parent().expect("case root"))
            .current_dir(&self.proj);
        c
    }

    /// `ntfyer signal --json` with `stdin`; returns (stdout, exit code)
    fn signal_json(&self, stdin: &str) -> (String, i32) {
        self.run(&["signal", "--json"], stdin, &[])
    }

    fn run(&self, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> (String, i32) {
        let mut c = self.cmd(args);
        for (k, v) in env {
            c.env(k, v);
        }
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(stdin.as_bytes())
            .expect("write stdin");
        let out = child.wait_with_output().expect("wait");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            out.status.code().unwrap_or(-1),
        )
    }

    fn env_json(&self, message: &str) -> String {
        serde_json::json!({"message": message, "project": self.proj}).to_string()
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.home.join(".local/state/ntfyer/log")).unwrap_or_default()
    }
}

/// The OS default chime, hardcoded here (not computed by the code under test); absent when this
/// machine does not have the file (e.g. a headless Linux CI image without freedesktop sounds).
fn default_sound_line() -> Option<String> {
    let file = if cfg!(target_os = "macos") {
        "/System/Library/Sounds/Glass.aiff"
    } else {
        "/usr/share/sounds/freedesktop/stereo/message-new-instant.oga"
    };
    Path::new(file).is_file().then(|| format!("sound:{file}"))
}

fn lines(xs: &[Option<String>]) -> String {
    xs.iter().flatten().map(|l| format!("{l}\n")).collect()
}

fn all(title: &str, msg: &str) -> String {
    lines(&[
        Some("bell".into()),
        default_sound_line(),
        Some(format!("popup:{title}|{msg}")),
    ])
}

#[test]
fn defaults_fire_every_channel() {
    let c = Case::new();
    assert_eq!(
        c.signal_json(&c.env_json("hello")),
        (all("proj", "hello"), 0)
    );
}

#[test]
fn global_enabled_false_silences_all() {
    let c = Case::new();
    c.global(r#"{"enabled":false}"#);
    assert_eq!(c.signal_json(&c.env_json("hello")).0, "");
}

#[test]
fn project_overrides_global() {
    let c = Case::new();
    c.global(r#"{"enabled":false}"#);
    c.project(r#"{"enabled":true}"#);
    assert_eq!(c.signal_json(&c.env_json("hello")).0, all("proj", "hello"));
}

#[test]
fn per_channel_switches() {
    let c = Case::new();
    c.project(r#"{"sound":false,"bell":false}"#);
    assert_eq!(c.signal_json(&c.env_json("hello")).0, "popup:proj|hello\n");
}

#[test]
fn sound_false_alone_keeps_bell_and_popup() {
    let c = Case::new();
    c.global(r#"{"sound":false}"#);
    assert_eq!(
        c.signal_json(&c.env_json("hello")).0,
        "bell\npopup:proj|hello\n"
    );
}

#[test]
fn bell_false_alone_keeps_sound_and_popup() {
    let c = Case::new();
    c.project(r#"{"bell":false}"#);
    assert_eq!(
        c.signal_json(&c.env_json("hello")).0,
        lines(&[default_sound_line(), Some("popup:proj|hello".into())])
    );
}

#[test]
fn null_in_project_does_not_override_global() {
    let c = Case::new();
    c.global(r#"{"enabled":false}"#);
    c.project(r#"{"enabled":null}"#);
    assert_eq!(c.signal_json(&c.env_json("hello")).0, "");
}

#[test]
fn project_cannot_point_sound_at_a_file() {
    let c = Case::new();
    let f = c.proj.join("evil.wav");
    std::fs::write(&f, b"x").expect("wav");
    c.project(&format!(r#"{{"sound":"{}"}}"#, f.display()));
    assert_eq!(
        c.signal_json(&c.env_json("hello")).0,
        all("proj", "hello"),
        "falls back to the default sound"
    );
}

#[test]
fn popup_off_keeps_bell_and_sound() {
    let c = Case::new();
    c.global(r#"{"popup":false}"#);
    assert_eq!(
        c.signal_json(&c.env_json("hello")).0,
        lines(&[Some("bell".into()), default_sound_line()])
    );
}

#[test]
fn env_off_silences_all() {
    let c = Case::new();
    let (out, code) = c.run(
        &["signal", "--json"],
        &c.env_json("hello"),
        &[("NTFYER", "off")],
    );
    assert_eq!((out.as_str(), code), ("", 0));
    assert!(c.log().contains("verdict=skip:env-off"));
}

#[test]
fn quiet_envelope_is_logged_not_signalled() {
    let c = Case::new();
    let input =
        serde_json::json!({"message":"m","project":c.proj,"quiet":true,"event":"stop"}).to_string();
    assert_eq!(c.signal_json(&input).0, "");
    assert!(c
        .log()
        .contains("event=stop verdict=skip:quiet project=proj"));
}

#[test]
fn stop_without_message_gets_default_text() {
    let c = Case::new();
    let input = serde_json::json!({"event":"stop","project":c.proj}).to_string();
    assert_eq!(
        c.signal_json(&input).0,
        all("proj", "Finished and waiting for you.")
    );
}

#[test]
fn title_from_envelope() {
    let c = Case::new();
    let input = serde_json::json!({"message":"m","title":"Claude Code · proj","project":c.proj})
        .to_string();
    assert_eq!(c.signal_json(&input).0, all("Claude Code · proj", "m"));
}

#[test]
fn flags_without_stdin() {
    let c = Case::new();
    let p = c.proj.to_string_lossy().into_owned();
    let (out, code) = c.run(
        &[
            "signal",
            "--message",
            "hi",
            "--title",
            "T",
            "--project",
            &p,
            "--event",
            "stop",
        ],
        "",
        &[],
    );
    assert_eq!((out, code), (all("T", "hi"), 0));
}

#[test]
fn no_input_uses_working_directory() {
    let c = Case::new();
    assert_eq!(
        c.run(&["signal"], "", &[]).0,
        all("proj", "Needs your attention.")
    );
}

#[test]
fn second_signal_inside_window_is_silent() {
    let c = Case::new();
    c.signal_json(&c.env_json("first"));
    assert_eq!(c.signal_json(&c.env_json("again")).0, "");
    assert!(c.log().contains("verdict=skip:debounce"));
}

#[test]
fn debounce_is_per_project() {
    let c = Case::new();
    c.signal_json(&c.env_json("first"));
    let other = c.proj.parent().expect("parent").join("other");
    std::fs::create_dir_all(&other).expect("other");
    let input = serde_json::json!({"message":"hello","project":other}).to_string();
    assert_eq!(c.signal_json(&input).0, all("other", "hello"));
}

#[test]
fn malformed_config_fails_open() {
    let c = Case::new();
    c.global("not json");
    assert_eq!(c.signal_json(&c.env_json("hello")).0, all("proj", "hello"));
}

#[test]
fn garbage_stdin_still_signals_and_exits_zero() {
    let c = Case::new();
    assert_eq!(
        c.signal_json("{{{ nope"),
        (all("proj", "Needs your attention."), 0)
    );
}

#[test]
fn sound_by_file_path_and_tilde() {
    let c = Case::new();
    let f = c.home.join("chime.wav");
    std::fs::write(&f, b"x").expect("wav");
    c.global(r#"{"sound":"~/chime.wav"}"#);
    let want = lines(&[
        Some("bell".into()),
        Some(format!("sound:{}", f.display())),
        Some("popup:proj|hello".into()),
    ]);
    assert_eq!(c.signal_json(&c.env_json("hello")).0, want);
}

#[test]
fn unknown_sound_plays_nothing_other_channels_fire() {
    let c = Case::new();
    c.global(r#"{"sound":"NoSuchSound"}"#);
    assert_eq!(
        c.signal_json(&c.env_json("hello")).0,
        "bell\npopup:proj|hello\n"
    );
}

#[test]
fn null_sound_keeps_default() {
    let c = Case::new();
    c.global(r#"{"sound":null}"#);
    assert_eq!(c.signal_json(&c.env_json("hello")).0, all("proj", "hello"));
}

#[cfg(target_os = "macos")]
#[test]
fn sound_by_system_name() {
    let c = Case::new();
    c.project(r#"{"sound":"Ping"}"#);
    assert_eq!(
        c.signal_json(&c.env_json("hello")).0,
        "bell\nsound:/System/Library/Sounds/Ping.aiff\npopup:proj|hello\n"
    );
}

#[test]
fn log_is_one_line_per_event_despite_newlines() {
    let c = Case::new();
    let weird = c.proj.parent().expect("parent").join("a\nb");
    std::fs::create_dir_all(&weird).expect("weird");
    let input = serde_json::json!({"message":"x\ny","project":weird,"session":"s\nt"}).to_string();
    c.signal_json(&input);
    assert_eq!(c.log().lines().count(), 1, "log: {:?}", c.log());
}

#[test]
fn config_path_prints_effective_paths() {
    let c = Case::new();
    let (out, code) = c.run(&["config", "path"], "", &[]);
    assert_eq!(code, 0);
    assert!(
        out.contains(&format!(
            "global: {}",
            c.home.join(".config/ntfyer/config.json").display()
        )),
        "{out}"
    );
    assert!(
        out.contains(&format!(
            "project: {}",
            c.proj.join(".ntfyer.json").display()
        )),
        "{out}"
    );
}

#[test]
fn usage_error_exits_64() {
    let c = Case::new();
    assert_eq!(c.run(&["bogus"], "", &[]).1, 64);
    assert_eq!(c.run(&["config", "nope"], "", &[]).1, 64);
}

#[test]
fn signal_with_unknown_flag_still_exits_zero() {
    let c = Case::new();
    assert_eq!(c.run(&["signal", "--from-the-future", "x"], "", &[]).1, 0);
}

#[test]
fn doctor_without_any_backend_is_degraded() {
    let c = Case::new();
    let (out, code) = c.run(&["doctor", "--format", "json"], "", &[("PATH", "")]);
    assert_eq!(code, 1, "{out}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("doctor emits JSON");
    assert_eq!(v["body"]["healthy"], false);
}

#[cfg(target_os = "linux")]
#[test]
fn build_is_a_no_op_on_linux() {
    let c = Case::new();
    assert_eq!(c.run(&["build"], "", &[]), (String::new(), 0));
}

#[test]
fn doctor_reports_json() {
    let c = Case::new();
    let (out, _) = c.run(&["doctor", "--format", "json"], "", &[]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("doctor emits JSON");
    assert_eq!(v["schema_version"], "1");
    assert!(v["body"]["config"]["global"].is_string());
    assert!(v["body"]["backends"].is_object());
}
