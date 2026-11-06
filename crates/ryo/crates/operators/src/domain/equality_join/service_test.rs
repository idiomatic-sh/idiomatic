//! Ordered Join meaning: bag order, view mapping, refusal order and the prefix a downstream refusal keeps.
use super::service::join_row;
use super::types::{JoinCells, JoinDriver, JoinInterrupted, JoinRow};
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

#[derive(Clone, Copy)]
enum Scenario {
    JoinedRows,
    MissingLeftKey,
    EmptyMatches,
    FirstOutputFault,
    DownstreamInterruption,
    ColumnarLeft,
}
type Input = Scenario;
type Value = std::borrow::Cow<'static, [Cell]>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("joined_rows_preserve_bag_order_duplicate_columns_and_view_mapping"),
        word: Word::Succeeds,
        input: || Scenario::JoinedRows,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Cell::Number(Integer(1)),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(1)),
        ])),
    },
    Case {
        name: CaseName("empty_matches_do_not_access_invalid_output_columns"),
        word: Word::Succeeds,
        input: || Scenario::EmptyMatches,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("columnar_left_decodes_only_demanded_key_and_projection_cells"),
        word: Word::Succeeds,
        input: || Scenario::ColumnarLeft,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Cell::Number(Integer(8)),
            Cell::Number(Integer(8)),
            Cell::Number(Integer(8)),
        ])),
    },
    Case {
        name: CaseName("missing_left_key_is_refused_before_candidate_access"),
        word: Word::Refuses,
        input: || Scenario::MissingLeftKey,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: 3 }),
    },
    Case {
        name: CaseName("first_output_fault_stops_before_any_delivery"),
        word: Word::Refuses,
        input: || Scenario::FirstOutputFault,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: 99 }),
    },
    Case {
        name: CaseName("downstream_interruption_preserves_delivered_prefix_and_stops_scanning"),
        word: Word::Fails,
        input: || Scenario::DownstreamInterruption,
        expect: Expect::Error(PortableOperatorFault::Interrupted),
    },
];
fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
struct Driver {
    rows: Vec<Vec<Cell>>,
    at: usize,
    next_calls: usize,
    reads: std::cell::Cell<usize>,
    outputs: Vec<Vec<Cell>>,
    interrupt: Option<usize>,
    stopped: bool,
}
impl JoinDriver for Driver {
    fn next(&mut self, key: &[Cell]) -> Result<bool, JoinInterrupted> {
        if self.stopped {
            return Err(JoinInterrupted);
        }
        assert_eq!(key, &[number(7)]);
        self.next_calls += 1;
        self.at += 1;
        Ok(self.at <= self.rows.len())
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Cell> {
        self.reads.set(self.reads.get() + 1);
        self.rows
            .get(self.at - 1)?
            .get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), JoinInterrupted> {
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
struct Columns {
    reads: std::cell::RefCell<Vec<u32>>,
}
impl JoinCells for Columns {
    fn cell(&self, column: ColumnIndex) -> Option<Cell> {
        self.reads.borrow_mut().push(column.0);
        match column.0 {
            0 => Some(number(7)),
            999 => Some(number(8)),
            _ => None,
        }
    }
}
fn join(
    left: &[Cell],
    left_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    driver: &mut Driver,
) -> Result<(), PortableOperatorFault> {
    join_row(
        JoinRow::Values(left),
        left_on,
        output,
        view,
        driver,
        &mut vec![number(99)],
        &mut vec![number(88)],
    )
}
fn right(column: u32) -> JoinColumn {
    JoinColumn {
        side: Side::Right,
        column: ColumnIndex(column),
    }
}
fn flattened(driver: Driver) -> Value {
    std::borrow::Cow::Owned(driver.outputs.into_iter().flatten().collect())
}
fn subject(input: Input) -> Result<Value, Failure> {
    let mut d = driver();
    match input {
        Scenario::JoinedRows => {
            let columns = [
                right(0),
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(1),
                },
                right(0),
            ];
            join(
                &[number(7), number(9)],
                &[ColumnIndex(0)],
                &columns,
                &[ColumnIndex(1)],
                &mut d,
            )?;
            assert_eq!(d.reads.get(), 6);
        }
        Scenario::MissingLeftKey => {
            let result = join(&[], &[ColumnIndex(3)], &[], &[], &mut d);
            assert_eq!(d.next_calls, 0);
            result?;
        }
        Scenario::EmptyMatches => {
            d.rows.clear();
            join(&[number(7)], &[ColumnIndex(0)], &[right(99)], &[], &mut d)?;
            assert_eq!(d.reads.get(), 0);
        }
        Scenario::FirstOutputFault => {
            let result = join(&[number(7)], &[ColumnIndex(0)], &[right(99)], &[], &mut d);
            assert!(d.outputs.is_empty());
            assert_eq!(d.next_calls, 1);
            result?;
        }
        Scenario::DownstreamInterruption => {
            d.interrupt = Some(1);
            let result = join(&[number(7)], &[ColumnIndex(0)], &[right(1)], &[], &mut d);
            assert_eq!(d.outputs, vec![vec![number(1)]]);
            assert_eq!(d.next_calls, 2);
            result?;
        }
        Scenario::ColumnarLeft => {
            let columns = Columns {
                reads: std::cell::RefCell::new(vec![]),
            };
            join_row(
                JoinRow::Columns(&columns),
                &[ColumnIndex(0)],
                &[JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(999),
                }],
                &[],
                &mut d,
                &mut vec![],
                &mut vec![],
            )?;
            assert_eq!(*columns.reads.borrow(), vec![0, 999, 999, 999]);
        }
    }
    Ok(flattened(d))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
