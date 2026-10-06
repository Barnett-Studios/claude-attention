//! A directory lock owned by a pid. A lock whose owner is gone (killed by a hook timeout, a crash,
//! a reboot) is taken over instead of blocking every later build.

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct LockGuard {
    dir: PathBuf,
}

impl LockGuard {
    pub fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Creates the parent directories, then `mkdir dir`: on success writes `pid` to `dir/pid` and
/// returns the guard. When `dir` already exists: if `dir/pid` holds a number for which `alive` is
/// true → None; otherwise removes `dir` and retries `mkdir` once (None if that fails too).
pub fn acquire(dir: &Path, pid: u32, alive: &dyn Fn(u32) -> bool) -> Option<LockGuard> {
    unimplemented!("delegated: lock-acquire")
}

/// Whether a process with this pid exists (`kill -0`).
pub fn pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
