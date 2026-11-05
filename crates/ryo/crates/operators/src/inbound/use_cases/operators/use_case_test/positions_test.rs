//! Public acceptance of deferred position rows and bag order.
#[test]
fn position_representation_keeps_slots_deferred_ordered_and_duplicate() {
    use crate::domain::contract::types::{OperatorAnswer, OperatorCall};
    use crate::domain::equality_join::types::{JoinInput, JoinInterrupted, PositionJoinDriver};
    use crate::inbound::use_cases::operators::{
        interface::OperatorsUseCase, use_case::OperatorsUseCaseImpl,
    };
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
    let mut driver = Positions {
        at: 0,
        rows: vec![vec![9, 3], vec![1, 7], vec![9, 3]],
    };
    let mut target = vec![99];
    assert_eq!(
        OperatorsUseCaseImpl::new().operators(OperatorCall::EqualityJoin {
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
