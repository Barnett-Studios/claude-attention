use ntfyer::config::Sound;
use ntfyer::sound::{player, resolve, Os};
use std::path::{Path, PathBuf};

fn yes(_: &Path) -> bool {
    true
}
fn no(_: &Path) -> bool {
    false
}

#[test]
fn resolve_macos() {
    assert_eq!(resolve(&Sound::Off, Os::MacOs, &yes), None);
    assert_eq!(resolve(&Sound::Default, Os::MacOs, &yes), Some(PathBuf::from("/System/Library/Sounds/Glass.aiff")));
    assert_eq!(resolve(&Sound::Name("Pop".into()), Os::MacOs, &yes), Some(PathBuf::from("/System/Library/Sounds/Pop.aiff")));
    assert_eq!(resolve(&Sound::File("/x/a.wav".into()), Os::MacOs, &yes), Some(PathBuf::from("/x/a.wav")));
}

#[test]
fn resolve_linux() {
    assert_eq!(
        resolve(&Sound::Default, Os::Linux, &yes),
        Some(PathBuf::from("/usr/share/sounds/freedesktop/stereo/message-new-instant.oga"))
    );
    assert_eq!(
        resolve(&Sound::Name("complete".into()), Os::Linux, &yes),
        Some(PathBuf::from("/usr/share/sounds/freedesktop/stereo/complete.oga"))
    );
}

#[test]
fn resolve_requires_existing_file() {
    assert_eq!(resolve(&Sound::Name("NoSuch".into()), Os::MacOs, &no), None);
    assert_eq!(resolve(&Sound::File("/x/a.wav".into()), Os::Linux, &no), None);
}

#[test]
fn player_macos() {
    let f = Path::new("/s/a.aiff");
    assert_eq!(player(Os::MacOs, f, &|p| p == "afplay"), Some(vec!["afplay".into(), "/s/a.aiff".into()]));
    assert_eq!(player(Os::MacOs, f, &|_| false), None);
}

#[test]
fn player_linux_preference_order() {
    let f = Path::new("/s/a.oga");
    assert_eq!(player(Os::Linux, f, &|_| true), Some(vec!["pw-play".into(), "/s/a.oga".into()]));
    assert_eq!(player(Os::Linux, f, &|p| p != "pw-play"), Some(vec!["paplay".into(), "/s/a.oga".into()]));
    assert_eq!(player(Os::Linux, f, &|p| p == "aplay"), Some(vec!["aplay".into(), "/s/a.oga".into()]));
    assert_eq!(player(Os::Linux, f, &|_| false), None);
}
