//! The `applying` service's cases, over the `check` fixture's workspace: its rule matched here, its files compressed.

use super::service::matched_rules;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::fixtures::{applications, matching, placement, workspace};
use crate::domain::applications::service::known;
use crate::domain::applications::types::Plan;

/// The fixture's tree.
const FILES: &[&str] = &[
    "tastemaker.toml",
    "src/a/errors.txt",
    "src/a/stray.txt",
    "src/b/lib.txt",
];

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The rules of the applications matched here, and what each reads.
    Matched,
    /// A `tastemaker.toml` rule entry whose include matches no file of the tree: a `#coverage` error, never silence.
    Unreached,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_files_an_application_reaches_are_its_rules_globs"),
        word: Word::Succeeds,
        input: || Question::Matched,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_entry_that_reaches_no_file_is_a_coverage_error"),
        word: Word::Succeeds,
        input: || Question::Unreached,
        expect: Expect::Value(String::new()),
    },
];

fn subject(question: Input) -> Value {
    let workspace = workspace(match question {
        Question::Matched => FILES,
        Question::Unreached => &[],
    });
    let placement = placement(&workspace);
    let applied = applications(&workspace, &placement);
    let mut plan = Plan::default();
    let (said, expected) = match question {
        Question::Matched => {
            let said = match matched_rules(
                &workspace,
                &placement,
                &applied,
                &known(&workspace, &placement),
                matching(),
                &mut plan,
            ) {
                Ok(rules) => rules
                    .iter()
                    .map(|rule| {
                        rule.include
                            .iter()
                            .map(|glob| &*glob.0)
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .collect::<Vec<_>>()
                    .join("; "),
                Err(failure) => format!("{failure:?}"),
            };
            (said, "**".to_owned())
        }
        Question::Unreached => {
            let said = match matched_rules(
                &workspace,
                &placement,
                &applied,
                &known(&workspace, &placement),
                matching(),
                &mut plan,
            ) {
                Ok(rules) if rules.is_empty() => plan
                    .findings
                    .iter()
                    .map(|finding| {
                        format!(
                            "{} {}",
                            crate::domain::findings::service::printed(&finding.rule).0,
                            finding.message.0
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; "),
                Ok(rules) => format!(
                    "{} rules handed for an entry that reaches no file",
                    rules.len()
                ),
                Err(failure) => format!("{failure:?}"),
            };
            (
                said,
                "p/marker#coverage `p/marker` reaches no file: its include matches nothing here"
                    .to_owned(),
            )
        }
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
