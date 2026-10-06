use ntfyer::popup::run_with_deadline;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn deadline_kills_a_hung_child() {
    let start = Instant::now();
    let status = run_with_deadline(Command::new("sleep").arg("10"), Duration::from_millis(500));
    assert!(status.is_none(), "timed out");
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "took {:?}",
        start.elapsed()
    );
}

#[test]
fn deadline_returns_the_status_of_a_quick_child() {
    let ok =
        run_with_deadline(&mut Command::new("true"), Duration::from_secs(5)).expect("finished");
    assert!(ok.success());
    let bad =
        run_with_deadline(&mut Command::new("false"), Duration::from_secs(5)).expect("finished");
    assert_eq!(bad.code(), Some(1));
}

#[test]
fn deadline_reports_a_missing_program_as_none() {
    assert!(run_with_deadline(
        &mut Command::new("/definitely/not/here"),
        Duration::from_secs(1)
    )
    .is_none());
}
