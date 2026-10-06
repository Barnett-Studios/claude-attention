//! One signal: decide whether to fire, log the decision, then fire the bell, the chime and the
//! popup. Never fails — every error degrades to a channel not firing.

use crate::config::{self, Config, Icon};
use crate::debounce;
use crate::envelope::{self, Envelope};
use crate::log;
use crate::paths::{self, Paths};
use crate::popup::{self, linux, macos, macos_app};
use crate::sound::{self, Os};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const CLAUDE_ICON: &str = "/Applications/Claude.app/Contents/Resources/electron.icns";

/// Everything a signal needs from its surroundings, so tests can pin it.
pub struct Context {
    pub paths: Paths,
    pub os: Option<Os>,
    pub cwd: PathBuf,
    pub now: u64,
    /// NTFYER_DRY_RUN=1: print each channel instead of firing it
    pub dry_run: bool,
    /// NTFYER=off
    pub off: bool,
    /// NTFYER_NO_FALLBACK=1: no osascript fallback popup (test seam)
    pub no_fallback: bool,
    /// NTFYER_REGISTER=0: never touch LaunchServices (test seam)
    pub register: bool,
    /// NTFYER_DEFAULT_ICON, else the Claude desktop app's icon
    pub default_icon: PathBuf,
}

impl Context {
    /// None when HOME is unset — there is nowhere to read config from.
    pub fn from_env() -> Option<Context> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        Some(Context {
            paths: Paths::from_env()?,
            os: sound::current_os(),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            now: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            dry_run: var("NTFYER_DRY_RUN").is_some_and(|v| v != "0"),
            off: var("NTFYER").is_some_and(|v| v == "off"),
            no_fallback: var("NTFYER_NO_FALLBACK").is_some_and(|v| v != "0"),
            register: var("NTFYER_REGISTER").is_none_or(|v| v != "0"),
            default_icon: var("NTFYER_DEFAULT_ICON")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(CLAUDE_ICON)),
        })
    }

    pub fn build_env(&self) -> macos_app::BuildEnv<'_> {
        macos_app::BuildEnv {
            app_dir: &self.paths.app_dir,
            default_icon: &self.default_icon,
            register: self.register,
        }
    }

    /// The effective config for `project`.
    pub fn config_for(&self, project: &Path) -> Config {
        config::resolve(
            &config::read_layer(&self.paths.global_config()),
            &config::read_layer(&paths::project_config(project)),
            &self.paths.home,
            &self.paths.config_dir,
        )
    }

    /// Logs that a configured icon file is missing.
    pub fn log_missing_icon(&self, icon: &Path) {
        log::append(
            &self.paths.log_file(),
            &format!(
                "{} notifier: icon not found: {} (using the generic icon)",
                timestamp(self.now),
                log::sanitize(&icon.to_string_lossy())
            ),
        );
    }
}

/// Runs one signal and returns the logged verdict (`signal` or `skip:<reason>`). Dry-run channel
/// lines go to `out`.
pub fn run(env: &Envelope, ctx: &Context, out: &mut dyn Write) -> String {
    let raw = env.project.clone().unwrap_or_else(|| ctx.cwd.clone());
    // `.` and its absolute path are the same project: same label, config and debounce key
    let project = std::fs::canonicalize(ctx.cwd.join(&raw)).unwrap_or(raw);
    let label = envelope::project_label(&project);
    let cfg = ctx.config_for(&project);

    let verdict = if ctx.off {
        "skip:env-off"
    } else if env.quiet {
        "skip:quiet"
    } else if !cfg.enabled {
        "skip:disabled"
    } else if !debounce::admit(
        &debounce::stamp_path(&ctx.paths.state_dir, &project),
        ctx.now,
        debounce::WINDOW_SECS,
    ) {
        "skip:debounce"
    } else {
        "signal"
    };
    log::append(
        &ctx.paths.log_file(),
        &format!(
            "{} event={} verdict={verdict} project={} session={}",
            timestamp(ctx.now),
            log::sanitize(env.event.as_deref().unwrap_or("manual")),
            log::sanitize(&label),
            log::sanitize(env.session.as_deref().unwrap_or("-")),
        ),
    );
    if verdict != "signal" {
        return verdict.to_string();
    }

    let message = env
        .message
        .clone()
        .unwrap_or_else(|| envelope::default_message(env.event.as_deref()).to_string());
    let title = env.title.clone().unwrap_or_else(|| label.clone());

    if cfg.bell {
        if ctx.dry_run {
            let _ = writeln!(out, "bell");
        } else {
            ring_bell();
        }
    }
    if let Some(os) = ctx.os {
        if let Some(file) = sound::resolve(&cfg.sound, os, &|p: &Path| p.is_file()) {
            if ctx.dry_run {
                let _ = writeln!(out, "sound:{}", file.display());
            } else if let Some(cmd) = sound::player(os, &file, &popup::which) {
                let _ = Command::new(&cmd[0])
                    .args(&cmd[1..])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            }
        }
    }
    if cfg.popup {
        if ctx.dry_run {
            let _ = writeln!(out, "popup:{title}|{message}");
        } else {
            show_popup(ctx, &cfg, &title, &message, &format!("ntfyer-{label}"));
        }
    }
    verdict.to_string()
}

/// Rings the controlling terminal's bell. Without one (the usual case inside a hook) there is no
/// bell: stdout belongs to the caller, which may read it, so nothing is ever written there.
fn ring_bell() {
    if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        let _ = tty.write_all(b"\x07");
    }
}

fn show_popup(ctx: &Context, cfg: &Config, title: &str, message: &str, group: &str) {
    match ctx.os {
        Some(Os::MacOs) => {
            let posted =
                macos_app::notify(&ctx.build_env(), &cfg.icon, title, message, group, &|p| {
                    ctx.log_missing_icon(p)
                });
            if !posted && !ctx.no_fallback {
                let _ = Command::new("osascript")
                    .args(macos::osascript_args(title, message))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
        Some(Os::Linux) => {
            let icon = match &cfg.icon {
                Icon::File(p) if p.is_file() => Some(p.as_path()),
                Icon::File(p) => {
                    ctx.log_missing_icon(p);
                    None
                }
                Icon::Default | Icon::None => None,
            };
            linux::notify(title, message, icon);
        }
        None => {}
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a Unix time (UTC).
pub fn timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // days since 1970-01-01 → civil date (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::timestamp;

    #[test]
    fn timestamp_formats_utc() {
        assert_eq!(timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(timestamp(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(timestamp(1_791_272_096), "2026-10-06T07:34:56Z");
    }
}
