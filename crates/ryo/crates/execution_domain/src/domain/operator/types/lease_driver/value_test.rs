//! Proves the `LeaseDriver` value's behaviour through the operators' driver ports: each port reaches its
//! strategy with the shared row delivery, and the evaluation hands the workspace's buffer back.

use super::value::LeaseDriver;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::types::PrimitiveWorkspace;
use crate::domain::operator::fixtures::{Kept, Resources, extend, with_lease};
use crate::domain::table_model::types::Verdict;
use ryo_operators::domain::contract::types::RowDriver;
use ryo_operators::domain::equality_join::types::JoinEvaluation;
use ryo_relations::domain::relations::types::{Integer, Position, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a case does through the driver's ports.
type Input = fn(&mut LeaseDriver<'_, '_>) -> Result<(), ()>;
/// The rows the sink kept, and the positions the workspace holds once the evaluation ends.
type Value = (usize, usize);
/// The engine error the driver kept.
type Failure = Option<OperatingError>;

fn subject(case: Input) -> Result<Value, Failure> {
    let resources = Resources::default();
    let mut workspace = PrimitiveWorkspace {
        positions: vec![Position(7)],
        ..PrimitiveWorkspace::default()
    };
    let operation = extend();
    let mut sink = Kept::default();
    let answer = with_lease(&resources, Verdict::Holds, &mut workspace, |lease| {
        let mut driver = LeaseDriver::new(lease, &mut sink, &operation, PreparedInputs::Views(&[]));
        let answer = case(&mut driver).map_err(|()| driver.take_failure());
        let _lease = driver.finish();
        answer
    });
    answer.map(|()| (sink.0.len(), workspace.positions.len()))
}

/// The rows of this value's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_emitted_through_the_row_port_reaches_the_sink"),
        word: Word::Succeeds,
        input: || |driver| RowDriver::emit(driver, &[Cell::Number(Integer(1))]).map_err(|_| ()),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("the_join_port_reaches_the_join_candidates"),
        word: Word::Refuses,
        input: || {
            |driver| {
                driver
                    .join()
                    .next(&[Cell::Number(Integer(1))])
                    .map(|_| ())
                    .map_err(|_| ())
            }
        },
        expect: Expect::Error(Some(OperatingError::NotRun)),
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
