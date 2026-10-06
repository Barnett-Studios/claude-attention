use ntfyer::envelope::{default_message, parse, project_label, Envelope};
use std::path::{Path, PathBuf};

#[test]
fn parse_envelope_full() {
    let e = parse(
        r#"{"message":"m","title":"t","project":"/p/x","event":"stop","session":"s1","quiet":true,"extra":1}"#,
    );
    assert_eq!(
        e,
        Envelope {
            message: Some("m".into()),
            title: Some("t".into()),
            project: Some(PathBuf::from("/p/x")),
            event: Some("stop".into()),
            session: Some("s1".into()),
            quiet: true,
        }
    );
}

#[test]
fn parse_envelope_partial_and_bad() {
    assert_eq!(parse(r#"{"message":"m"}"#).message.as_deref(), Some("m"));
    assert!(!parse(r#"{"message":"m"}"#).quiet);
    assert_eq!(parse(""), Envelope::default());
    assert_eq!(parse("   \n"), Envelope::default());
    assert_eq!(parse("not json"), Envelope::default());
    assert_eq!(parse("[1]"), Envelope::default());
    assert_eq!(parse(r#"{"quiet":"yes"}"#), Envelope::default());
}

#[test]
fn default_message_cases() {
    assert_eq!(
        default_message(Some("stop")),
        "Finished and waiting for you."
    );
    assert_eq!(
        default_message(Some("Stop")),
        "Finished and waiting for you."
    );
    assert_eq!(
        default_message(Some("notification")),
        "Needs your attention."
    );
    assert_eq!(default_message(None), "Needs your attention.");
}

#[test]
fn project_label_cases() {
    assert_eq!(project_label(Path::new("/a/proj")), "proj");
    assert_eq!(project_label(Path::new("/a/pr\noj\r")), "proj");
    assert_eq!(project_label(Path::new("/")), "/");
}
