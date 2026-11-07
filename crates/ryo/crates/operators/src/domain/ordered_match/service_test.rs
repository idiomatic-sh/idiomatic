//! FIFO witnesses, repeated projections and first-failure delivery use independent expected pairs.
use super::service::pairs;
use crate::domain::contract::types::{PortableOperatorFault};
use crate::domain::contract::errors::ContractError;
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
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: ryo_values::domain::plan_parts::types::ColumnIndex(2) }),
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
    fn left_len(&self) -> ryo_values::domain::rows::types::RowCount {
        ryo_values::domain::rows::types::RowCount(LEFT.len())
    }
    fn right_len(&self) -> ryo_values::domain::rows::types::RowCount {
        ryo_values::domain::rows::types::RowCount(RIGHT.len())
    }
    fn compare_keys(&self, left: ryo_values::domain::rows::types::RowIndex, right: ryo_values::domain::rows::types::RowIndex) -> std::cmp::Ordering {
        LEFT[left.0].0.cmp(&RIGHT[right.0].0)
    }
    fn right_order_before_or_equal(&self, left: ryo_values::domain::rows::types::RowIndex, right: ryo_values::domain::rows::types::RowIndex) -> crate::domain::ordered_match::types::KeyOrder {
        if RIGHT[right.0].1 <= LEFT[left.0].1 {
            crate::domain::ordered_match::types::KeyOrder::BeforeOrEqual
        } else {
            crate::domain::ordered_match::types::KeyOrder::After
        }
    }
    fn visit_pair(
        &mut self,
        left: ryo_values::domain::rows::types::RowIndex,
        right: ryo_values::domain::rows::types::RowIndex,
        project: &mut dyn FnMut(&[Cell], &[Cell]) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        self.reads += 1;
        project(
            &[Cell::Number(Integer(i64::try_from(left.0).unwrap()))],
            &[Cell::Number(Integer(i64::try_from(right.0).unwrap()))],
        )
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), ContractError> {
        self.deliveries += 1;
        if matches!(self.scenario, Scenario::Interrupted) {
            return Err(ContractError::Interrupted);
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
