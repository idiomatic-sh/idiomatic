//! What `up` promises its caller, one row of `CASES` for each case it has, over a product on disk checked first.

use super::command::{UpArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::ratchet::check::fixtures::{Members, chosen, edge, product, workspace};

/// What `check` ran over first.
#[derive(Clone, Copy, Debug)]
enum Before {
    /// Nothing: no check has run.
    Nothing,
    /// A product with one new violation.
    Lint,
    /// A workspace root: `crates/a` with one new violation and `crates/b` clean, and the root's own tools as `Members` says.
    Workspace(Members),
}

/// The command line, after `up`, and what `check` ran over first.
type Input = (Vec<&'static str>, Before);
/// What the answer says.
type Value = std::borrow::Cow<'static, str>;
/// How the command ends when it cannot answer.
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: UpArguments,
    /// The format the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    output: crate::output::FormatSwitch,
    /// The dry run the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    dry_run: crate::output::dry_run::DryRunSwitch,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_new_violation_at_a_place_is_recorded"),
        word: Word::Succeeds,
        input: || {
            (
                vec!["--reason", "why", "--at", "src/a.rs:1:5"],
                Before::Lint,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "recorded src/a.rs: lint/r (1)\n1 recorded, 0 new left\n",
        )),
    },
    Case {
        name: CaseName("a_dry_run_says_what_it_would_record"),
        word: Word::Succeeds,
        input: || (vec!["--reason", "why", "-n"], Before::Lint),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "would record src/a.rs: lint/r (1)\n1 would record, 0 new left\n",
        )),
    },
    Case {
        name: CaseName("a_violation_left_is_a_place_in_the_json"),
        word: Word::Succeeds,
        input: || {
            (
                vec!["--reason", "why", "--rule", "other", "-n"],
                Before::Lint,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            r#"left src/a.rs:1:5: lint/r
0 would record, 1 new left
 | [{"column":5,"fingerprint":"1c4c0e71c81e8cdd","level":"warning","line":1,"message":"a marker","path":"src/a.rs","rule":"r","status":"new","tool":"lint"}]"#,
        )),
    },
    Case {
        name: CaseName("up_at_a_workspace_root_with_no_tools_of_its_own_records_in_each_member"),
        word: Word::Succeeds,
        input: || (vec!["--reason", "why"], Before::Workspace(Members::Two)),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "crates/a: recorded src/a.rs: lint/r (1)\ncrates/a: 1 recorded, 0 new left\ncrates/b: 0 recorded, 0 new left\n",
        )),
    },
    Case {
        name: CaseName("up_at_a_workspace_root_records_in_the_root_then_in_each_member"),
        word: Word::Succeeds,
        input: || {
            (
                vec!["--reason", "why", "-n"],
                Before::Workspace(Members::TwoAndRoot),
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            ".: 0 would record, 0 new left\ncrates/a: would record src/a.rs: lint/r (1)\ncrates/a: 1 would record, 0 new left\ncrates/b: 0 would record, 0 new left\n",
        )),
    },
    Case {
        name: CaseName("up_with_no_check_before_it_fails"),
        word: Word::Fails,
        input: || (vec!["--reason", "why"], Before::Nothing),
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("an_at_that_is_no_place_is_refused"),
        word: Word::Refuses,
        input: || (vec!["--reason", "why", "--at", "nowhere"], Before::Lint),
        expect: Expect::error(Outcome::Failure),
    },
];

/// The product on disk the row runs over, checked first unless `before` is `Nothing`.
fn scene(before: Before) -> (tempfile::TempDir, crate::edge::Edge) {
    let directory = match before {
        Before::Nothing | Before::Lint => product(),
        Before::Workspace(members) => workspace(members),
    }
    .expect("the product is written");
    let edge = edge(&directory);
    if !matches!(before, Before::Nothing) {
        let check = <crate::resources::ratchet::check::command::CheckArguments as clap::FromArgMatches>::from_arg_matches(
            &<crate::resources::ratchet::check::command::CheckArguments as clap::Args>::augment_args(clap::Command::new("check"))
                .get_matches_from(["check"]),
        )
        .expect("`check` parses");
        let quiet = crate::edge::Edge {
            format: crate::output::Format::Quiet,
            ..edge.clone()
        };
        assert_ne!(
            crate::resources::ratchet::check::command::run(check, &quiet, |_| {}),
            Outcome::Error,
            "the scene's check runs over the product"
        );
    }
    (directory, edge)
}

/// Parses the row's command line, which every row writes to parse.
fn parsed(line: Vec<&'static str>) -> Line {
    <Line as clap::Parser>::try_parse_from(std::iter::once("up").chain(line))
        .expect("the row's command line parses")
}

/// Runs `up` with the row's command line over its scene.
fn subject((line, before): Input) -> Result<Value, Failure> {
    let (_directory, edge) = scene(before);
    let line = parsed(line);
    let edge = chosen(edge, line.output, line.dry_run);
    answer(line.arguments, &edge).map(|(said, _)| {
        // A violation left is said in the JSON as a place, its parts apart.
        let mut written = Vec::new();
        let left =
            crate::output::Encoded::json(&said, &mut written, crate::output::Spacing::Compact)
                .ok()
                .and_then(|()| serde_json::from_slice::<serde_json::Value>(&written).ok())
                .and_then(|json| json.get("left").cloned())
                .filter(|left| left.as_array().is_some_and(|left| !left.is_empty()));
        std::borrow::Cow::Owned(match left {
            Some(left) => format!("{said} | {left}"),
            None => said.to_string(),
        })
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
