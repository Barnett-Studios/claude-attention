//! Building, refreshing and driving the macOS notifier bundle. The pure parts (identity, plist,
//! digests, argument building) live in `macos.rs`; this module does the filesystem and process work.

use super::macos::{icon_digest, icon_source, identity, info_plist, source_digest, SWIFT_SOURCE};
use crate::config::Icon;
use crate::lock;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// How long a popup may take: a notifier still waiting on the permission prompt is killed and the
/// caller falls back.
pub const NOTIFIER_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

pub const LSREGISTER: &str =
    "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";

/// Inputs to a build that do not come from the config file.
pub struct BuildEnv<'a> {
    pub app_dir: &'a Path,
    /// the icon `"claude"` means (normally the Claude desktop app's)
    pub default_icon: &'a Path,
    /// register with LaunchServices (off in tests: stale registrations confuse Notification Center)
    pub register: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Built {
    UpToDate,
    /// a bundle was (re)assembled; `compiled` says whether the Swift binary was rebuilt too
    Fresh {
        compiled: bool,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuildError {
    /// the binary is missing or stale and the caller forbade compiling (it runs inside a hook)
    NeedsCompile,
    Locked,
    Toolchain,
    Failed(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::NeedsCompile => {
                write!(f, "the notifier needs compiling (run: ntfyer build)")
            }
            BuildError::Locked => write!(f, "another build is running"),
            BuildError::Toolchain => write!(
                f,
                "Xcode Command Line Tools not installed (run: xcode-select --install)"
            ),
            BuildError::Failed(why) => write!(f, "build failed: {why}"),
        }
    }
}

/// The bundle the current icon maps to, building whatever is missing. `on_missing_icon` is told
/// about a configured icon file that does not exist (the build then uses the generic icon).
pub fn ensure(
    env: &BuildEnv,
    icon: &Icon,
    allow_compile: bool,
    on_missing_icon: &dyn Fn(&Path),
) -> Result<(PathBuf, Built), BuildError> {
    let exists = |p: &Path| p.is_file();
    if let Icon::File(p) = icon {
        if !exists(p) {
            on_missing_icon(p);
        }
    }
    let icon_file = icon_source(icon, env.default_icon, &exists);
    let id = identity(&icon_digest(icon_file.as_deref()));
    let app = env.app_dir.join(&id.bundle_name);
    let cache = env.app_dir.join("build");
    let bin = cache.join("Ntfyer");
    let stamp = cache.join("src-digest");
    let want_src = source_digest();

    let freshness = || {
        let bin_stale =
            !(bin.is_file() && fs::read_to_string(&stamp).is_ok_and(|s| s.trim() == want_src));
        let app_stale = !(app.join("Contents/MacOS/Ntfyer").is_file()
            && app.join("Contents/Info.plist").is_file());
        (bin_stale, app_stale)
    };
    let (fresh_bin, fresh_app) = freshness();
    if !fresh_bin && !fresh_app {
        return Ok((app, Built::UpToDate));
    }
    if fresh_bin && !allow_compile {
        return Err(BuildError::NeedsCompile);
    }

    let _lock = lock::acquire(&env.app_dir.join(".build.lock")).ok_or(BuildError::Locked)?;
    // another build may have finished while we were deciding: never rebuild (and swap out) a
    // bundle that a concurrent notify could be running
    let (fresh_bin, fresh_app) = freshness();
    if !fresh_bin && !fresh_app {
        return Ok((app, Built::UpToDate));
    }
    let scratch = Scratch::new(env.app_dir.join(format!(".tmp-{}", std::process::id())))?;

    if fresh_bin {
        if !have_toolchain() {
            return Err(BuildError::Toolchain);
        }
        let src = scratch.0.join("Ntfyer.swift");
        fs::write(&src, SWIFT_SOURCE).map_err(fail("write swift source"))?;
        let out = scratch.0.join("Ntfyer");
        run(
            Command::new("swiftc")
                .arg("-O")
                .arg(&src)
                .arg("-o")
                .arg(&out),
            "swiftc",
        )?;
        fs::create_dir_all(&cache).map_err(fail("create build cache"))?;
        fs::rename(&out, &bin).map_err(fail("install binary"))?;
        fs::write(&stamp, &want_src).map_err(fail("write source stamp"))?;
    }

    // assemble the bundle off to the side, then swap it in whole
    let stage = scratch.0.join("Ntfyer.app");
    fs::create_dir_all(stage.join("Contents/MacOS")).map_err(fail("create bundle"))?;
    fs::create_dir_all(stage.join("Contents/Resources")).map_err(fail("create bundle"))?;
    fs::copy(&bin, stage.join("Contents/MacOS/Ntfyer")).map_err(fail("copy binary"))?;
    if let Some(f) = &icon_file {
        let dest = stage.join("Contents/Resources/AppIcon.icns");
        if f.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("icns"))
        {
            fs::copy(f, &dest).map_err(fail("copy icon"))?;
        } else {
            run(
                Command::new("sips")
                    .args(["-z", "1024", "1024", "-s", "format", "icns"])
                    .arg(f)
                    .arg("--out")
                    .arg(&dest),
                "sips icon conversion",
            )?;
        }
    }
    fs::write(
        stage.join("Contents/Info.plist"),
        info_plist(&id.bundle_id, icon_file.is_some()),
    )
    .map_err(fail("write Info.plist"))?;
    run(
        Command::new("codesign")
            .args(["--force", "-s", "-"])
            .arg(&stage),
        "codesign",
    )?;
    if app.exists() {
        fs::remove_dir_all(&app).map_err(fail("remove old bundle"))?;
    }
    fs::rename(&stage, &app).map_err(fail("install bundle"))?;

    // retire every other icon's bundle, so only the current one is registered and listed
    if let Ok(entries) = fs::read_dir(env.app_dir) {
        for old in entries.flatten().map(|e| e.path()) {
            let name = old
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if old != app && name.starts_with("Ntfyer-") && name.ends_with(".app") {
                if env.register {
                    let _ = Command::new(LSREGISTER)
                        .arg("-u")
                        .arg(&old)
                        .stderr(Stdio::null())
                        .status();
                }
                let _ = fs::remove_dir_all(&old);
            }
        }
    }
    if env.register {
        let _ = Command::new(LSREGISTER)
            .arg("-f")
            .arg(&app)
            .stderr(Stdio::null())
            .status();
    }
    Ok((
        app,
        Built::Fresh {
            compiled: fresh_bin,
        },
    ))
}

/// Posts through the bundle. Never compiles: a missing or stale binary starts a detached
/// `ntfyer build` and returns false so the caller falls back. Returns whether the popup posted.
pub fn notify(
    env: &BuildEnv,
    icon: &Icon,
    title: &str,
    body: &str,
    group: &str,
    on_missing_icon: &dyn Fn(&Path),
) -> bool {
    match ensure(env, icon, false, on_missing_icon) {
        Ok((app, _)) => {
            let status = super::run_with_deadline(
                Command::new(app.join("Contents/MacOS/Ntfyer"))
                    .args([title, body, group])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null()),
                NOTIFIER_DEADLINE,
            );
            match status.map(|s| s.code()) {
                Some(Some(0)) => true,
                Some(Some(3)) => {
                    // not allowed yet: a LaunchServices launch registers the app with Notification
                    // Center so the user can allow it; this popup still falls back
                    let _ = Command::new("open")
                        .args(["-g", "-n", "-a"])
                        .arg(&app)
                        .args(["--args", title, body, group])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status();
                    false
                }
                _ => false,
            }
        }
        Err(BuildError::NeedsCompile) => {
            spawn_detached_build();
            false
        }
        Err(_) => false,
    }
}

fn spawn_detached_build() {
    use std::os::unix::process::CommandExt;
    if let Ok(exe) = std::env::current_exe() {
        // its own process group, so a hook timeout that kills ours does not take the compile down
        let _ = Command::new(exe)
            .arg("build")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn();
    }
}

/// `/usr/bin/swiftc` exists even without the Command Line Tools: it is a shim that pops the
/// install dialog, so ask `xcode-select` first.
pub fn have_toolchain() -> bool {
    let tools = Command::new("xcode-select")
        .arg("-p")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    tools && super::which("swiftc")
}

fn run(cmd: &mut Command, what: &str) -> Result<(), BuildError> {
    match cmd.stdout(Stdio::null()).stderr(Stdio::piped()).output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(BuildError::Failed(format!(
            "{what}: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ))),
        Err(e) => Err(BuildError::Failed(format!("{what}: {e}"))),
    }
}

fn fail(what: &'static str) -> impl Fn(std::io::Error) -> BuildError {
    move |e| BuildError::Failed(format!("{what}: {e}"))
}

/// A scratch directory removed on every exit path.
struct Scratch(PathBuf);

impl Scratch {
    fn new(dir: PathBuf) -> Result<Scratch, BuildError> {
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).map_err(fail("create scratch dir"))?;
        Ok(Scratch(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
