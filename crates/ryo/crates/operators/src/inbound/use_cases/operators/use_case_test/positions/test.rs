//! Public acceptance of deferred position rows and bag order.
use crate::inbound::use_cases::operators::{fixtures, interface::OperatorsUseCase};
use crate::domain::capture::types::*;
use crate::domain::contract::types::*;
use crate::inbound::use_cases::operators::types::*;
use crate::domain::equality_join::types::*;
use crate::domain::ordered_existence::types::*;
use crate::domain::ordered_match::types::*;
use crate::domain::reduction::types::*;
use crate::domain::set_rows::types::*;
use crate::domain::source_matches::types::*;
use crate::domain::words::types::*;

struct Positions {
    at: usize,
    rows: Vec<Vec<u32>>,
}
impl PositionJoinDriver for Positions {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        self.at += 1;
        Ok(self.at <= self.rows.len())
    }
    fn right_positions(&self) -> &[u32] {
        &self.rows[self.at - 1]
    }
}
fn position_representation_keeps_slots_deferred_ordered_and_duplicate() {
    let mut driver = Positions {
        at: 0,
        rows: vec![vec![9, 3], vec![1, 7], vec![9, 3]],
    };
    let mut target = vec![99];
    assert_eq!(
        fixtures::operators().operators(OperatorCall::EqualityJoin {
            input: JoinInput::Positions {
                left: &[4, 2],
                driver: &mut driver,
                target: &mut target
            }
        }),
        Ok(OperatorAnswer::Written)
    );
    assert_eq!(target, vec![99, 4, 2, 9, 3, 4, 2, 1, 7, 4, 2, 9, 3]);
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("position_representation_keeps_slots_deferred_ordered_and_duplicate"),
    word: Word::Succeeds,
    input: || position_representation_keeps_slots_deferred_ordered_and_duplicate,
    expect: Expect::Value(()),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    input();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
