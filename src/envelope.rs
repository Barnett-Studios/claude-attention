//! The harness-neutral input: what an adapter (a Claude Code hook, a git hook, a script) hands to
//! `ntfyer signal --json` on stdin.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Envelope {
    pub message: Option<String>,
    pub title: Option<String>,
    pub project: Option<PathBuf>,
    pub event: Option<String>,
    pub session: Option<String>,
    /// the adapter decided this event must not signal (it is still logged)
    pub quiet: bool,
}

/// Empty or whitespace-only input, malformed JSON, a non-object, or a field of the wrong type →
/// `Envelope::default()`. Unknown fields are ignored.
pub fn parse(input: &str) -> Envelope {
    serde_json::from_str::<Envelope>(input).unwrap_or_default()
}

/// `"Finished and waiting for you."` when `event` equals `stop` ignoring ASCII case, otherwise
/// `"Needs your attention."`.
pub fn default_message(event: Option<&str>) -> &'static str {
    unimplemented!("delegated: default-message")
}

/// The project's display name: its last path component with `\n` and `\r` removed, or the whole
/// path (lossy) when it has no last component (e.g. `/`).
pub fn project_label(project: &Path) -> String {
    unimplemented!("delegated: project-label")
}
