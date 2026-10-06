//! Linux: freedesktop notifications. The icon is passed per notification, so — unlike macOS —
//! no per-icon app identity is needed.

use std::path::Path;
use std::process::{Command, Stdio};

pub const APP_NAME: &str = "ntfyer";

/// `-a ntfyer`, then `-i <icon>` when there is one, then `--`, `title`, `body`.
pub fn notify_send_args(title: &str, body: &str, icon: Option<&Path>) -> Vec<String> {
    unimplemented!("delegated: notify-send-args")
}

/// Arguments for `gdbus` calling `org.freedesktop.Notifications.Notify` on the session bus:
/// `call --session --dest org.freedesktop.Notifications --object-path /org/freedesktop/Notifications
/// --method org.freedesktop.Notifications.Notify ntfyer 0 <icon or ""> <title> <body> [] {} -1`
pub fn gdbus_args(title: &str, body: &str, icon: Option<&Path>) -> Vec<String> {
    unimplemented!("delegated: gdbus-args")
}

/// Posts through `notify-send`, falling back to `gdbus`. Returns whether either succeeded.
pub fn notify(title: &str, body: &str, icon: Option<&Path>) -> bool {
    let run = |prog: &str, args: Vec<String>| {
        super::which(prog)
            && Command::new(prog)
                .args(args)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
    };
    run("notify-send", notify_send_args(title, body, icon)) || run("gdbus", gdbus_args(title, body, icon))
}
