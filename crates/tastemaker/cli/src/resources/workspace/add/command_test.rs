//! What `add` promises its caller, one row of `CASES` for each case it has, over a workspace on disk.

use super::command::{AddArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::workspace::check::fixtures::{dry_run, edge, workspace};

/// The command line, after `add`.
type Input = Vec<&'static str>;
/// What the answer says, as JSON text.
type Value = bool;
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: AddArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_spec_entry_would_be_added"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", "src/c", "-n"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_printed_command_scaffolds_the_requested_hidden_path"),
        word: Word::Succeeds,
        input: || vec!["part", "--at", ".config/owner's space"],
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_param_without_a_value_is_a_usage_error_that_ends_2"),
        word: Word::Refuses,
        input: || vec!["part", "--at", "src/c", "--param", "novalue", "-n"],
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_definition_no_package_holds_is_refused"),
        word: Word::Refuses,
        input: || vec!["absent", "--at", "src/c", "-n"],
        expect: Expect::error(Outcome::Error),
    },
];

fn subject(line: Input) -> Result<Value, Failure> {
    let directory = workspace();
    // `-n` is the command line's, taken before the command reaches its arguments.
    let dry = line.contains(&"-n");
    let line = line.into_iter().filter(|argument| *argument != "-n");
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("add").chain(line))
        .map_err(|_error| Outcome::Error)?;
    let edge = if dry {
        dry_run(edge(&directory))
    } else {
        edge(&directory)
    };
    if !dry {
        let path = directory.path().join("tastemaker.toml");
        let text = std::fs::read_to_string(&path).expect("workspace");
        let text = text
            .lines()
            .map(|row| {
                if row.starts_with("specs =") {
                    "specs = []"
                } else {
                    row
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(path, text).expect("unplaced workspace");
    }
    let answer = answer(parsed.arguments, &edge)?;
    if !dry {
        let value = serde_json::from_str::<serde_json::Value>(
            &crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?,
        )
        .map_err(|_error| Outcome::Error)?;
        let next = value["next"].as_str().expect("printed next command");
        let arguments = shlex::split(next)
            .expect("shell arguments")
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect();
        assert_eq!(
            crate::resources::run_from(arguments, &directory.path().display().to_string(), "test"),
            Outcome::Success
        );
        assert!(
            directory
                .path()
                .join(".config/owner's space/errors.toml")
                .is_file()
        );
        assert!(!directory.path().join("errors.toml").exists());
        assert!(!directory.path().join("config").exists());
    }
    let said = crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?;
    Ok(said.contains("p/part@0.1.0"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
