//! Public acceptance of extrema, projection, antijoin and interruption semantics.
use crate::domain::contract::types::OperatorCall;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::JoinInterrupted;
use crate::domain::ordered_existence::types::{
    OrderedExistenceDriver, OrderedExistenceScratch, PartitionExtremum,
};
use crate::domain::set_rows::types::{SetInput, SetInterrupted, SetKind, SetRowDriver};
use crate::inbound::use_cases::operators::{
    interface::OperatorsUseCase, use_case::OperatorsUseCaseImpl,
};
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

struct Driver {
    rows: Vec<[Value; 3]>,
    emitted: Vec<usize>,
    fail_read: Option<usize>,
    fail_emit: bool,
    failed: bool,
}
impl OrderedExistenceDriver for Driver {
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn key_cell(&mut self, row: usize, column: ColumnIndex) -> Result<Value, JoinInterrupted> {
        if self.failed {
            return Err(JoinInterrupted);
        }
        self.rows
            .get(row)
            .and_then(|row| row.get(column.0 as usize))
            .copied()
            .ok_or(JoinInterrupted)
    }
    fn number(&mut self, row: usize, column: ColumnIndex) -> Result<Integer, JoinInterrupted> {
        if self.failed || self.fail_read == Some(row) {
            self.failed = true;
            return Err(JoinInterrupted);
        }
        match self
            .rows
            .get(row)
            .and_then(|row| row.get(column.0 as usize))
        {
            Some(Value::Number(value)) => Ok(*value),
            _ => {
                self.failed = true;
                Err(JoinInterrupted)
            }
        }
    }
    fn emit_dominated(&mut self, row: usize, witness: usize) -> Result<(), JoinInterrupted> {
        if self.failed || self.fail_emit {
            self.failed = true;
            return Err(JoinInterrupted);
        }
        assert_eq!(self.rows[row][0], self.rows[witness][0]);
        assert_ne!(self.rows[row][2], self.rows[witness][2]);
        self.emitted.push(row);
        Ok(())
    }
}
fn driver(rows: &[[i64; 3]]) -> Driver {
    Driver {
        rows: rows
            .iter()
            .map(|row| row.map(|n| Value::Number(Integer(n))))
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
    OperatorsUseCaseImpl::new()
        .operators(OperatorCall::OrderedExistence {
            partition: &[ColumnIndex(0)],
            order: ColumnIndex(2),
            extremum,
            driver,
            scratch,
        })
        .map(|_| ())
}
fn oracle(rows: &[[Value; 3]], extremum: PartitionExtremum) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(at, left)| {
            rows.iter()
                .any(|right| {
                    left[0] == right[0]
                        && match (left[2], right[2]) {
                            (Value::Number(a), Value::Number(b)) => match extremum {
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
    struct Anti {
        keys: Vec<Value>,
        rows: Vec<Vec<Value>>,
    }
    impl SetRowDriver for Anti {
        fn right_empty(&mut self) -> Result<bool, SetInterrupted> {
            Ok(self.keys.is_empty())
        }
        fn contains_right(&mut self, key: &[Value]) -> Result<bool, SetInterrupted> {
            Ok(self.keys.contains(&key[0]))
        }
        fn first_time(&mut self, _: &[Value]) -> Result<bool, SetInterrupted> {
            panic!("antijoin must preserve left bags")
        }
        fn emit(&mut self, row: &[Value]) -> Result<(), SetInterrupted> {
            self.rows.push(row.to_vec());
            Ok(())
        }
    }
    let mut anti = Anti {
        keys: input.emitted.iter().map(|at| input.rows[*at][1]).collect(),
        rows: Vec::new(),
    };
    for row in &input.rows {
        OperatorsUseCaseImpl::new()
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
        OperatorsUseCaseImpl::new()
            .operators(OperatorCall::Project {
                row: &joined,
                columns: &[ColumnIndex(0), ColumnIndex(2)],
                target: &mut projected,
            })
            .expect("unchanged project");
        dropped.push(projected);
    }
    struct Anti {
        dropped: Vec<Vec<Value>>,
        emitted: Vec<Vec<Value>>,
    }
    impl SetRowDriver for Anti {
        fn right_empty(&mut self) -> Result<bool, SetInterrupted> {
            Ok(self.dropped.is_empty())
        }
        fn contains_right(&mut self, key: &[Value]) -> Result<bool, SetInterrupted> {
            Ok(self.dropped.iter().any(|row| row == key))
        }
        fn first_time(&mut self, _: &[Value]) -> Result<bool, SetInterrupted> {
            panic!("bag-preserving antijoin")
        }
        fn emit(&mut self, row: &[Value]) -> Result<(), SetInterrupted> {
            self.emitted.push(row.to_vec());
            Ok(())
        }
    }
    assert_eq!(dropped, vec![vec![source[0][1], source[0][1]]]);
    let mut anti = Anti {
        dropped,
        emitted: Vec::new(),
    };
    for row in &source {
        OperatorsUseCaseImpl::new()
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

use ryo_values::cases::{Case, CaseName, Expect, Word, run};
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
