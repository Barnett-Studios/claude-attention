//! Linux: freedesktop notifications. The icon is passed per notification, so — unlike macOS —
//! no per-icon app identity is needed.

use std::path::Path;
use std::process::{Command, Stdio};

pub const APP_NAME: &str = "ntfyer";

/// `-a ntfyer`, then `-i <icon>` when there is one, then `--`, `title`, `body`.
pub fn notify_send_args(title: &str, body: &str, icon: Option<&Path>) -> Vec<String> {
    let mut args = vec!["-a".to_string(), APP_NAME.to_string()];
    if let Some(i) = icon {
        args.push("-i".to_string());
        args.push(i.to_string_lossy().into_owned());
    }
    args.extend(["--".to_string(), title.to_string(), body.to_string()]);
    args
}

/// Arguments for `gdbus` calling `org.freedesktop.Notifications.Notify` on the session bus:
/// `call --session --dest org.freedesktop.Notifications --object-path /org/freedesktop/Notifications
/// --method org.freedesktop.Notifications.Notify -- ntfyer 0 <icon or ""> <title> <body> [] {} -1`
/// (`--` so a title or body starting with `-` is never read as an option)
pub fn gdbus_args(title: &str, body: &str, icon: Option<&Path>) -> Vec<String> {
    let args = vec![
        "call".to_string(),
        "--session".to_string(),
        "--dest".to_string(),
        "org.freedesktop.Notifications".to_string(),
        "--object-path".to_string(),
        "/org/freedesktop/Notifications".to_string(),
        "--method".to_string(),
        "org.freedesktop.Notifications.Notify".to_string(),
        "--".to_string(),
        APP_NAME.to_string(),
        "0".to_string(),
        icon.map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        title.to_string(),
        body.to_string(),
        "[]".to_string(),
        "{}".to_string(),
        "-1".to_string(),
    ];
    args
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
    run("notify-send", notify_send_args(title, body, icon))
        || run("gdbus", gdbus_args(title, body, icon))
}
