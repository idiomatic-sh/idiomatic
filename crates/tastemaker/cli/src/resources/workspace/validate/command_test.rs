//! What `validate` promises its caller, one row of `CASES` for each case it has, over a workspace on disk.

use super::command::{ValidateArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::workspace::check::fixtures::{edge, workspace};

/// Where `validate` runs: in the fixture's workspace, or in a directory that holds none.
#[derive(Clone, Copy, Debug)]
enum Place {
    /// The fixture's workspace, holding one package.
    Workspace,
    /// A filter that names no definition.
    Unmatched,
    /// An empty directory.
    Empty,
    /// The fixture's workspace, whose rule is spoiled after one run proved it.
    Spoiled,
}

/// Where the command runs.
type Input = Place;
/// Whether the answer says what the row promises.
type Value = bool;
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: ValidateArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_explicit_filter_must_match_a_definition"),
        word: Word::Refuses,
        input: || Place::Unmatched,
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("every_version_of_the_package_is_proved"),
        word: Word::Succeeds,
        input: || Place::Workspace,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_directory_that_holds_no_workspace_is_refused"),
        word: Word::Refuses,
        input: || Place::Empty,
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_failing_validate_leaves_the_store_as_it_was_byte_for_byte"),
        word: Word::Succeeds,
        input: || Place::Spoiled,
        expect: Expect::Value(true),
    },
];

/// A validation in which a version does not hold keeps nothing: the store file the last run that succeeded wrote is
/// the same bytes after it. Whether the first run proves both versions, the spoiled one does not, and the store is the
/// same bytes.
fn leaves_the_store(directory: &tempfile::TempDir) -> Result<Value, Failure> {
    let validate = || -> Result<String, Failure> {
        let parsed = <Line as clap::Parser>::try_parse_from(["validate"])
            .map_err(|_error| Outcome::Error)?;
        let answer = answer(parsed.arguments, &edge(directory))?;
        crate::output::json_text(&answer).map_err(|_error| Outcome::Error)
    };
    let first = validate()?.contains("\"proven\":2");
    let store = directory.path().join(".tastemaker_cache/validate.store");
    let kept = std::fs::read(&store).map_err(|_error| Outcome::Error)?;
    let rule = directory.path().join("p/rules/marker/0.1.0/rule.toml");
    let text = std::fs::read_to_string(&rule).map_err(|_error| Outcome::Error)?;
    let spoiled = text.replace("line = 1 }", "line = 2 }");
    std::fs::write(&rule, &spoiled).map_err(|_error| Outcome::Error)?;
    let held = validate()?.contains("\"proven\":2");
    let after = std::fs::read(&store).map_err(|_error| Outcome::Error)?;
    Ok(first && text != spoiled && !held && after == kept)
}

fn subject(place: Input) -> Result<Value, Failure> {
    let directory = match place {
        Place::Workspace | Place::Unmatched | Place::Spoiled => workspace(),
        Place::Empty => tempfile::tempdir().map_err(|_error| Outcome::Error)?,
    };
    if matches!(place, Place::Spoiled) {
        return leaves_the_store(&directory);
    }
    let arguments = if matches!(place, Place::Unmatched) {
        vec!["validate", "no-such-definition"]
    } else {
        vec!["validate"]
    };
    let parsed =
        <Line as clap::Parser>::try_parse_from(arguments).map_err(|_error| Outcome::Error)?;
    let edge = edge(&directory);
    let answer = answer(parsed.arguments, &edge)?;
    let said = crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?;
    Ok(said.contains("\"proven\":2"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
