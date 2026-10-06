//! ntfyer — gets a human's attention when an agent harness, a script or a git hook needs them:
//! a desktop popup, a chime and the terminal bell, on macOS and Linux.
//!
//! Fail-open by contract: [`signal::run`] never errors and the `ntfyer signal` CLI always exits 0.
//! The contract (front doors, envelope, config, exit codes) is in CONTRACT.md.

pub mod config;
pub mod doctor;
pub mod envelope;
pub mod debounce;
pub mod lock;
pub mod log;
pub mod paths;
pub mod popup;
pub mod signal;
pub mod sound;
