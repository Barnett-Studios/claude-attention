//! Where ntfyer keeps its config, state and (macOS) notifier app — the same layout on every OS.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub home: PathBuf,
    /// `$XDG_CONFIG_HOME/ntfyer`, else `$HOME/.config/ntfyer`
    pub config_dir: PathBuf,
    /// `$XDG_STATE_HOME/ntfyer`, else `$HOME/.local/state/ntfyer`
    pub state_dir: PathBuf,
    /// `$HOME/Library/Application Support/ntfyer` (only used on macOS)
    pub app_dir: PathBuf,
}

impl Paths {
    /// Resolves the layout from an environment lookup. `HOME` unset or empty → `None`.
    /// An empty `XDG_CONFIG_HOME` / `XDG_STATE_HOME` counts as unset.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Option<Paths> {
        let home = get("HOME")?;
        if home.is_empty() {
            return None;
        }
        let home = PathBuf::from(home);

        let config_dir = if let Some(val) = get("XDG_CONFIG_HOME") {
            if val.is_empty() {
                home.join(".config").join("ntfyer")
            } else {
                PathBuf::from(val).join("ntfyer")
            }
        } else {
            home.join(".config").join("ntfyer")
        };

        let state_dir = if let Some(val) = get("XDG_STATE_HOME") {
            if val.is_empty() {
                home.join(".local").join("state").join("ntfyer")
            } else {
                PathBuf::from(val).join("ntfyer")
            }
        } else {
            home.join(".local").join("state").join("ntfyer")
        };

        let app_dir = home.join("Library").join("Application Support").join("ntfyer");

        Some(Paths {
            home,
            config_dir,
            state_dir,
            app_dir,
        })
    }

    pub fn from_env() -> Option<Paths> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    pub fn global_config(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }

    pub fn log_file(&self) -> PathBuf {
        self.state_dir.join("log")
    }
}

/// The per-project override file.
pub fn project_config(project: &Path) -> PathBuf {
    project.join(".ntfyer.json")
}
