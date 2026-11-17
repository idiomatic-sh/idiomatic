//! What the `explanations` service writes, on values alone.

use super::service::{page, path_sections, placed_by, rule_line, workspace_sections};
use super::types::{Section, SectionLine, Tally, Totals};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::RuleId;
use crate::domain::scopes::types::Identity;

/// What a row writes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// What places a path a template entry places.
    PlacedByAnEntry,
    /// What places a path no entry places.
    PlacedByNothing,
    /// A rule that reads a path, by its id.
    ReadByARule,
    /// The sections of a path, as a page.
    PathPage,
    /// A workspace that governs and places one file, with one rule.
    OneOfEach,
}

type Input = Scenario;
/// Every line written, in order.
type Value = Vec<String>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_an_entry_places_names_the_entry"),
        word: Word::Succeeds,
        input: || Scenario::PlacedByAnEntry,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("a_path_nothing_places_says_so"),
        word: Word::Succeeds,
        input: || Scenario::PlacedByNothing,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("a_rule_is_shown_by_its_id"),
        word: Word::Succeeds,
        input: || Scenario::ReadByARule,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("one_file_and_one_rule_are_said_in_the_singular"),
        word: Word::Succeeds,
        input: || Scenario::OneOfEach,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("a_page_writes_each_heading_then_its_lines"),
        word: Word::Succeeds,
        input: || Scenario::PathPage,
        expect: Expect::Value(Vec::new()),
    },
];

/// The lines a row writes, and the ones it must.
fn written(scenario: Scenario) -> (Vec<String>, Vec<&'static str>) {
    let texts = |lines: Vec<SectionLine>| lines.into_iter().map(|line| line.0.into()).collect();
    match scenario {
        Scenario::PlacedByAnEntry => (
            texts(placed_by(Some(&Identity("p/part/errors".into())))),
            vec!["- placed by `p/part/errors`"],
        ),
        Scenario::PlacedByNothing => (
            texts(placed_by(None)),
            vec![
                "- placed by no template entry: in a closed directory this is a `#closed` finding",
            ],
        ),
        Scenario::ReadByARule => (
            texts(vec![rule_line(&RuleId("p/marker".into()))]),
            vec!["- `p/marker`"],
        ),
        Scenario::OneOfEach => {
            let sections = workspace_sections(
                &crate::domain::findings::types::WorkspaceName("w".into()),
                Totals {
                    governed: Tally(1),
                    placed: Tally(1),
                    rules: Tally(1),
                },
                &[],
                &[],
            );
            (
                sections
                    .into_iter()
                    .flat_map(|section| section.lines)
                    .take(1)
                    .map(|line| line.0.into())
                    .collect(),
                vec!["1 file governed, 1 placed, 1 rule in the lint call"],
            )
        }
        Scenario::PathPage => {
            let sections: Vec<Section> = path_sections(
                vec![SectionLine("- `w` at `.`".into())],
                placed_by(None),
                Vec::new(),
            );
            (
                vec![page(&sections).0.into()],
                vec![
                    "## Scopes\n\n- `w` at `.`\n\n## Placement\n\n- placed by no template entry: in a closed directory this is a `#closed` finding\n\n## Rules\n\n\n",
                ],
            )
        }
    }
}

/// The lines that differ from the ones the row must write: none when it writes them all.
fn subject(scenario: Input) -> Value {
    let (got, wanted) = written(scenario);
    if got.iter().map(String::as_str).eq(wanted.iter().copied()) {
        Vec::new()
    } else {
        got
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
