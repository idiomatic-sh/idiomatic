//! The `counting` service's cases: a worker that scanned nothing counted nothing, and a rule set with no unit has no ledger row,
//! however many workers' counts are added up.

use super::service::{counted, ledger, merged};
use super::types::{LedgerRow, LedgerUnit};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::programs::service::scratch_measuring;
use crate::domain::programs::types::Programs;
use crate::domain::statistics::types::PointCounts;

/// How many workers' counts a row adds up.
type Input = usize;
/// The units of the ledger's rows.
type Value = Vec<LedgerUnit>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_set_with_no_unit_has_no_ledger_row"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("the_counts_of_three_workers_add_up_to_no_row_more"),
        word: Word::Succeeds,
        input: || 3,
        expect: Expect::Value(Vec::new()),
    },
];

fn subject(workers: Input) -> Result<Value, Failure> {
    let programs = Programs::default();
    let mut all = PointCounts::default();
    for _ in 0..workers {
        merged(&mut all, &counted(&scratch_measuring(&programs)));
    }
    Ok(ledger(&programs, &all)
        .0
        .iter()
        .map(|row: &LedgerRow| row.unit)
        .collect())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
