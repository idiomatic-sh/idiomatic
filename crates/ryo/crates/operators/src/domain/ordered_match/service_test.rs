//! FIFO witnesses, repeated projections and first-failure delivery use independent expected pairs.
use super::service::pairs;
use crate::domain::contract::types::{PortableOperatorFault, SemanticInterrupted};
use crate::domain::ordered_match::types::OrderedMatchDriver;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
#[derive(Clone, Copy)]
enum Scenario {
    Pairs,
    MissingColumn,
    Interrupted,
}
type Input = Scenario;
type Value = std::borrow::Cow<'static, [Cell]>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fifo_pairs_skip_same_or_earlier_order_and_preserve_projection"),
        word: Word::Succeeds,
        input: || Scenario::Pairs,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Cell::Number(Integer(1)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(5)),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(5)),
        ])),
    },
    Case {
        name: CaseName("missing_projection_column_fails_before_delivery"),
        word: Word::Refuses,
        input: || Scenario::MissingColumn,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: 2 }),
    },
    Case {
        name: CaseName("sink_failure_stops_further_reads"),
        word: Word::Refuses,
        input: || Scenario::Interrupted,
        expect: Expect::Error(PortableOperatorFault::Interrupted),
    },
];
struct Driver {
    scenario: Scenario,
    reads: usize,
    deliveries: usize,
    rows: Vec<Cell>,
}
const LEFT: &[(i64, i64)] = &[(1, 1), (1, 2), (2, 1), (3, 2)];
const RIGHT: &[(i64, i64)] = &[(1, 0), (1, 2), (1, 3), (2, 2), (3, 1), (3, 3)];
impl OrderedMatchDriver for Driver {
    fn left_len(&self) -> usize {
        LEFT.len()
    }
    fn right_len(&self) -> usize {
        RIGHT.len()
    }
    fn compare_keys(&self, left: usize, right: usize) -> std::cmp::Ordering {
        LEFT[left].0.cmp(&RIGHT[right].0)
    }
    fn right_order_before_or_equal(&self, left: usize, right: usize) -> bool {
        RIGHT[right].1 <= LEFT[left].1
    }
    fn visit_pair(
        &mut self,
        left: usize,
        right: usize,
        project: &mut dyn FnMut(&[Cell], &[Cell]) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        self.reads += 1;
        project(
            &[Cell::Number(Integer(i64::try_from(left).unwrap()))],
            &[Cell::Number(Integer(i64::try_from(right).unwrap()))],
        )
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), SemanticInterrupted> {
        self.deliveries += 1;
        if matches!(self.scenario, Scenario::Interrupted) {
            return Err(SemanticInterrupted);
        }
        self.rows.extend_from_slice(row);
        Ok(())
    }
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut driver = Driver {
        scenario,
        reads: 0,
        deliveries: 0,
        rows: Vec::new(),
    };
    let output = if matches!(scenario, Scenario::MissingColumn) {
        vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(2),
        }]
    } else {
        vec![
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
        ]
    };
    let mut target = Vec::new();
    let result = pairs(&output, &mut driver, &mut target);
    match scenario {
        Scenario::Pairs => {
            assert_eq!(driver.reads, 4);
            assert_eq!(driver.deliveries, 4);
        }
        Scenario::MissingColumn => {
            assert_eq!(driver.reads, 1);
            assert_eq!(driver.deliveries, 0);
        }
        Scenario::Interrupted => {
            assert_eq!(driver.reads, 1);
            assert_eq!(driver.deliveries, 1);
        }
    }
    result?;
    Ok(std::borrow::Cow::Owned(driver.rows))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
