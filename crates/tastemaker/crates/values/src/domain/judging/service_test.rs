//! What the `judging` service hosts for the count queries and reads back from them; which counts are refused is the
//! queries', proven through ryo in the ryo adapter's tests.

use std::collections::BTreeMap;

use crate::domain::closing::types::Closings;

use super::constants::{BRANCH, COUNT_RULE, PAIR_RULE};
use super::service::{host, judgment_of};
use super::types::{BoundsHeld, EntryKind, EntryNumber, Judgment};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::{AnswerCell, CellText, CellValue, LintAnswer, LintRuleId};
use crate::domain::definitions::types::ColumnName;
use crate::domain::entries::types::{Amount, ConditionName, Count};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::{
    Answer, CountCheck, CountMessage, Identity, PairCheck, ScopeIndex,
};

/// What a row of the cases does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A count and a pair: the count numbered first.
    Numbered,
    /// A count reading a condition no rule answered: its answer written failing.
    Unasked,
    /// A count reading no condition: no answer written, and its own bounds.
    Plain,
    /// A row of the count query: the count its subject numbers, held to the branch it names.
    Refused,
    /// A row of the pair query: the pair its subject numbers.
    Missing,
    /// A row whose subject is no number: no row of the count queries.
    Foreign,
}

type Input = Scenario;
/// What the case gave, written out; empty when it is the row's.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("counts_are_numbered_before_pairs"),
        word: Word::Succeeds,
        input: || Scenario::Numbered,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_condition_no_rule_answered_is_hosted_failing"),
        word: Word::Succeeds,
        input: || Scenario::Unasked,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_count_that_reads_no_condition_hosts_its_own_bounds"),
        word: Word::Succeeds,
        input: || Scenario::Plain,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_refused_row_names_its_count_and_branch"),
        word: Word::Succeeds,
        input: || Scenario::Refused,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_missing_row_names_its_pair"),
        word: Word::Succeeds,
        input: || Scenario::Missing,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_row_whose_subject_is_no_number_is_none"),
        word: Word::Succeeds,
        input: || Scenario::Foreign,
        expect: Expect::Value(String::new()),
    },
];

/// A spec's count over `count`, one found.
fn count(count: Count) -> CountCheck {
    CountCheck {
        rule: crate::domain::findings::service::check_id("p/t", "count"),
        path: RepoPath("tastemaker.toml".into()),
        scope: ScopeIndex(0),
        count,
        found: Amount(1),
        message: CountMessage::Spec {
            identity: Identity("p/t".into()),
        },
    }
}

/// A row `rule` answered of the entry `subject`, with these text cells.
fn answer(rule: &str, subject: &str, cells: &[(&str, &str)]) -> LintAnswer {
    LintAnswer {
        rule: LintRuleId(rule.into()),
        path: RepoPath(subject.into()),
        cells: cells
            .iter()
            .map(|(name, text)| AnswerCell {
                name: ColumnName((*name).into()),
                value: CellValue::Text(CellText((*text).into())),
            })
            .collect(),
    }
}

fn subject(scenario: Scenario) -> Value {
    let when = Count::When {
        condition: ConditionName("c".into()),
        then: Amount(1),
        otherwise: Amount(0),
    };
    match scenario {
        Scenario::Numbered => {
            let rows = host(
                &[count(Count::Exactly(Amount(1)))],
                &[PairCheck {
                    scope: ScopeIndex(0),
                    count: Count::Exactly(Amount(1)),
                    findings: Vec::new(),
                }],
                &BTreeMap::new(),
                &Closings::default(),
            );
            let kinds = rows
                .entries
                .iter()
                .map(|row| (row.entry, row.kind))
                .collect::<Vec<_>>();
            if kinds
                == [
                    (EntryNumber(0), EntryKind::Count),
                    (EntryNumber(1), EntryKind::Pair),
                ]
            {
                String::new()
            } else {
                format!("{kinds:?}")
            }
        }
        Scenario::Unasked => {
            let rows = host(&[count(when)], &[], &BTreeMap::new(), &Closings::default());
            let answers = rows
                .conditions
                .iter()
                .map(|row| (row.scope, row.condition.0.to_string(), row.holds))
                .collect::<Vec<_>>();
            if answers == [(ScopeIndex(0), "c".to_owned(), Answer::Fails)] {
                String::new()
            } else {
                format!("{answers:?}")
            }
        }
        Scenario::Plain => {
            let rows = host(
                &[count(Count::Exactly(Amount(2)))],
                &[],
                &BTreeMap::new(),
                &Closings::default(),
            );
            match (rows.entries.first(), rows.conditions.is_empty()) {
                (Some(row), true)
                    if row.condition.0.is_empty()
                        && row.bounds.min == Amount(2)
                        && row.bounds.max == Amount(2) =>
                {
                    String::new()
                }
                _ => format!("{rows:?}"),
            }
        }
        Scenario::Refused => {
            let read = judgment_of(&answer(COUNT_RULE, "3", &[(BRANCH, "2")]));
            if read
                == Some(Judgment::Refused {
                    entry: EntryNumber(3),
                    held: BoundsHeld::Otherwise,
                })
            {
                String::new()
            } else {
                format!("{read:?}")
            }
        }
        Scenario::Missing => {
            let read = judgment_of(&answer(PAIR_RULE, "1", &[]));
            if read
                == Some(Judgment::Missing {
                    entry: EntryNumber(1),
                })
            {
                String::new()
            } else {
                format!("{read:?}")
            }
        }
        Scenario::Foreign => {
            let read = judgment_of(&answer(COUNT_RULE, "src/a.txt", &[(BRANCH, "0")]));
            read.map_or_else(String::new, |read| format!("{read:?}"))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
