//! The popup channel: one backend per OS.

pub mod linux;
pub mod macos;
pub mod macos_app;

use std::path::Path;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

/// Runs `cmd` and waits at most `limit`. A child still running then is killed and reaped, and the
/// result is None — as it is when the program cannot be started. Hooks run on a short clock, so
/// nothing ntfyer launches may outlive it (a pending macOS permission prompt otherwise holds the
/// notifier for up to a minute).
pub fn run_with_deadline(cmd: &mut Command, limit: Duration) -> Option<ExitStatus> {
    let mut child = cmd.spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if start.elapsed() < limit => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Whether `prog` is an executable file on `$PATH`.
pub fn which(prog: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| is_executable(&dir.join(prog)))
}

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
