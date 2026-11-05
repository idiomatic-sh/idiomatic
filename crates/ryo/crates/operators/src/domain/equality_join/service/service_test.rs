use super::*;
use crate::domain::equality_join::types::JoinInterrupted;
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
        self.rows
            .get(self.at - 1)?
            .get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
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
fn columnar_left_decodes_only_demanded_key_and_projection_cells() {
    use crate::domain::equality_join::types::{JoinCells, JoinRow};
    struct Columns {
        reads: std::cell::RefCell<Vec<u32>>,
    }
    impl JoinCells for Columns {
        fn cell(&self, column: ColumnIndex) -> Option<Value> {
            self.reads.borrow_mut().push(column.0);
            match column.0 {
                0 => Some(number(7)),
                999 => Some(number(8)),
                _ => None,
            }
        }
    }
    let columns = Columns {
        reads: std::cell::RefCell::new(vec![]),
    };
    let mut d = driver();
    assert_eq!(
        join_row(
            JoinRow::Columns(&columns),
            &[ColumnIndex(0)],
            &[JoinColumn {
                side: Side::Left,
                column: ColumnIndex(999)
            }],
            &[],
            &mut d,
            &mut vec![],
            &mut vec![]
        ),
        Ok(())
    );
    assert_eq!(*columns.reads.borrow(), vec![0, 999, 999, 999]);
    assert_eq!(d.outputs, vec![vec![number(8)]; 3]);
}
