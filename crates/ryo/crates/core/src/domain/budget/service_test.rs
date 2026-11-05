//! The `budget` service's cases, on values alone.

use super::errors::BudgetError;
use super::service::{fit, repository_size, within};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run, selects};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, FileSize, Nanoseconds};
use ryo_values::domain::runs::types::MemoryBytes;
use ryo_values::domain::runs::types::RuleId;

/// What a row of the cases asks of the budget.
#[derive(Clone, Copy)]
enum Scenario {
    /// The bytes of three files together.
    SizeOfThreeFiles,
    /// A repository of 10 bytes against a budget of 10.
    FitsExactly,
    /// A repository of 11 bytes against a budget of 10.
    FitsNot,
    /// A process that holds 10 bytes against a budget of 10.
    WithinExactly,
    /// A process that holds 11 bytes against a budget of 10, working for two rules.
    WithinNot,
}

/// What a row of the cases asks of the budget.
type Input = Scenario;
/// A number of bytes, or one for a check that passed.
type Value = u64;
type Failure = BudgetError;

fn file(path: &str, size: u64) -> CorpusEntry {
    CorpusEntry {
        executable: Default::default(),
        path: FilePath(path.into()),
        size: FileSize(size),
        modified: Nanoseconds(0),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::SizeOfThreeFiles => Ok(repository_size(
            &[file("a", 1), file("b", 20), file("c", 300)].map(CorpusInput::Available),
        )
        .0),
        Scenario::FitsExactly => fit(MemoryBytes(10), MemoryBytes(10)).map(|()| 1),
        Scenario::FitsNot => fit(MemoryBytes(11), MemoryBytes(10)).map(|()| 1),
        Scenario::WithinExactly => within(MemoryBytes(10), MemoryBytes(10), Vec::new).map(|()| 1),
        Scenario::WithinNot => within(MemoryBytes(11), MemoryBytes(10), || {
            vec![RuleId("a".into()), RuleId("b".into())]
        })
        .map(|()| 1),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_size_of_a_repository_is_the_sizes_of_its_files_together"),
        word: Word::Succeeds,
        input: || Scenario::SizeOfThreeFiles,
        expect: Expect::Value(321),
    },
    Case {
        name: CaseName("a_repository_as_large_as_the_budget_fits"),
        word: Word::Succeeds,
        input: || Scenario::FitsExactly,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_run_that_holds_as_much_as_its_budget_is_within_it"),
        word: Word::Succeeds,
        input: || Scenario::WithinExactly,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_repository_larger_than_the_budget_is_refused"),
        word: Word::Refuses,
        input: || Scenario::FitsNot,
        expect: Expect::Refusal(|error| selects(matches!(error, BudgetError::TooLarge { .. }))),
    },
    Case {
        name: CaseName("a_run_that_holds_more_than_its_budget_is_refused_naming_the_rules"),
        word: Word::Refuses,
        input: || Scenario::WithinNot,
        expect: Expect::Refusal(|error| {
            if matches!(
                error,
                BudgetError::Exceeded { used, budget, plans }
                    if used.0 == 11
                        && budget.0 == 10
                        && plans.len() == 2
                        && &*plans[0].0 == "a"
                        && &*plans[1].0 == "b"
            ) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
