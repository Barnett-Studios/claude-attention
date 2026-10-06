//! `ntfyer doctor`: what this machine can do, as text or as the family's JSON envelope.

use crate::paths;
use crate::popup::{self, macos_app};
use crate::signal::Context;
use crate::sound::Os;
use serde_json::{json, Value};

/// The report body, and whether at least one popup path works.
pub fn report(ctx: &Context) -> (Value, bool) {
    let (backends, healthy) = match ctx.os {
        Some(Os::MacOs) => {
            let built = ctx.paths.app_dir.join("build/Ntfyer").is_file();
            let toolchain = macos_app::have_toolchain();
            let b = json!({
                "notifier_built": built,
                "toolchain": toolchain,
                "osascript": popup::which("osascript"),
                "afplay": popup::which("afplay"),
            });
            (b, built || toolchain || popup::which("osascript"))
        }
        Some(Os::Linux) => {
            let names = ["notify-send", "gdbus", "pw-play", "paplay", "aplay"];
            let b: serde_json::Map<String, Value> = names.iter().map(|n| (n.to_string(), json!(popup::which(n)))).collect();
            let healthy = popup::which("notify-send") || popup::which("gdbus");
            (Value::Object(b), healthy)
        }
        None => (json!({}), false),
    };
    let global = ctx.paths.global_config();
    let body = json!({
        "os": match ctx.os { Some(Os::MacOs) => "macos", Some(Os::Linux) => "linux", None => "unsupported" },
        "healthy": healthy,
        "config": {
            "global": global.display().to_string(),
            "global_exists": global.is_file(),
            "project": paths::project_config(&ctx.cwd).display().to_string(),
        },
        "log": ctx.paths.log_file().display().to_string(),
        "backends": backends,
    });
    (body, healthy)
}

/// `{"schema_version":"1","status":"ok","body":…}`
pub fn envelope(body: Value) -> Value {
    json!({"schema_version": "1", "status": "ok", "body": body})
}

pub fn text(body: &Value) -> String {
    let mut out = String::new();
    let mut walk = |prefix: &str, v: &Value| {
        if let Value::Object(m) = v {
            for (k, v) in m {
                out.push_str(&format!("{prefix}{k}: {}\n", v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())));
            }
        }
    };
    for key in ["os", "healthy", "log"] {
        if let Some(v) = body.get(key) {
            walk("", &json!({ key: v }));
        }
    }
    walk("config.", &body["config"]);
    walk("backend.", &body["backends"]);
    out
}
