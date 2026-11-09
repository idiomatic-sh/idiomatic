//! The counting adapter of the observer port: the core's contract rows run through it, then the cases specific to it.

use super::constants::COUNTED_FED;
use super::types::CountedFed;
use crate::outbound::observer::factory::counting;
use crate::outbound::observer::fixtures::contract::check;
use crate::outbound::observer::interface::ObserverPort as _;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

type Input = Bound;
type Value = bool;
type Failure = ();

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reports_of_one_rule_add_up_by_step"),
        word: Word::Succeeds,
        input: || Bound::Within,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_fresh_counted_fed_holds_no_totals"),
        word: Word::Succeeds,
        input: || Bound::Fresh,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_past_the_counted_fed_bound_is_not_kept"),
        word: Word::Succeeds,
        input: || Bound::Past,
        expect: Expect::Value(true),
    },
];

/// Where the reported rule lies against `COUNTED_FED`, the bound of the `CountedFed` the adapter keeps.
#[derive(Clone, Copy, Debug)]
enum Bound {
    Fresh,
    Within,
    Past,
}

/// Reports two rounds of a rule and reads the totals back.
fn subject(input: Input) -> Value {
    if let Bound::Fresh = input {
        return CountedFed::default().0.lock().is_ok_and(|totals| totals.0.is_empty());
    }
    let observer = counting();
    let rule = match input {
        Bound::Fresh | Bound::Within => RuleIndex(1),
        Bound::Past => RuleIndex(COUNTED_FED),
    };
    for _ in 0..2 {
        observer.fed(
            rule,
            LogicalCounts {
                fed: vec![RowsFed([1, 2, 3]), RowsFed([4, 0, 0])].into_boxed_slice(),
            },
        );
    }
    match input {
        Bound::Fresh | Bound::Within => {
            observer.totals().0 == vec![vec![], vec![RowsFed([2, 4, 6]), RowsFed([8, 0, 0])]]
        }
        Bound::Past => observer.totals().0.is_empty(),
    }
}

#[test]
fn succeeds() {
    check(&counting());
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
