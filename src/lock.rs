//! The build lock: an exclusive `flock` on a lock file. The kernel drops it when the holder exits —
//! even when it is killed by a hook timeout — so a lock can never be left stale.

use std::fs::{File, OpenOptions};
use std::path::Path;

#[derive(Debug)]
pub struct LockGuard {
    _file: File,
}

/// Creates the parent directories and the lock file if needed, then takes an exclusive,
/// non-blocking lock on it (`File::try_lock`). Returns None when another holder has it or the file
/// cannot be opened. The lock is released when the guard is dropped; the file itself stays.
pub fn acquire(path: &Path) -> Option<LockGuard> {
    unimplemented!("delegated: lock-acquire")
}

/// Opens the lock file for locking (shared by `acquire` and callers that only want to probe it).
pub fn open(path: &Path) -> std::io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
}
