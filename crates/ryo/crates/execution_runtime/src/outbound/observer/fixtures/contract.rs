//! What every `observer` adapter must pass.

use crate::outbound::observer::interface::ObserverPort;
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
    /// The rows of the first rule.
    Declared,
    /// The rows of a rule after those reported before.
    Later,
    /// A rule whose steps were fed nothing.
    Empty,
    /// The totals asked for after the reports.
    Totals,
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
        name: CaseName("a_rule_after_those_reported_before_is_taken"),
        word: Word::Succeeds,
        input: || Report::Later,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_rule_fed_nothing_is_taken"),
        word: Word::Succeeds,
        input: || Report::Empty,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the_totals_answer_after_the_reports"),
        word: Word::Succeeds,
        input: || Report::Totals,
        expect: Expect::Value(()),
    },
];

/// Makes one report to the adapter.
fn report(adapter: &impl ObserverPort, report: Report) {
    let (rule, fed) = match report {
        // The totals hold no rule past those it was told of.
        Report::Totals => {
            let _totals = adapter.totals();
            return;
        }
        Report::Declared => (RuleIndex(0), vec![RowsFed([1, 0, 0])]),
        Report::Later => (RuleIndex(7), vec![RowsFed([1, 1, 1])]),
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
pub fn check(adapter: &impl ObserverPort) {
    run(CASES, Word::Succeeds, |each| {
        report(adapter, each);
        Ok(())
    });
}
