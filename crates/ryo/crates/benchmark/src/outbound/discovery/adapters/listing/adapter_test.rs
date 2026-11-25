//! A real command lists a real worktree; refusals keep their categories.
use super::adapter::DiscoveryListingAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::discovery::{fixtures::contract::check, interface::DiscoveryPort as _};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;

/// Which command the adapter is bound to.
#[derive(Clone, Copy, Debug)]
enum Command {
    ListsTwoFiles,
    ListsTwoFilesRelativeToTheRoot,
    ListsNothing,
    ListsAFileOutsideTheRoot,
    FailsSayingWhyOnStandardOutput,
    Missing,
    Contract,
}
type Input = Command;
/// The listed paths, as the adapter yields them.
type Value = &'static [&'static str];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Invalid,
    /// A refusal whose message is compared in full.
    Said(&'static str),
    Missing,
    Unexpected,
}
type Failure = Refusal;

/// What the refusal of the failing command says when it is the adapter's text around the platform's own spelling of exit status 1.
const FAILED_LISTING: &str = "file discovery failed: <the platform's spelling of exit status 1>; stderr: ; stdout: configuration not loaded";

/// How the platform spells a process that exited with status 1.
fn exit_one() -> Result<std::process::ExitStatus, Refusal> {
    std::process::Command::new("sh")
        .args(["-c", "exit 1"])
        .status()
        .map_err(|_error| Refusal::Unexpected)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the listed files are relative to the root and sorted"),
        word: Word::Succeeds,
        input: || Command::ListsTwoFiles,
        expect: Expect::Value(&["a.txt", "b.txt"]),
    },
    Case {
        name: CaseName("files listed relative to the root yield the paths absolute ones do"),
        word: Word::Succeeds,
        input: || Command::ListsTwoFilesRelativeToTheRoot,
        expect: Expect::Value(&["a.txt", "b.txt"]),
    },
    Case {
        name: CaseName("the listing adapter satisfies the discovery contract"),
        word: Word::Succeeds,
        input: || Command::Contract,
        expect: Expect::Value(&["a.txt", "b.txt"]),
    },
    Case {
        name: CaseName("an empty listing is refused"),
        word: Word::Refuses,
        input: || Command::ListsNothing,
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a file outside the root is refused"),
        word: Word::Refuses,
        input: || Command::ListsAFileOutsideTheRoot,
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a failed listing carries what the command printed"),
        word: Word::Refuses,
        input: || Command::FailsSayingWhyOnStandardOutput,
        expect: Expect::Error(Refusal::Said(FAILED_LISTING)),
    },
    Case {
        name: CaseName("a command that cannot start is a failure, not an empty selection"),
        word: Word::Fails,
        input: || Command::Missing,
        expect: Expect::Error(Refusal::Missing),
    },
];

/// The adapter bound to `sh` running one script over the paths of the root, or to an executable that does not exist.
///
/// The root reaches the script as its arguments, `$1` the path of `b.txt` and `$2` the path of `a.txt`, spelled as the
/// platform joins them: `$PWD` is a POSIX spelling that Windows paths are not.
fn adapter(command: Command, root: &Path) -> DiscoveryListingAdapter {
    let script = match command {
        Command::ListsTwoFiles | Command::Contract => r#"echo "$1"; echo "$2""#,
        Command::ListsTwoFilesRelativeToTheRoot => "echo b.txt; echo ./a.txt",
        Command::ListsAFileOutsideTheRoot => "echo /elsewhere/a.txt",
        Command::FailsSayingWhyOnStandardOutput => "echo 'configuration not loaded'; exit 1",
        Command::ListsNothing | Command::Missing => "true",
    };
    let executable = if matches!(command, Command::Missing) {
        "/nonexistent/lister"
    } else {
        "sh"
    };
    DiscoveryListingAdapter::new(
        executable.into(),
        vec![
            "-c".into(),
            script.into(),
            "sh".into(),
            root.join("b.txt").into(),
            root.join("a.txt").into(),
        ],
    )
}

fn subject(command: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().map_err(|_error| Refusal::Unexpected)?;
    let canonical = std::path::absolute(root.path()).map_err(|_error| Refusal::Unexpected)?;
    if matches!(command, Command::Contract) {
        check(
            &adapter(Command::ListsTwoFiles, &canonical),
            &adapter(Command::ListsNothing, &canonical),
            &canonical,
        );
    }
    adapter(command, &canonical)
        .files(&canonical)
        .map(|files| {
            &*Box::leak(
                files
                    .into_iter()
                    .map(|file| &*Box::leak(file.0))
                    .collect::<Box<[_]>>(),
            )
        })
        .map_err(|error| match error {
            Error::Invalid(message)
                if matches!(command, Command::FailsSayingWhyOnStandardOutput) =>
            {
                let status = match exit_one() {
                    Ok(status) => status,
                    Err(refusal) => return refusal,
                };
                let expected = format!(
                    "file discovery failed: {status}; stderr: ; stdout: configuration not loaded"
                );
                if message == expected {
                    Refusal::Said(FAILED_LISTING)
                } else {
                    Refusal::Said(Box::leak(message.into_boxed_str()))
                }
            }
            Error::Invalid(_) => Refusal::Invalid,
            Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => Refusal::Missing,
            Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Unexpected,
        })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
