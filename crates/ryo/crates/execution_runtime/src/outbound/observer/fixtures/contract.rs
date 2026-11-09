//! What every `observer` adapter must pass.

use crate::outbound::observer::interface::ExecutionObserver;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: which report to make to the adapter.
type Input = Report;
/// What a row of the contract expects when it succeeds: the adapter took the report.
type Value = ();
/// What a row of the contract expects when it is refused.
type Failure = ();

/// A report the contract makes to an adapter.
#[derive(Clone, Copy, Debug)]
pub enum Report {
    /// The rows of a rule the adapter was sized for.
    Declared,
    /// The rows of a rule beyond those the adapter was sized for.
    Undeclared,
    /// A rule whose steps were fed nothing.
    Empty,
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_declared_rule_is_taken"),
        word: Word::Succeeds,
        input: || Report::Declared,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_undeclared_rule_is_taken_without_a_panic"),
        word: Word::Succeeds,
        input: || Report::Undeclared,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_rule_fed_nothing_is_taken"),
        word: Word::Succeeds,
        input: || Report::Empty,
        expect: Expect::Value(()),
    },
];

/// Makes one report to the adapter.
fn report(adapter: &impl ExecutionObserver, report: Report) {
    let (rule, fed) = match report {
        Report::Declared => (RuleIndex(0), vec![RowsFed([1, 0, 0])]),
        Report::Undeclared => (RuleIndex(usize::MAX), vec![RowsFed([1, 1, 1])]),
        Report::Empty => (RuleIndex(0), Vec::new()),
    };
    adapter.fed(
        rule,
        LogicalCounts {
            fed: fed.into_boxed_slice(),
        },
    );
}

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl ExecutionObserver) {
    run(CASES, Word::Succeeds, |each| {
        report(adapter, each);
        Ok(())
    });
}
