use super::*;
use crate::domain::contract::types::JoinInterrupted;
use ryo_relations::domain::relations::types::Integer;
fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}
struct Driver {
    rows: Vec<Vec<Value>>,
    at: usize,
    next_calls: usize,
    reads: std::cell::Cell<usize>,
    outputs: Vec<Vec<Value>>,
    interrupt: Option<usize>,
    stopped: bool,
}
impl JoinDriver for Driver {
    fn next(&mut self, key: &[Value]) -> Result<bool, JoinInterrupted> {
        if self.stopped {
            return Err(JoinInterrupted);
        }
        assert_eq!(key, &[number(7)]);
        self.next_calls += 1;
        self.at += 1;
        Ok(self.at <= self.rows.len())
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Value> {
        self.reads.set(self.reads.get() + 1);
        self.rows.get(self.at - 1)?.get(column.0 as usize).copied()
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), JoinInterrupted> {
        if self.stopped {
            return Err(JoinInterrupted);
        }
        if self.interrupt == Some(self.outputs.len()) {
            self.stopped = true;
            return Err(JoinInterrupted);
        }
        self.outputs.push(row.to_vec());
        Ok(())
    }
}
fn driver() -> Driver {
    Driver {
        rows: vec![
            vec![number(7), number(1)],
            vec![number(7), number(2)],
            vec![number(7), number(1)],
        ],
        at: 0,
        next_calls: 0,
        reads: std::cell::Cell::new(0),
        outputs: vec![],
        interrupt: None,
        stopped: false,
    }
}
#[test]
fn joined_rows_preserve_bag_order_duplicate_columns_and_view_mapping() {
    let mut d = driver();
    let mut key = vec![number(99)];
    let mut target = vec![number(88)];
    let columns = [
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(0),
        },
        JoinColumn {
            side: Side::Left,
            column: ColumnIndex(1),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(0),
        },
    ];
    assert_eq!(
        join(
            &[number(7), number(9)],
            &[ColumnIndex(0)],
            &columns,
            &[ColumnIndex(1)],
            &mut d,
            &mut key,
            &mut target
        ),
        Ok(())
    );
    assert_eq!(
        d.outputs,
        vec![
            vec![number(1), number(9), number(1)],
            vec![number(2), number(9), number(2)],
            vec![number(1), number(9), number(1)]
        ]
    );
    assert_eq!(d.reads.get(), 6);
}
#[test]
fn missing_left_key_is_refused_before_candidate_access() {
    let mut d = driver();
    assert_eq!(
        join(
            &[],
            &[ColumnIndex(3)],
            &[],
            &[],
            &mut d,
            &mut vec![],
            &mut vec![]
        ),
        Err(PortableOperatorFault::NoSuchColumn { index: 3 })
    );
    assert_eq!(d.next_calls, 0);
}
#[test]
fn empty_matches_do_not_access_invalid_output_columns() {
    let mut d = driver();
    d.rows.clear();
    assert_eq!(
        join(
            &[number(7)],
            &[ColumnIndex(0)],
            &[JoinColumn {
                side: Side::Right,
                column: ColumnIndex(99)
            }],
            &[],
            &mut d,
            &mut vec![],
            &mut vec![]
        ),
        Ok(())
    );
    assert_eq!(d.reads.get(), 0);
}
#[test]
fn first_output_fault_stops_before_any_delivery() {
    let mut d = driver();
    assert_eq!(
        join(
            &[number(7)],
            &[ColumnIndex(0)],
            &[JoinColumn {
                side: Side::Right,
                column: ColumnIndex(99)
            }],
            &[],
            &mut d,
            &mut vec![],
            &mut vec![]
        ),
        Err(PortableOperatorFault::NoSuchColumn { index: 99 })
    );
    assert!(d.outputs.is_empty());
    assert_eq!(d.next_calls, 1);
}
#[test]
fn downstream_interruption_preserves_delivered_prefix_and_stops_scanning() {
    let mut d = driver();
    d.interrupt = Some(1);
    assert_eq!(
        join(
            &[number(7)],
            &[ColumnIndex(0)],
            &[JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1)
            }],
            &[],
            &mut d,
            &mut vec![],
            &mut vec![]
        ),
        Err(PortableOperatorFault::Interrupted)
    );
    assert_eq!(d.outputs, vec![vec![number(1)]]);
    assert_eq!(d.next_calls, 2);
}

#[test]
fn position_representation_keeps_slots_deferred_ordered_and_duplicate() {
    use crate::domain::contract::types::{
        JoinInput, OperatorAnswer, OperatorCall, PositionJoinDriver,
    };
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
