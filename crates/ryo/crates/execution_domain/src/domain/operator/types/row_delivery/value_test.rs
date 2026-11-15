//! Proves the `RowDelivery` value's behaviour: rows reach the sink until an engine error is kept, and the
//! first error kept is the one the evaluation answers.

use super::value::RowDelivery;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::InputPlace;
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::types::PrimitiveWorkspace;
use crate::domain::operator::fixtures::{Kept, Resources, extend, with_lease};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::InputNumber;

/// What a case does with a row delivery over a row-mode evaluation of one number.
type Input = fn(&mut RowDelivery<'_, '_>) -> Result<PredicateOutcome, ()>;
/// The rows the sink kept, and the predicate's outcome when the case asked for it.
type Value = (usize, PredicateOutcome);
/// The engine error the delivery kept.
type Failure = Option<OperatingError>;

fn subject(case: Input) -> Result<Value, Failure> {
    let resources = Resources::default();
    let mut workspace = PrimitiveWorkspace::default();
    let operation = extend();
    let row = [Cell::Number(Integer(1))];
    let mut sink = Kept::default();
    let answer = with_lease(
        &resources,
        PredicateOutcome::Fails,
        &mut workspace,
        |lease| {
            let mut rows = RowDelivery::new(
                lease,
                &mut sink,
                &operation,
                PreparedInputs::Row {
                    input: InputPlace(0),
                    row: &row,
                    sides: &[],
                },
            );
            case(&mut rows).map_err(|()| rows.take_failure())
        },
    );
    answer.map(|outcome| (sink.0.len(), outcome))
}

fn emit_one(rows: &mut RowDelivery<'_, '_>) -> Result<PredicateOutcome, ()> {
    rows.emit(&[Cell::Number(Integer(2))]).map_err(|_| ())?;
    Ok(PredicateOutcome::Holds)
}

fn emit_after_a_kept_error(rows: &mut RowDelivery<'_, '_>) -> Result<PredicateOutcome, ()> {
    let _interrupted = rows.hold::<()>(Err(OperatingError::NotRun));
    let _later = rows.hold::<()>(Err(OperatingError::WrongBinding));
    rows.emit(&[Cell::Number(Integer(2))]).map_err(|_| ())?;
    Ok(PredicateOutcome::Holds)
}

fn ask_the_predicate(rows: &mut RowDelivery<'_, '_>) -> Result<PredicateOutcome, ()> {
    rows.holds(&[Cell::Number(Integer(2))]).map_err(|_| ())
}

fn count_a_population_of_a_row(rows: &mut RowDelivery<'_, '_>) -> Result<PredicateOutcome, ()> {
    let _rows = rows.population(InputNumber(0)).map_err(|_| ())?;
    Ok(PredicateOutcome::Holds)
}

/// The rows of this value's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_emitted_row_reaches_the_sink"),
        word: Word::Succeeds,
        input: || emit_one,
        expect: Expect::Value((1, PredicateOutcome::Holds)),
    },
    Case {
        name: CaseName("the_bound_predicate_decides_the_outcome"),
        word: Word::Succeeds,
        input: || ask_the_predicate,
        expect: Expect::Value((0, PredicateOutcome::Fails)),
    },
    Case {
        name: CaseName("the_first_kept_error_interrupts_the_next_emit"),
        word: Word::Fails,
        input: || emit_after_a_kept_error,
        expect: Expect::Error(Some(OperatingError::NotRun)),
    },
    Case {
        name: CaseName("a_row_evaluation_has_no_population"),
        word: Word::Refuses,
        input: || count_a_population_of_a_row,
        expect: Expect::Error(Some(OperatingError::NotRun)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
