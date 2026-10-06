use ntfyer::popup::linux::{gdbus_args, notify_send_args};
use std::path::Path;

fn v(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

#[test]
fn notify_send_args_with_and_without_icon() {
    assert_eq!(
        notify_send_args("T", "B", None),
        v(&["-a", "ntfyer", "--", "T", "B"])
    );
    assert_eq!(
        notify_send_args("T", "-B", Some(Path::new("/i.png"))),
        v(&["-a", "ntfyer", "-i", "/i.png", "--", "T", "-B"])
    );
}

#[test]
fn gdbus_args_shape() {
    let prefix = [
        "call",
        "--session",
        "--dest",
        "org.freedesktop.Notifications",
        "--object-path",
        "/org/freedesktop/Notifications",
        "--method",
        "org.freedesktop.Notifications.Notify",
        "--",
        "ntfyer",
        "0",
    ];
    let mut want = v(&prefix);
    want.extend(v(&["", "T", "B", "[]", "{}", "-1"]));
    assert_eq!(gdbus_args("T", "B", None), want);
    let mut want = v(&prefix);
    want.extend(v(&["/i.png", "T", "B", "[]", "{}", "-1"]));
    assert_eq!(gdbus_args("T", "B", Some(Path::new("/i.png"))), want);
}
