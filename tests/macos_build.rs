//! `ntfyer build` on macOS: the app bundle, one identity per icon, rebuild avoidance, the build
//! lock, and never compiling inside `signal`. Builds for real (needs the Xcode Command Line Tools)
//! into an isolated HOME and never registers with LaunchServices (NTFYER_REGISTER=0): a stale
//! registration of a deleted copy confuses Notification Center. Never posts a notification.
#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const GENERIC: &str = "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericApplicationIcon.icns";

struct Env {
    dir: tempfile::TempDir,
    home: PathBuf,
}

impl Env {
    fn new() -> Env {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("home");
        std::fs::create_dir_all(home.join(".config/ntfyer")).expect("config");
        Env { dir, home }
    }
    fn app_dir(&self) -> PathBuf {
        self.home.join("Library/Application Support/ntfyer")
    }
    fn config(&self, json: &str) {
        std::fs::write(self.home.join(".config/ntfyer/config.json"), json).expect("config");
    }
    fn cmd(&self, args: &[&str], path_prefix: Option<&Path>, default_icon: &str) -> Command {
        let path = std::env::var("PATH").unwrap_or_default();
        let path = match path_prefix {
            Some(p) => format!("{}:{path}", p.display()),
            None => path,
        };
        let mut c = Command::new(env!("CARGO_BIN_EXE_ntfyer"));
        c.args(args)
            .env_clear()
            .env("PATH", path)
            .env("HOME", &self.home)
            .env("NTFYER_REGISTER", "0")
            .env("NTFYER_DEFAULT_ICON", default_icon)
            .stdin(Stdio::null());
        c
    }
    fn build(&self) -> Output {
        self.build_with_icon(&self.dir.path().join("no-default.icns").to_string_lossy())
    }
    fn build_with_icon(&self, default_icon: &str) -> Output {
        self.cmd(&["build"], None, default_icon).output().expect("run build")
    }
    fn apps(&self) -> Vec<PathBuf> {
        std::fs::read_dir(self.app_dir())
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("Ntfyer-") && n.ends_with(".app")))
                    .collect()
            })
            .unwrap_or_default()
    }
    fn app(&self) -> PathBuf {
        let apps = self.apps();
        assert_eq!(apps.len(), 1, "exactly one bundle: {apps:?}");
        apps[0].clone()
    }
    fn bundle_id(&self) -> String {
        plist_value(&self.app().join("Contents/Info.plist"), "CFBundleIdentifier")
    }
    fn icns(&self) -> PathBuf {
        self.app().join("Contents/Resources/AppIcon.icns")
    }
    fn png(&self, name: &str, rotate: Option<&str>) -> PathBuf {
        let out = self.dir.path().join(name);
        let mut c = Command::new("sips");
        c.args(["-s", "format", "png"]);
        if let Some(r) = rotate {
            c.args(["-r", r]);
        }
        let ok = c.arg(GENERIC).arg("--out").arg(&out).stdout(Stdio::null()).status().expect("sips").success();
        assert!(ok, "sips made {name}");
        out
    }
}

fn plist_value(plist: &Path, key: &str) -> String {
    let out = Command::new("/usr/libexec/PlistBuddy").args(["-c", &format!("Print :{key}")]).arg(plist).output().expect("PlistBuddy");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn sha(p: &Path) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(std::fs::read(p).expect("read")).to_vec()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn signed(app: &Path) -> bool {
    Command::new("codesign").arg("--verify").arg(app).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

fn wait_until(limit: Duration, cond: impl Fn() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    cond()
}

#[test]
fn build_lifecycle_and_icons() {
    let e = Env::new();
    let first = e.build();
    assert!(first.status.success(), "build ok: {}", String::from_utf8_lossy(&first.stderr));
    assert_eq!(stdout(&first), "compiled");
    let app = e.app();
    assert!(app.join("Contents/MacOS/Ntfyer").is_file(), "executable");
    assert!(e.bundle_id().starts_with("dev.ntfyer.notifier."), "namespaced id");
    assert!(signed(&app), "signed");
    assert!(!e.icns().exists(), "no icon when the default icon is absent");
    let usage = Command::new(app.join("Contents/MacOS/Ntfyer")).output().expect("run notifier");
    assert_eq!(usage.status.code(), Some(2), "notifier usage error");
    let generic_id = e.bundle_id();

    let a = e.png("a.png", None);
    e.config(&format!(r#"{{"icon":"{}"}}"#, a.display()));
    let out = e.build();
    assert_eq!(stdout(&out), "icon updated", "icon change does not recompile");
    assert!(std::fs::read(e.icns()).expect("icns").starts_with(b"icns"), "png converted to icns");
    assert_ne!(e.bundle_id(), generic_id, "new identity per icon");
    assert!(signed(&e.app()), "new bundle signed");
    let (first_sum, first_id) = (sha(&e.icns()), e.bundle_id());

    assert_eq!(stdout(&e.build()), "", "unchanged build does nothing");
    assert_eq!(sha(&e.icns()), first_sum);

    let b = e.png("b.png", Some("90"));
    e.config(&format!(r#"{{"icon":"{}"}}"#, b.display()));
    e.build();
    assert_ne!(sha(&e.icns()), first_sum, "different icon replaces the old");
    assert_ne!(e.bundle_id(), first_id, "different icon, different identity");

    e.config(&format!(r#"{{"icon":"{}"}}"#, a.display()));
    e.build();
    assert_eq!(e.bundle_id(), first_id, "returning to an icon returns to its identity");

    e.config(&format!(r#"{{"icon":"{GENERIC}"}}"#));
    e.build();
    assert_eq!(sha(&e.icns()), sha(Path::new(GENERIC)), "icns used as-is");

    e.config(r#"{"icon":"claude"}"#);
    e.build_with_icon(GENERIC);
    assert_eq!(sha(&e.icns()), sha(Path::new(GENERIC)), "\"claude\" uses the default icon");

    e.config(r#"{"icon":false}"#);
    e.build_with_icon(GENERIC);
    assert!(!e.icns().exists() && e.bundle_id() == generic_id, "icon false → generic");

    let missing = e.dir.path().join("missing.png");
    e.config(&format!(r#"{{"icon":"{}"}}"#, missing.display()));
    e.build();
    assert!(!e.icns().exists() && e.bundle_id() == generic_id, "missing icon → generic");
    let log = std::fs::read_to_string(e.home.join(".local/state/ntfyer/log")).unwrap_or_default();
    assert!(log.contains(&format!("icon not found: {}", missing.display())), "missing icon logged: {log}");

    std::fs::copy(&a, e.home.join(".config/ntfyer/rel.png")).expect("copy");
    e.config(r#"{"icon":"rel.png"}"#);
    let out = e.cmd(&["build"], None, "/none").current_dir(e.dir.path()).output().expect("build");
    assert!(out.status.success());
    let rel_id = e.bundle_id();
    assert!(e.icns().exists(), "relative icon resolves against the config dir");
    e.cmd(&["build"], None, "/none").current_dir("/").output().expect("build");
    assert_eq!(e.bundle_id(), rel_id, "same from any working directory");

    std::fs::remove_dir_all(e.app()).expect("rm app");
    e.cmd(&["build"], None, "/none").output().expect("build");
    assert!(signed(&e.app()) && e.app().join("Contents/Info.plist").is_file(), "deleted bundle rebuilt whole");
}

#[test]
fn build_lock_dead_and_live_owners() {
    let e = Env::new();
    let lock = e.app_dir().join(".build.lock");
    std::fs::create_dir_all(&lock).expect("lock");
    std::fs::write(lock.join("pid"), "999999").expect("pid");
    assert!(e.build().status.success(), "dead owner's lock taken over");
    assert!(!lock.exists(), "lock released");

    std::fs::remove_dir_all(e.app()).expect("rm app");
    std::fs::create_dir_all(&lock).expect("lock");
    std::fs::write(lock.join("pid"), std::process::id().to_string()).expect("pid");
    assert!(!e.build().status.success(), "live owner respected");
    std::fs::remove_dir_all(&lock).expect("unlock");
}

fn fake_bin(dir: &Path, name: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).expect("fake dir");
    let p = dir.join(name);
    std::fs::write(&p, script).expect("fake");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

#[test]
fn signal_never_compiles_inside_the_hook() {
    let e = Env::new();
    e.config(r#"{"bell":false,"sound":false}"#);
    let fake = e.dir.path().join("fakebin");
    let marker = e.dir.path().join("swiftc-ran");
    fake_bin(&fake, "swiftc", &format!("#!/bin/sh\ntouch '{}'\nsleep 4\nexit 1\n", marker.display()));
    let start = Instant::now();
    let out = e
        .cmd(&["signal", "--message", "m"], Some(&fake), "/none")
        .env("NTFYER_NO_FALLBACK", "1")
        .output()
        .expect("signal");
    assert_eq!(out.status.code(), Some(0), "signal always exits 0");
    assert!(start.elapsed() < Duration::from_secs(3), "returned in {:?}", start.elapsed());
    assert!(wait_until(Duration::from_secs(3), || marker.exists()), "compile started in the background");
    assert!(wait_until(Duration::from_secs(10), || !e.app_dir().join(".build.lock").exists()), "background build finished");
}

#[test]
fn build_without_command_line_tools_never_runs_the_shim() {
    let e = Env::new();
    let fake = e.dir.path().join("fakebin");
    let marker = e.dir.path().join("swiftc-ran");
    fake_bin(&fake, "xcode-select", "#!/bin/sh\nexit 2\n");
    fake_bin(&fake, "swiftc", &format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()));
    let out = e.cmd(&["build"], Some(&fake), "/none").output().expect("build");
    assert_eq!(out.status.code(), Some(1), "build fails");
    assert!(String::from_utf8_lossy(&out.stderr).contains("xcode-select --install"));
    assert!(!marker.exists(), "swiftc shim never invoked");
}
