//! The counting adapter of the observer port: the core's contract rows run through it, then the cases specific to it.

use super::constants::COUNTED_FED;
use super::constants::COUNTED_HANDOVERS;
use super::types::CountedFed;
use super::types::CountedHandovers;
use crate::outbound::observer::factory::counting;
use crate::outbound::observer::fixtures::contract::check;
use crate::outbound::observer::interface::ObserverPort as _;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
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
        name: CaseName("a_rule_past_the_counted_fed_bound_is_refused_and_not_kept"),
        word: Word::Succeeds,
        input: || Bound::Past,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_fresh_counted_handovers_holds_no_handover"),
        word: Word::Succeeds,
        input: || Bound::FreshHandovers,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_handover_past_the_counted_handovers_bound_is_refused_and_not_kept"),
        word: Word::Succeeds,
        input: || Bound::PastHandovers,
        expect: Expect::Value(true),
    },
];

/// Where the reported rule lies against `COUNTED_FED`, the bound of the `CountedFed` the adapter keeps.
#[derive(Clone, Copy, Debug)]
enum Bound {
    Fresh,
    Within,
    Past,
    FreshHandovers,
    PastHandovers,
}

/// Reports two rounds of a rule and reads the totals back.
fn subject(input: Input) -> Value {
    match input {
        Bound::Fresh => {
            return CountedFed::default()
                .0
                .lock()
                .is_ok_and(|totals| totals.0.is_empty());
        }
        Bound::FreshHandovers => {
            return CountedHandovers::default()
                .0
                .lock()
                .is_ok_and(|told| told.is_empty());
        }
        Bound::PastHandovers => return past_handovers(),
        Bound::Within | Bound::Past => {}
    }
    let observer = counting();
    let rule = match input {
        Bound::Fresh | Bound::Within | Bound::FreshHandovers | Bound::PastHandovers => RuleIndex(1),
        Bound::Past => RuleIndex(COUNTED_FED),
    };
    let mut refused = Vec::new();
    for _ in 0..2 {
        let report = observer.fed(
            rule,
            LogicalCounts {
                fed: vec![RowsFed([1, 2, 3]), RowsFed([4, 0, 0])].into_boxed_slice(),
            },
        );
        refused.push(report.err());
    }
    match input {
        Bound::Fresh | Bound::Within | Bound::FreshHandovers | Bound::PastHandovers => {
            refused == [None, None]
                && observer.totals().is_ok_and(|totals| {
                    totals.0 == vec![vec![], vec![RowsFed([2, 4, 6]), RowsFed([8, 0, 0])]]
                })
        }
        Bound::Past => {
            refused
                == [
                    Some(OperatingError::ShapeMismatch),
                    Some(OperatingError::ShapeMismatch),
                ]
                && observer.totals().is_ok_and(|totals| totals.0.is_empty())
        }
    }
}

/// Tells the observer `COUNTED_HANDOVERS` handovers and one more: the one past the bound is refused, and those within are kept.
fn past_handovers() -> Value {
    use ryo_values::domain::runs::types::FileHandover;
    use ryo_values::domain::runs::types::{ParsedRows, RowsHanded};
    let observer = counting();
    let handover = |file: usize| FileHandover {
        file: ryo_relations::domain::relations::types::FileId(
            u32::try_from(file).unwrap_or(u32::MAX),
        ),
        rule: ryo_values::domain::runs::types::RuleNumber(0),
        handed: RowsHanded(1),
        parsed: ParsedRows(1),
    };
    let within = (0..COUNTED_HANDOVERS).all(|file| observer.handed(handover(file)) == Ok(()));
    within
        && observer.handed(handover(COUNTED_HANDOVERS)) == Err(OperatingError::ShapeMismatch)
        && observer
            .handovers()
            .is_ok_and(|told| told.len() == COUNTED_HANDOVERS)
}

#[test]
fn succeeds() {
    check(&counting());
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
