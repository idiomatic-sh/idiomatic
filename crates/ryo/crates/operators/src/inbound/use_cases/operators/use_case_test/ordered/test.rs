//! The public ordered-pair call observes prepared inputs and immediately honors downstream refusal.
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
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    OrderedPairs,
    OrderedMissing,
    OrderedInterrupted,
}
#[derive(Debug, PartialEq)]
enum Observation {
    Cells(Cow<'static, [Cell]>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered_dispatch_preserves_fifo_witnesses_and_repeated_columns"),
        word: Word::Succeeds,
        input: || Scenario::OrderedPairs,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(1)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3)),
        ]))),
    },
    Case {
        name: CaseName("ordered_projection_fault_precedes_any_delivery"),
        word: Word::Refuses,
        input: || Scenario::OrderedMissing,
        expect: Expect::Error(Failure::NoSuchColumn { index: 2 }),
    },
    Case {
        name: CaseName("ordered_sink_failure_preserves_prefix_and_stops_reads"),
        word: Word::Fails,
        input: || Scenario::OrderedInterrupted,
        expect: Expect::Error(Failure::Interrupted),
    },
];
fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
fn written(call: OperatorCall<'_>) -> Result<(), Failure> {
    assert_eq!(
        fixtures::operators().operators(call)?,
        OperatorAnswer::Written
    );
    Ok(())
}
struct Driver {
    visits: Vec<(usize, usize)>,
    attempts: usize,
    rows: Vec<Cell>,
    fail_second: bool,
}
// Right0 is earlier than every open; Right1 and Right2 are the two FIFO witnesses.
const LEFT: &[(i64, i64)] = &[(1, 1), (1, 2), (1, 3)];
const RIGHT: &[(i64, i64)] = &[(1, 0), (1, 2), (1, 3), (1, 4)];
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
        self.visits.push((left, right));
        project(
            &[number(i64::try_from(left).unwrap())],
            &[number(i64::try_from(right).unwrap())],
        )
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), SemanticInterrupted> {
        self.attempts += 1;
        if self.fail_second && self.attempts == 2 {
            return Err(SemanticInterrupted);
        }
        self.rows.extend_from_slice(row);
        Ok(())
    }
}
fn subject(input: Scenario) -> Result<Observation, Failure> {
    let mut driver = Driver {
        visits: Vec::new(),
        attempts: 0,
        rows: Vec::new(),
        fail_second: matches!(input, Scenario::OrderedInterrupted),
    };
    let output = if matches!(input, Scenario::OrderedMissing) {
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
    let result = written(OperatorCall::OrderedMatch {
        output: &output,
        driver: &mut driver,
        target: &mut target,
    });
    match input {
        Scenario::OrderedPairs => {
            assert_eq!(driver.visits, vec![(0, 1), (1, 2), (2, 3)]);
            assert_eq!(driver.attempts, 3);
            assert_eq!(
                &driver.rows[6..],
                vec![number(3), number(2), number(3)],
                "third FIFO pair is checked separately"
            );
        }
        Scenario::OrderedMissing => {
            assert_eq!(driver.visits, vec![(0, 1)]);
            assert_eq!(driver.attempts, 0);
            assert!(driver.rows.is_empty());
        }
        Scenario::OrderedInterrupted => {
            assert_eq!(
                driver.visits,
                vec![(0, 1), (1, 2)],
                "third pair is never read after failure"
            );
            assert_eq!(driver.attempts, 2);
            assert_eq!(
                driver.rows,
                vec![number(1), number(0), number(1)],
                "first delivered row survives second delivery refusal"
            );
        }
    }
    result?;
    Ok(Observation::Cells(Cow::Owned(driver.rows)))
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
