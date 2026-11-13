//! A real command lists a real worktree; refusals keep their categories.
use super::adapter::DiscoveryListingAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::discovery::{fixtures::contract::check, interface::DiscoveryPort as _};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// Which command the adapter is bound to.
#[derive(Clone, Copy, Debug)]
enum Command {
    ListsTwoFiles,
    ListsTwoFilesRelativeToTheRoot,
    ListsNothing,
    ListsAFileOutsideTheRoot,
    Missing,
    Contract,
}
type Input = Command;
/// The listed paths, as the adapter yields them.
type Value = &'static [&'static str];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Invalid,
    Missing,
    Unexpected,
}
type Failure = Refusal;

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
        name: CaseName("a command that cannot start is a failure, not an empty selection"),
        word: Word::Fails,
        input: || Command::Missing,
        expect: Expect::Error(Refusal::Missing),
    },
];

/// The adapter bound to `sh` running one script, or to an executable that does not exist.
fn adapter(command: Command) -> DiscoveryListingAdapter {
    let script = match command {
        Command::ListsTwoFiles | Command::Contract => r#"echo "$PWD/b.txt"; echo "$PWD/a.txt""#,
        Command::ListsTwoFilesRelativeToTheRoot => "echo b.txt; echo ./a.txt",
        Command::ListsAFileOutsideTheRoot => "echo /elsewhere/a.txt",
        Command::ListsNothing | Command::Missing => "true",
    };
    let executable = if matches!(command, Command::Missing) {
        "/nonexistent/lister"
    } else {
        "sh"
    };
    DiscoveryListingAdapter::new(executable.into(), vec!["-c".into(), script.into()])
}

fn subject(command: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().map_err(|_error| Refusal::Unexpected)?;
    let canonical = root
        .path()
        .canonicalize()
        .map_err(|_error| Refusal::Unexpected)?;
    if matches!(command, Command::Contract) {
        check(
            &adapter(Command::ListsTwoFiles),
            &adapter(Command::ListsNothing),
            &canonical,
        );
    }
    adapter(command)
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
