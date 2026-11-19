//! What every `observer` adapter must pass.

use crate::outbound::observer::interface::ObserverPort;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: which report to make to the adapter.
type Input = Report;
/// What a row of the contract expects when it succeeds: whether the totals after the report are those before it plus the report.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = ();

/// A report the contract makes to an adapter.
#[derive(Clone, Copy, Debug)]
pub enum Report {
    /// Rows of the first rule's two steps.
    First,
    /// Rows of the same rule again, added to the first.
    Again,
    /// Rows of a rule after those reported before.
    Later,
    /// A rule whose steps were fed nothing.
    Empty,
    /// A handover of a file after the files told before.
    HandedLater,
    /// A handover of a file before the files told before.
    HandedEarlier,
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_reported_is_in_the_totals"),
        word: Word::Succeeds,
        input: || Report::First,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_reported_again_adds_up_by_step"),
        word: Word::Succeeds,
        input: || Report::Again,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_after_those_reported_before_is_in_the_totals"),
        word: Word::Succeeds,
        input: || Report::Later,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_fed_nothing_leaves_the_totals_as_they_were"),
        word: Word::Succeeds,
        input: || Report::Empty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_handover_told_is_among_the_handovers"),
        word: Word::Succeeds,
        input: || Report::HandedLater,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_handovers_are_in_the_order_of_their_files_whatever_the_order_told"),
        word: Word::Succeeds,
        input: || Report::HandedEarlier,
        expect: Expect::Value(true),
    },
];

/// Makes one report to the adapter and compares its totals after it with those before plus the report.
fn answer(adapter: &impl ObserverPort, report: Report) -> Value {
    let (rule, fed) = match report {
        Report::HandedLater => return handovers(adapter, FileId(9)),
        Report::HandedEarlier => return handovers(adapter, FileId(1)),
        Report::First => (RuleIndex(0), vec![RowsFed([1, 0, 0]), RowsFed([2, 3, 0])]),
        Report::Again => (RuleIndex(0), vec![RowsFed([4, 0, 0]), RowsFed([0, 0, 5])]),
        Report::Later => (RuleIndex(7), vec![RowsFed([1, 1, 1])]),
        Report::Empty => (RuleIndex(2), Vec::new()),
    };
    let Ok(mut expected) = adapter.totals() else {
        return false;
    };
    // The totals before the report plus the report, as the port promises to keep them.
    if expected.0.len() <= rule.0 {
        expected.0.resize_with(rule.0 + 1, Vec::new);
    }
    if let Some(steps) = expected.0.get_mut(rule.0) {
        if steps.len() < fed.len() {
            steps.resize(fed.len(), RowsFed::default());
        }
        for (total, more) in steps.iter_mut().zip(&fed) {
            for (slot, each) in total.0.iter_mut().zip(more.0) {
                *slot += each;
            }
        }
    }
    let reported = adapter.fed(
        rule,
        LogicalCounts {
            fed: fed.clone().into_boxed_slice(),
        },
    );
    reported.is_ok() && adapter.totals().is_ok_and(|after| after == expected)
}

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate. The rows run in order on the
/// one implementation, so each row's totals carry the rows before it.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl ObserverPort) {
    run(CASES, Word::Succeeds, |report| Ok(answer(adapter, report)));
}

/// Tells the adapter one handover of `file` and compares its handovers after it with those before, the one told put in the order
/// of the files.
fn handovers(adapter: &impl ObserverPort, file: FileId) -> Value {
    use ryo_values::domain::runs::types::FileHandover;
    use ryo_values::domain::runs::types::{ParsedRows, RowsHanded};
    let handover = FileHandover {
        file,
        rule: ryo_values::domain::runs::types::RuleNumber(3),
        handed: RowsHanded(6),
        parsed: ParsedRows(2),
    };
    let Ok(mut expected) = adapter.handovers() else {
        return false;
    };
    expected.0.push(handover);
    expected.0.sort_unstable();
    adapter.handed(handover) == Ok(()) && adapter.handovers().is_ok_and(|after| after == expected)
}
