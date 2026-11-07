//! What `down` promises its caller, one row of `CASES` for each case it has, over a product on disk checked, recorded,
//! fixed and checked again.

use super::command::{DownArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::ratchet::check::fixtures::{Members, edge, ignoring, workspace};

/// What happens to the product before `down`.
#[derive(Clone, Copy, Debug)]
enum Before {
    /// Nothing: no check has run.
    Unchecked,
    /// Its violation is checked, recorded, fixed and checked again.
    Fixed,
    /// Its violation, suppressed by an in-source ignore, is checked.
    Ignored,
    /// A workspace root with no tools of its own: `crates/a`'s violation is checked, recorded, fixed and checked again, and
    /// `crates/b` is clean.
    FixedInAMember,
}

/// The command line, after `down`, and what happens before it.
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
    arguments: DownArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_fixed_violation_is_drained"),
        word: Word::Succeeds,
        input: || (Vec::new(), Before::Fixed),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "drained src/a.rs: lint/r (fixed, 0 left; ignored in the source)\n1 drained\n",
        )),
    },
    Case {
        name: CaseName("a_new_ignored_violation_is_recorded"),
        word: Word::Succeeds,
        input: || (Vec::new(), Before::Ignored),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "recorded src/a.rs: lint/r (1, ignored in the source)\n0 drained; 1 recorded\n",
        )),
    },
    Case {
        name: CaseName("down_at_a_workspace_root_with_no_tools_of_its_own_drains_each_member"),
        word: Word::Succeeds,
        input: || (Vec::new(), Before::FixedInAMember),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "crates/a: drained src/a.rs: lint/r (fixed, 0 left; accepted for: why)\ncrates/a: 1 drained\ncrates/b: 0 drained\n",
        )),
    },
    Case {
        name: CaseName("down_with_no_check_before_it_fails"),
        word: Word::Fails,
        input: || (Vec::new(), Before::Unchecked),
        expect: Expect::error(Outcome::Error),
    },
];

/// Runs `check` over the product, quietly: the scene's setup, which must run, whatever it finds.
fn check(edge: &crate::edge::Edge) {
    use crate::resources::ratchet::check::command::{CheckArguments, run};
    let arguments = <CheckArguments as clap::FromArgMatches>::from_arg_matches(
        &<CheckArguments as clap::Args>::augment_args(clap::Command::new("check"))
            .get_matches_from(["check"]),
    )
    .expect("`check` parses");
    let quiet = crate::edge::Edge {
        format: crate::output::Format::Quiet,
        ..edge.clone()
    };
    assert_ne!(
        run(arguments, &quiet, |_| {}),
        Outcome::Error,
        "the scene's check runs over the product"
    );
}

/// The product on disk the row runs over: checked, its ignored violation recorded, fixed and checked again when `Fixed`,
/// only checked when `Ignored`, and untouched otherwise.
fn scene(before: Before) -> (tempfile::TempDir, crate::edge::Edge) {
    let directory = match before {
        Before::FixedInAMember => workspace(Members::Two),
        Before::Unchecked | Before::Fixed | Before::Ignored => ignoring(),
    }
    .expect("the product is written");
    let edge = edge(&directory);
    let fixed = match before {
        Before::FixedInAMember => "crates/a/lint.sarif",
        Before::Unchecked | Before::Fixed | Before::Ignored => "lint.sarif",
    };
    if matches!(before, Before::Ignored) {
        check(&edge);
    }
    if matches!(before, Before::Fixed | Before::FixedInAMember) {
        check(&edge);
        let up = <crate::resources::ratchet::up::command::UpArguments as clap::FromArgMatches>::from_arg_matches(
            &<crate::resources::ratchet::up::command::UpArguments as clap::Args>::augment_args(clap::Command::new("up"))
                .get_matches_from(["up", "--reason", "why"]),
        )
        .expect("`up --reason why` parses");
        let quiet = crate::edge::Edge {
            format: crate::output::Format::Quiet,
            ..edge.clone()
        };
        assert_eq!(
            crate::resources::ratchet::up::command::run(up, &quiet),
            Outcome::Success,
            "up records the violation"
        );
        std::fs::write(directory.path().join(fixed), r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "lint", "rules": [{"id": "r"}]}}, "results": []}]}"#)
            .expect("the fixed log is written");
        check(&edge);
    }
    (directory, edge)
}

/// Parses the row's command line, which every row writes to parse.
fn parsed(line: Vec<&'static str>) -> Line {
    <Line as clap::Parser>::try_parse_from(std::iter::once("down").chain(line))
        .expect("the row's command line parses")
}

/// Runs `down` with the row's command line over its scene.
fn subject((line, before): Input) -> Result<Value, Failure> {
    let (_directory, edge) = scene(before);
    answer(parsed(line).arguments, &edge).map(|(said, _)| std::borrow::Cow::Owned(said.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
