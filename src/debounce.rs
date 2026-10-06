//! One signal per burst per project, so a manual call followed by a hook never doubles up while
//! another project's session still gets through.

use std::path::{Path, PathBuf};

pub const WINDOW_SECS: u64 = 8;

/// `state_dir/debounce/<first 12 lowercase hex chars of sha256(project path bytes)>.last`
pub fn stamp_path(state_dir: &Path, project: &Path) -> PathBuf {
    unimplemented!("delegated: stamp-path")
}

/// Reads the last-signal time from `stamp` (whole seconds; missing or unparseable = 0). Inside the
/// window (`now - last < window`, saturating) → `false` and the stamp is left alone. Otherwise
/// creates the parent directories, writes `now`, and returns `true` — also when writing fails
/// (fail-open: better a double signal than a missed one).
pub fn admit(stamp: &Path, now: u64, window: u64) -> bool {
    unimplemented!("delegated: debounce-admit")
}
