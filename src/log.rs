//! Audit trail: one line per decision, so a stray or missing signal can be traced to its input.

use std::path::Path;

pub const KEEP_LINES: usize = 200;

/// Replaces every control character (`char::is_control`, which covers `\n`, `\r` and ESC) and
/// U+2028 / U+2029 with a space, so a field can neither forge a log line nor smuggle terminal
/// escape sequences into whoever tails the log.
pub fn sanitize(field: &str) -> String {
    unimplemented!("delegated: log-sanitize")
}

/// Creates parent directories, appends `line` plus `\n`, then — when the file holds more than
/// `KEEP_LINES` lines — rewrites it to the last `KEEP_LINES` lines through a temp file named
/// `<file name>.<process id>` in the same directory, renamed over the original. Every error is
/// ignored (logging must never break a signal).
pub fn append(log: &Path, line: &str) {
    unimplemented!("delegated: log-append")
}
