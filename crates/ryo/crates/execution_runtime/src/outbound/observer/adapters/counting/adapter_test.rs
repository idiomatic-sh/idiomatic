//! The counting adapter of the observer port: the core's contract rows run through it, then the cases specific to it.

use crate::outbound::observer::factory::counting;
use crate::outbound::observer::fixtures::contract::check;
use crate::outbound::observer::interface::ExecutionObserver as _;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
type Value = bool;
type Failure = ();

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("reports_of_one_rule_add_up_by_step"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Reports two rounds of one rule and reads the totals back.
fn subject(_input: Input) -> Value {
    let observer = counting(&[2]);
    for _ in 0..2 {
        observer.fed(
            RuleIndex(0),
            LogicalCounts {
                fed: vec![RowsFed([1, 2, 3]), RowsFed([4, 0, 0])].into_boxed_slice(),
            },
        );
    }
    observer.totals() == vec![vec![RowsFed([2, 4, 6]), RowsFed([8, 0, 0])]]
}

#[test]
fn succeeds() {
    check(&counting(&[2, 1]));
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
