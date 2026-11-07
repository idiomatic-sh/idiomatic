//! Proves which pager a long terminal answer goes through, and when it goes straight to stdout.

use super::{pager, spawned};

/// The program and the arguments a pager is started with.
fn started(variable: Option<&str>) -> Option<(String, Vec<String>)> {
    pager(variable.map(std::ffi::OsString::from)).map(|command| {
        (
            command.get_program().to_string_lossy().into_owned(),
            command
                .get_args()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect(),
        )
    })
}

/// With no `PAGER`, `less -FRX` pages.
#[test]
fn pager_succeeds_with_less_when_none_is_named() {
    assert_eq!(
        started(None),
        Some(("less".to_owned(), vec!["-FRX".to_owned()]))
    );
}

/// A `PAGER` is split on whitespace into the program and its arguments.
#[test]
fn pager_succeeds_with_the_one_named() {
    assert_eq!(
        started(Some("less -R")),
        Some(("less".to_owned(), vec!["-R".to_owned()]))
    );
    assert_eq!(started(Some("bat")), Some(("bat".to_owned(), Vec::new())));
}

/// An empty `PAGER` asks for no pager.
#[test]
fn pager_refuses_an_empty_variable() {
    assert_eq!(started(Some("")), None);
    assert_eq!(started(Some("   ")), None);
}

/// `cat` asks for no pager, as git takes it.
#[test]
fn pager_refuses_cat() {
    assert_eq!(started(Some("cat")), None);
    assert_eq!(started(Some("cat -n")), None);
}

/// A pager whose program is not installed is no pager: the answer goes straight to stdout.
#[test]
fn spawned_succeeds_with_none_when_the_program_is_missing() {
    let missing = std::process::Command::new("no-such-pager-anywhere");
    assert!(matches!(spawned(missing), Ok(None)));
}
