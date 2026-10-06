//! Config: a global file overridden key by key by a project file. Every key is optional; a missing
//! file, a malformed file, a missing key and a `null` value all keep the default.

use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sound {
    Off,
    /// the OS's default chime
    Default,
    /// a system sound name, e.g. `Pop` on macOS or `complete` on Linux
    Name(String),
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Icon {
    /// the installed Claude desktop app's icon where there is one (macOS), else none
    Default,
    None,
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub enabled: bool,
    pub bell: bool,
    pub popup: bool,
    pub sound: Sound,
    pub icon: Icon,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            bell: true,
            popup: true,
            sound: Sound::Default,
            icon: Icon::Default,
        }
    }
}

/// `~` → `home`, `~/rest` → `home/rest`; anything else is returned unchanged.
pub fn expand_home(raw: &str, home: &Path) -> PathBuf {
    if raw == "~" {
        return home.to_path_buf();
    }
    if raw.starts_with("~/") {
        return home.join(&raw[2..]);
    }
    PathBuf::from(raw)
}

/// `false` → Off; `true` → Default; a string: empty → Default, starting with `~` or containing `/`
/// → File (with `~` expanded, and a relative path resolved against `config_dir`, so the result is
/// always absolute), otherwise Name. Any other JSON type → Default.
pub fn parse_sound(value: &Value, home: &Path, config_dir: &Path) -> Sound {
    match value {
        Value::Bool(false) => Sound::Off,
        Value::Bool(true) => Sound::Default,
        Value::String(s) => {
            if s.is_empty() {
                Sound::Default
            } else if s.starts_with('~') || s.contains('/') {
                let expanded = if s.starts_with('~') {
                    expand_home(s, home)
                } else {
                    config_dir.join(s)
                };
                Sound::File(expanded)
            } else {
                Sound::Name(s.clone())
            }
        }
        _ => Sound::Default,
    }
}

/// `false` → None; `true`, `"claude"` or `""` → Default; any other string → File, with `~` expanded
/// and a relative path resolved against `config_dir` (never against the caller's working
/// directory). Any other JSON type → Default.
pub fn parse_icon(value: &Value, home: &Path, config_dir: &Path) -> Icon {
    unimplemented!("delegated: parse-icon")
}

/// The file's JSON object, or `Value::Null` when the file is missing, unreadable, malformed or not
/// a JSON object.
pub fn read_layer(path: &Path) -> Value {
    unimplemented!("delegated: read-layer")
}

/// Starts from `Config::default()` and applies `global` then `project`, key by key, skipping keys
/// that are absent or `null` and layers that are not objects. `enabled` / `bell` / `popup` are
/// false only for JSON `false` (any other non-null value means true). `sound` goes through
/// `parse_sound`; a project layer's sound that parses to `File` is ignored (a cloned repo must not
/// make the machine play an arbitrary file), so only the global layer may name a sound file.
/// `icon` is taken from `global` only (one icon per machine), via `parse_icon`.
pub fn resolve(global: &Value, project: &Value, home: &Path, config_dir: &Path) -> Config {
    unimplemented!("delegated: resolve-config")
}
