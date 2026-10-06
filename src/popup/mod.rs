//! The popup channel: one backend per OS.

pub mod linux;
pub mod macos;
pub mod macos_app;

use std::path::Path;

/// Whether `prog` is an executable file on `$PATH`.
pub fn which(prog: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else { return false };
    std::env::split_paths(&path).any(|dir| is_executable(&dir.join(prog)))
}

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}
