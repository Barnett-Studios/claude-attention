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

/// Empty or whitespace-only input, malformed JSON or a non-object → `Envelope::default()`. Each
/// field is read on its own: one of the wrong type is treated as absent without discarding the
/// others (a malformed `title` must not drop `quiet`). Unknown fields are ignored.
pub fn parse(input: &str) -> Envelope {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(input)
    else {
        return Envelope::default();
    };
    let text = |key: &str| map.get(key).and_then(|v| v.as_str()).map(str::to_string);
    Envelope {
        message: text("message"),
        title: text("title"),
        project: text("project").map(PathBuf::from),
        event: text("event"),
        session: text("session"),
        quiet: map.get("quiet").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

/// `"Finished and waiting for you."` when `event` equals `stop` ignoring ASCII case, otherwise
/// `"Needs your attention."`.
pub fn default_message(event: Option<&str>) -> &'static str {
    match event {
        Some(e) if e.eq_ignore_ascii_case("stop") => "Finished and waiting for you.",
        _ => "Needs your attention.",
    }
}

/// The project's display name: its last path component with `\n` and `\r` removed, or the whole
/// path (lossy) when it has no last component (e.g. `/`).
pub fn project_label(project: &Path) -> String {
    if let Some(last) = project.components().next_back() {
        let s = last.as_os_str().to_string_lossy().to_string();
        s.replace(['\n', '\r'], "")
    } else {
        project.to_string_lossy().to_string()
    }
}
