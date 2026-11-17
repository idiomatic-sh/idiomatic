//! What the files handed over, read through `held_frontier` and `handed_coverage`: a held entry lends its rows and coverage, a
//! withheld one its token, and `reason` answers a token with the refusal `withheld` recorded for it.
use super::service::{handed_coverage, held_frontier, reason, withheld};
use super::types::{FrontierRows, HandedOver, Withholdings};
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::WithheldInput;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

/// The frontier entry read: a held one, one withheld for a missing producer, and one past the handover.
#[derive(Clone, Copy, Debug)]
enum Entry {
    /// Entry 0, held.
    Held,
    /// Entry 1, withheld after a first withheld input.
    Withheld,
    /// Entry 2, which the files did not hand over.
    Past,
}

type Input = Entry;
/// What the read answered, as text.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_held_entry_lends_its_rows_and_coverage"),
        word: Word::Succeeds,
        input: || Entry::Held,
        expect: Expect::Value(Cow::Borrowed(
            "held RowCount(0); Some(Complete); unknown token NotRun",
        )),
    },
    Case {
        name: CaseName("a_withheld_entry_answers_the_refusal_recorded_for_its_token"),
        word: Word::Succeeds,
        input: || Entry::Withheld,
        expect: Expect::Value(Cow::Borrowed(
            "withheld WithheldInput(1) ObservationIncomplete { reason: \"a producer feeding this retained frontier is unavailable\" } (first WithheldInput(0)); None; unknown token NotRun",
        )),
    },
    Case {
        name: CaseName("an_entry_past_the_handover_is_withheld_with_no_recorded_refusal"),
        word: Word::Succeeds,
        input: || Entry::Past,
        expect: Expect::Value(Cow::Borrowed(
            "withheld WithheldInput(4294967295) NotRun (first WithheldInput(0)); None; unknown token NotRun",
        )),
    },
];

fn subject(entry: Input) -> Result<Value, Failure> {
    let mut withholdings = Withholdings::default();
    let first = withheld(&mut withholdings, OperatingError::NotRead);
    let second = withheld(
        &mut withholdings,
        OperatingError::ObservationIncomplete {
            reason: "a producer feeding this retained frontier is unavailable".into(),
        },
    );
    let handed = HandedOver(vec![
        FrontierRows::Held {
            table: std::sync::Arc::new(Table::default()),
            coverage: CoverageState::Complete,
        },
        FrontierRows::Withheld(second),
    ]);
    let at = match entry {
        Entry::Held => 0,
        Entry::Withheld => 1,
        Entry::Past => 2,
    };
    let place = FrontierPlace::of(RuleOrdinal(0), InputNumber(at));
    let read = match held_frontier(&handed, place) {
        HeldRows::Held(table) => format!("held {:?}", table.count),
        HeldRows::Withheld(token) => format!(
            "withheld {token:?} {:?} (first {first:?})",
            reason(&withholdings, token)
        ),
    };
    let coverage = handed_coverage(&handed, InputNumber(at));
    let unknown = reason(&withholdings, WithheldInput(u32::MAX));
    Ok(Cow::Owned(format!(
        "{read}; {coverage:?}; unknown token {unknown:?}"
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
