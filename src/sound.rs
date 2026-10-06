//! Which file the `sound` setting names on this OS, and which player can play it.

use crate::config::Sound;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Linux,
}

/// The OS this binary was built for, or `None` where ntfyer has no desktop backends.
pub fn current_os() -> Option<Os> {
    if cfg!(target_os = "macos") {
        Some(Os::MacOs)
    } else if cfg!(target_os = "linux") {
        Some(Os::Linux)
    } else {
        None
    }
}

/// Off → None. Default → the name `Glass` (macOS) / `message-new-instant` (Linux). Name(n) →
/// `/System/Library/Sounds/{n}.aiff` (macOS) / `/usr/share/sounds/freedesktop/stereo/{n}.oga`
/// (Linux). File(p) → p. The result is returned only when `exists` says the file is there.
pub fn resolve(sound: &Sound, os: Os, exists: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    let named = |n: &str| match os {
        Os::MacOs => PathBuf::from(format!("/System/Library/Sounds/{n}.aiff")),
        Os::Linux => PathBuf::from(format!("/usr/share/sounds/freedesktop/stereo/{n}.oga")),
    };
    let file = match sound {
        Sound::Off => return None,
        Sound::Default => named(match os {
            Os::MacOs => "Glass",
            Os::Linux => "message-new-instant",
        }),
        Sound::Name(n) => named(n),
        Sound::File(p) => p.clone(),
    };
    exists(&file).then_some(file)
}

/// The command that plays `file`: macOS `afplay <file>`; Linux the first of `pw-play`, `paplay`,
/// `aplay` for which `have` is true, followed by `<file>`. None when no player is available.
pub fn player(os: Os, file: &Path, have: &dyn Fn(&str) -> bool) -> Option<Vec<String>> {
    let candidates: &[&str] = match os {
        Os::MacOs => &["afplay"],
        Os::Linux => &["pw-play", "paplay", "aplay"],
    };
    let prog = candidates.iter().find(|p| have(p))?;
    Some(vec![prog.to_string(), file.to_string_lossy().into_owned()])
}
