//! Public acceptance of extrema, projection, antijoin and interruption semantics.
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
use ryo_relations::domain::relations::types::{Integer, Value as ValueCell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

struct Driver {
    rows: Vec<[ValueCell; 3]>,
    emitted: Vec<usize>,
    fail_read: Option<usize>,
    fail_emit: bool,
    failed: bool,
}
impl OrderedExistenceDriver for Driver {
    fn len(&self) -> RowCount {
        RowCount(self.rows.len())
    }
    fn key_cell(&mut self, row: RowIndex, column: ColumnIndex) -> Result<ValueCell, EqualityJoinError> {
        if self.failed {
            return Err(EqualityJoinError::Interrupted);
        }
        self.rows
            .get(row.0)
            .and_then(|row| row.get(column.0 as usize))
            .copied()
            .ok_or(EqualityJoinError::Interrupted)
    }
    fn number(&mut self, row: RowIndex, column: ColumnIndex) -> Result<Integer, EqualityJoinError> {
        if self.failed || self.fail_read == Some(row.0) {
            self.failed = true;
            return Err(EqualityJoinError::Interrupted);
        }
        match self
            .rows
            .get(row.0)
            .and_then(|row| row.get(column.0 as usize))
        {
            Some(ValueCell::Number(value)) => Ok(*value),
            _ => {
                self.failed = true;
                Err(EqualityJoinError::Interrupted)
            }
        }
    }
    fn emit_dominated(&mut self, row: RowIndex, witness: RowIndex) -> Result<(), EqualityJoinError> {
        if self.failed || self.fail_emit {
            self.failed = true;
            return Err(EqualityJoinError::Interrupted);
        }
        assert_eq!(self.rows[row.0][0], self.rows[witness.0][0]);
        assert_ne!(self.rows[row.0][2], self.rows[witness.0][2]);
        self.emitted.push(row.0);
        Ok(())
    }
}
fn driver(rows: &[[i64; 3]]) -> Driver {
    Driver {
        rows: rows
            .iter()
            .map(|row| row.map(|n| ValueCell::Number(Integer(n))))
            .collect(),
        emitted: Vec::new(),
        fail_read: None,
        fail_emit: false,
        failed: false,
    }
}
fn execute(
    driver: &mut Driver,
    extremum: PartitionExtremum,
    scratch: &mut OrderedExistenceScratch,
) -> Result<(), PortableOperatorFault> {
    fixtures::operators()
        .operators(OperatorCall::OrderedExistence {
            partition: &[ColumnIndex(0)],
            order: ColumnIndex(2),
            extremum,
            driver,
            scratch,
        })
        .map(|_| ())
}
fn oracle(rows: &[[ValueCell; 3]], extremum: PartitionExtremum) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(at, left)| {
            rows.iter()
                .any(|right| {
                    left[0] == right[0]
                        && match (left[2], right[2]) {
                            (ValueCell::Number(a), ValueCell::Number(b)) => match extremum {
                                PartitionExtremum::Maximum => a < b,
                                PartitionExtremum::Minimum => a > b,
                            },
                            _ => false,
                        }
                })
                .then_some(at)
        })
        .collect()
}

struct Poisoned {
    keys: Vec<ValueCell>,
    rows: Vec<Vec<ValueCell>>,
}
impl SetRowDriver for Poisoned {
    fn right_empty(&mut self) -> Result<Population, SetRowsError> {
        Ok(Population::from(self.keys.is_empty()))
    }
    fn contains_right(&mut self, key: &[ValueCell]) -> Result<Membership, SetRowsError> {
        Ok(Membership::from(self.keys.contains(&key[0])))
    }
    fn first_time(&mut self, _: &[ValueCell]) -> Result<Occurrence, SetRowsError> {
        panic!("antijoin must preserve left bags")
    }
    fn emit(&mut self, row: &[ValueCell]) -> Result<(), SetRowsError> {
        self.rows.push(row.to_vec());
        Ok(())
    }
}

struct Reordered {
    dropped: Vec<Vec<ValueCell>>,
    emitted: Vec<Vec<ValueCell>>,
}
impl SetRowDriver for Reordered {
    fn right_empty(&mut self) -> Result<Population, SetRowsError> {
        Ok(Population::from(self.dropped.is_empty()))
    }
    fn contains_right(&mut self, key: &[ValueCell]) -> Result<Membership, SetRowsError> {
        Ok(Membership::from(self.dropped.iter().any(|row| row == key)))
    }
    fn first_time(&mut self, _: &[ValueCell]) -> Result<Occurrence, SetRowsError> {
        panic!("bag-preserving antijoin")
    }
    fn emit(&mut self, row: &[ValueCell]) -> Result<(), SetRowsError> {
        self.emitted.push(row.to_vec());
        Ok(())
    }
}

fn finite_extrema_matches_pair_oracle_with_ties_bags_partitions_and_integer_limits() {
    let cases: &[&[[i64; 3]]] = &[
        &[],
        &[[0, 0, 5]],
        &[[0, 0, 5], [0, 1, 5]],
        &[[0, 0, 5], [0, 0, 7], [0, 1, 6]],
        &[[0, 0, 5], [0, 0, 5], [0, 1, 7], [0, 2, 7]],
        &[[0, 0, 7], [1, 1, 3], [0, 2, 5], [1, 3, 3]],
        &[[0, 0, i64::MIN], [0, 1, i64::MAX], [1, 2, i64::MIN]],
    ];
    let mut scratch = OrderedExistenceScratch::default();
    for case in cases {
        for extremum in [PartitionExtremum::Maximum, PartitionExtremum::Minimum] {
            let mut input = driver(case);
            let expected = oracle(&input.rows, extremum);
            execute(&mut input, extremum, &mut scratch).expect("admitted total input");
            assert_eq!(input.emitted, expected);
            assert!(scratch.groups.is_empty());
            assert!(scratch.key.is_empty());
        }
    }
}

fn projected_dominated_keys_poison_the_entire_unchanged_antijoin_key_group() {
    // Review arithmetic corrected: maximum7 belongs to k, not k'. Both keys are
    // produced by dominated rows, so the unchanged antijoin removes ALL THREE.
    let mut input = driver(&[[0, 0, 5], [0, 0, 7], [0, 1, 6]]);
    execute(
        &mut input,
        PartitionExtremum::Maximum,
        &mut OrderedExistenceScratch::default(),
    )
    .expect("flag");
    assert_eq!(input.emitted, [0, 2]);
    let mut anti = Poisoned {
        keys: input.emitted.iter().map(|at| input.rows[*at][1]).collect(),
        rows: Vec::new(),
    };
    for row in &input.rows {
        fixtures::operators()
            .operators(OperatorCall::SetRow {
                input: SetInput::Values {
                    kind: SetKind::Antijoin,
                    row,
                    left_on: &[ColumnIndex(1)],
                    driver: &mut anti,
                    key: &mut Vec::new(),
                },
            })
            .expect("unchanged antijoin");
    }
    assert!(anti.rows.is_empty());
}

fn read_and_emit_interruption_stop_callbacks_and_allow_clean_scratch_reuse() {
    let mut scratch = OrderedExistenceScratch::default();
    let mut input = driver(&[[0, 0, 1], [0, 1, 2]]);
    input.fail_read = Some(1);
    assert_eq!(
        execute(&mut input, PartitionExtremum::Maximum, &mut scratch),
        Err(PortableOperatorFault::Interrupted)
    );
    assert!(input.emitted.is_empty());
    assert!(scratch.groups.is_empty());
    let mut input = driver(&[[0, 0, 1], [0, 1, 2]]);
    input.fail_emit = true;
    assert_eq!(
        execute(&mut input, PartitionExtremum::Maximum, &mut scratch),
        Err(PortableOperatorFault::Interrupted)
    );
    assert!(input.emitted.is_empty());
    assert!(scratch.groups.is_empty());
    let mut input = driver(&[[0, 0, 10], [0, 1, 10]]);
    execute(&mut input, PartitionExtremum::Maximum, &mut scratch).expect("reused scratch");
    assert!(input.emitted.is_empty());
}

fn original_project_reads_join_coordinates_with_reordered_repeated_left_columns() {
    let source = driver(&[[10, 20, 5], [10, 21, 7], [11, 22, 9], [11, 22, 9]]).rows;
    let mut input = driver(&[[10, 20, 5], [10, 21, 7], [11, 22, 9], [11, 22, 9]]);
    execute(
        &mut input,
        PartitionExtremum::Maximum,
        &mut OrderedExistenceScratch::default(),
    )
    .expect("flag");
    let mut dropped = Vec::new();
    for left in input.emitted {
        let witness = source
            .iter()
            .find(|right| right[0] == source[left][0] && right[2] > source[left][2])
            .expect("real dominating witness");
        // J=[Left1,Left0,Left1,Right2]; P=[0,2] reads S[1] twice.
        let joined = [
            source[left][1],
            source[left][0],
            source[left][1],
            witness[2],
        ];
        let mut projected = Vec::new();
        fixtures::operators()
            .operators(OperatorCall::Project {
                row: &joined,
                columns: &[ColumnIndex(0), ColumnIndex(2)],
                target: &mut projected,
            })
            .expect("unchanged project");
        dropped.push(projected);
    }
    assert_eq!(dropped, vec![vec![source[0][1], source[0][1]]]);
    let mut anti = Reordered {
        dropped,
        emitted: Vec::new(),
    };
    for row in &source {
        fixtures::operators()
            .operators(OperatorCall::SetRow {
                input: SetInput::Values {
                    kind: SetKind::Antijoin,
                    row,
                    left_on: &[ColumnIndex(1), ColumnIndex(1)],
                    driver: &mut anti,
                    key: &mut Vec::new(),
                },
            })
            .expect("unchanged antijoin");
    }
    assert_eq!(
        anti.emitted,
        source[1..]
            .iter()
            .map(|row| row.to_vec())
            .collect::<Vec<_>>()
    );
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "finite_extrema_matches_pair_oracle_with_ties_bags_partitions_and_integer_limits",
        ),
        word: Word::Succeeds,
        input: || finite_extrema_matches_pair_oracle_with_ties_bags_partitions_and_integer_limits,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("projected_dominated_keys_poison_the_entire_unchanged_antijoin_key_group"),
        word: Word::Succeeds,
        input: || projected_dominated_keys_poison_the_entire_unchanged_antijoin_key_group,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("read_and_emit_interruption_stop_callbacks_and_allow_clean_scratch_reuse"),
        word: Word::Succeeds,
        input: || read_and_emit_interruption_stop_callbacks_and_allow_clean_scratch_reuse,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "original_project_reads_join_coordinates_with_reordered_repeated_left_columns",
        ),
        word: Word::Succeeds,
        input: || original_project_reads_join_coordinates_with_reordered_repeated_left_columns,
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    input();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
