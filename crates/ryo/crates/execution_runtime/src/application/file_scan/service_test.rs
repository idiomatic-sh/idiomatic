//! The harness's cases: equal rows of the accepted rules compare, a planted difference is the first place they differ, a rule the
//! new engine refused is not compared, and with no rule selected nothing is accepted.

use super::service::{compared, loaded};
use super::types::{HarnessDifference, HarnessRows, RowText, RuleFile};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{Count, FileId};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// What a row compares.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Both engines found the same row of rule 0 in file 1.
    Equal,
    /// The new engine found a row the old one did not: the planted difference.
    Planted,
    /// They differ only for rule 1, which the new engine refused.
    Refused,
    /// No rule is selected: the load accepts none.
    NothingSelected,
}

/// Rule 0's rows in file 1, one row of `text`, and rule 1's row `other`.
fn rows(text: &str, other: &str) -> HarnessRows {
    let place = |rule| RuleFile {
        rule: SelectedRuleOrdinal(rule),
        file: FileId(1),
    };
    HarnessRows(
        [
            (place(0), vec![RowText(text.into())]),
            (place(1), vec![RowText(other.into())]),
        ]
        .into_iter()
        .collect(),
    )
}

type Input = Scenario;
/// How many rules were compared.
type Value = Count;
type Failure = HarnessDifference;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_rows_of_the_accepted_rules_compare"),
        word: Word::Succeeds,
        input: || Scenario::Equal,
        expect: Expect::Value(Count(1)),
    },
    Case {
        name: CaseName("a_rule_the_new_engine_refused_is_not_compared"),
        word: Word::Succeeds,
        input: || Scenario::Refused,
        expect: Expect::Value(Count(1)),
    },
    Case {
        name: CaseName("with_no_rule_selected_nothing_is_accepted_or_compared"),
        word: Word::Succeeds,
        input: || Scenario::NothingSelected,
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_planted_difference_is_the_first_place_the_rows_differ"),
        word: Word::Refuses,
        input: || Scenario::Planted,
        expect: Expect::Error(HarnessDifference {
            at: RuleFile {
                rule: SelectedRuleOrdinal(0),
                file: FileId(1),
            },
            old: Vec::new(),
            new: Vec::new(),
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let accepted = [SelectedRuleOrdinal(0)];
    match scenario {
        Scenario::Equal => compared(&rows("a", "b"), &rows("a", "b"), &accepted),
        Scenario::Refused => compared(&rows("a", "b"), &rows("a", "c"), &accepted),
        Scenario::NothingSelected => {
            let nothing = loaded(&[]);
            compared(
                &HarnessRows::default(),
                &HarnessRows::default(),
                &nothing.accepted,
            )
        }
        Scenario::Planted => {
            compared(&rows("a", "b"), &rows("z", "b"), &accepted).map_err(|difference| {
                HarnessDifference {
                    old: Vec::new(),
                    new: Vec::new(),
                    ..difference
                }
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
