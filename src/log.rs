//! Audit trail: one line per decision, so a stray or missing signal can be traced to its input.

use std::path::Path;

pub const KEEP_LINES: usize = 200;

/// Replaces every control character (`char::is_control`, which covers `\n`, `\r` and ESC) and
/// U+2028 / U+2029 with a space, so a field can neither forge a log line nor smuggle terminal
/// escape sequences into whoever tails the log.
pub fn sanitize(field: &str) -> String {
    field
        .chars()
        .map(|c| {
            if c.is_control() || c == '\u{2028}' || c == '\u{2029}' {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// Creates parent directories, appends `line` plus `\n`, then — when the file holds more than
/// `KEEP_LINES` lines — rewrites it to the last `KEEP_LINES` lines through a temp file named
/// `<file name>.<process id>` in the same directory, renamed over the original. Every error is
/// ignored (logging must never break a signal).
pub fn append(log: &Path, line: &str) {
    use std::io::Write;
    if let Some(parent) = log.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let appended = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .and_then(|mut f| writeln!(f, "{line}"));
    if appended.is_err() {
        return;
    }
    let Ok(text) = std::fs::read_to_string(log) else {
        return;
    };
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= KEEP_LINES {
        return;
    }
    let Some(name) = log.file_name() else { return };
    let tmp = log.with_file_name(format!("{}.{}", name.to_string_lossy(), std::process::id()));
    let kept: String = lines[lines.len() - KEEP_LINES..]
        .iter()
        .map(|l| format!("{l}\n"))
        .collect();
    if std::fs::write(&tmp, kept).is_err() || std::fs::rename(&tmp, log).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}
