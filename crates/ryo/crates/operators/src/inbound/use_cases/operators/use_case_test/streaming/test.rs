//! Public streaming and finishing calls borrow engine-owned reduction state and keep fault order; the metadata checks are in
//! `aggregation_checks`.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::errors::ContractError;
use crate::domain::contract::types::FirstOccurrence;
use crate::domain::contract::types::*;
use crate::domain::reduction::types::*;
use crate::inbound::use_cases::operators::types::*;
use crate::inbound::use_cases::operators::{fixtures, interface::OperatorsUseCase};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::WholeNumber;
use ryo_values::domain::rows::types::{RowCount, RowIndex};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    StreamCount,
    StreamDigest,
    StreamEarlyFault,
    StreamStorageFailure,
    FinishDigest,
}
#[derive(Debug, PartialEq)]
enum Observation {
    Groups(Cow<'static, [(usize, usize)]>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("stream_count_and_finish_keep_original_representatives"),
        word: Word::Succeeds,
        input: || Scenario::StreamCount,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2), (1, 1)]))),
    },
    Case {
        name: CaseName("stream_digest_and_finish_count_distinct_resolved_values"),
        word: Word::Succeeds,
        input: || Scenario::StreamDigest,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2), (1, 1)]))),
    },
    Case {
        name: CaseName("stream_bad_early_row_prevents_later_upstream_demand"),
        word: Word::Refuses,
        input: || Scenario::StreamEarlyFault,
        expect: Expect::Error(Failure::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(1),
        }),
    },
    Case {
        name: CaseName("stream_group_storage_refusal_prevents_value_access"),
        word: Word::Fails,
        input: || Scenario::StreamStorageFailure,
        expect: Expect::Error(Failure::Interrupted),
    },
    Case {
        name: CaseName("finish_digest_sorts_deduplicates_and_keeps_group_coordinates"),
        word: Word::Succeeds,
        input: || Scenario::FinishDigest,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2), (1, 1)]))),
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
#[derive(Default)]
struct Groups {
    rows: Vec<(RowIndex, RowCount, i64)>,
    clears: usize,
}
impl GroupOutput for Groups {
    fn clear(&mut self) {
        self.clears += 1;
        self.rows.clear();
    }
    fn push(&mut self, first: RowIndex, counted: RowCount, digest: GroupDigest) {
        self.rows.push((first, counted, digest.0));
    }
    fn sort_first(&mut self) {
        self.rows.sort_unstable_by_key(|group| group.0);
    }
}
fn observation(groups: Groups) -> Observation {
    Observation::Groups(Cow::Owned(
        groups
            .rows
            .into_iter()
            .map(|(first, count, _)| (first.0, count.0))
            .collect(),
    ))
}
#[derive(Default)]
struct State {
    keys: Vec<Vec<Cell>>,
    first_rows: Vec<Vec<Cell>>,
    counts: Vec<RowCount>,
    values: std::collections::HashSet<(usize, Cell)>,
    hashes: Vec<AggregationHash>,
    group_calls: usize,
    refuses_group: bool,
}
impl StreamingReduction for State {
    fn group(&mut self, key: &[Cell], row: &[Cell]) -> Result<RowIndex, ContractError> {
        self.group_calls += 1;
        if self.refuses_group {
            return Err(ContractError::Interrupted);
        }
        let group = if let Some(group) = self.keys.iter().position(|stored| stored == key) {
            group
        } else {
            let group = self.keys.len();
            self.keys.push(key.to_vec());
            self.first_rows.push(row.to_vec());
            self.counts.push(RowCount(0));
            group
        };
        Ok(RowIndex(group))
    }
    fn insert_value(&mut self, group: RowIndex, value: Cell) -> FirstOccurrence {
        if self.values.insert((group.0, value)) {
            FirstOccurrence::First
        } else {
            FirstOccurrence::Repeat
        }
    }
    fn count(&mut self, group: RowIndex) -> Option<&mut RowCount> {
        self.counts.get_mut(group.0)
    }
    fn datum(&self, value: Cell) -> DigestDatum<'_> {
        let Cell::Number(Integer(value)) = value else {
            panic!("numeric fixture")
        };
        DigestDatum::Number(WholeNumber(value))
    }
    fn append_hash(&mut self, hash: AggregationHash) {
        self.hashes.push(hash);
    }
}
fn stream(input: Input) -> Result<Value, Failure> {
    let digest = matches!(
        input,
        Scenario::StreamDigest | Scenario::StreamEarlyFault | Scenario::StreamStorageFailure
    );
    let plan = if digest {
        AggregationPlan::Digest {
            keys: &[ColumnIndex(1)],
            over: ColumnIndex(0),
        }
    } else {
        AggregationPlan::Count {
            keys: &[ColumnIndex(1)],
            distinct: Some(ColumnIndex(0)),
        }
    };
    if matches!(
        input,
        Scenario::StreamEarlyFault | Scenario::StreamStorageFailure
    ) {
        // A complete metadata schema does not turn a malformed first row into a complete row.
        let plan = AggregationPlan::Digest {
            keys: &[ColumnIndex(0)],
            over: ColumnIndex(1),
        };
        let mut state = State {
            refuses_group: matches!(input, Scenario::StreamStorageFailure),
            ..State::default()
        };
        let rows: &[&[Cell]] = &[
            &[Cell::Number(Integer(1))],
            &[Cell::Number(Integer(1)), Cell::Number(Integer(2))],
        ];
        let mut demanded = 0;
        let mut result = Ok(());
        for row in rows {
            demanded += 1;
            result = written(OperatorCall::StreamAggregation {
                program: plan,
                row,
                key: &mut Vec::new(),
                driver: &mut state,
            });
            if result.is_err() {
                break;
            }
        }
        assert_eq!(
            demanded, 1,
            "the first fault is returned before later upstream work is requested"
        );
        assert_eq!(state.group_calls, 1);
        assert!(state.hashes.is_empty(), "the bad value is never hashed");
        if matches!(input, Scenario::StreamStorageFailure) {
            assert!(state.first_rows.is_empty());
        }
        result?;
        panic!("the malformed first row or refused storage must fail");
    }
    let mut state = State::default();
    let mut key = Vec::with_capacity(4);
    let original = key.as_ptr();
    for row in [[9, 2], [9, 2], [8, 1], [7, 2]] {
        written(OperatorCall::StreamAggregation {
            program: plan,
            row: &row.map(number),
            key: &mut key,
            driver: &mut state,
        })?;
    }
    assert_eq!(
        key.as_ptr(),
        original,
        "the semantic call borrows the engine's key scratch"
    );
    assert_eq!(
        state.first_rows,
        vec![vec![number(9), number(2)], vec![number(8), number(1)]],
        "original columns survive representative retention"
    );
    let mut output = Groups::default();
    let counts: Vec<usize> = state.counts.iter().map(|count| count.0).collect();
    written(OperatorCall::FinishAggregation {
        counts: GroupCounts(&counts),
        hashes: &mut state.hashes,
        digest: if digest {
            Finishing::Digested
        } else {
            Finishing::Counted
        },
        target: &mut output,
    })?;
    Ok(observation(output))
}
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::StreamCount
        | Scenario::StreamDigest
        | Scenario::StreamEarlyFault
        | Scenario::StreamStorageFailure => stream(input),
        Scenario::FinishDigest => {
            let mut hashes = vec![
                AggregationHash {
                    group: GroupCoordinate(1),
                    hash: ValueHash(9),
                },
                AggregationHash {
                    group: GroupCoordinate(0),
                    hash: ValueHash(5),
                },
                AggregationHash {
                    group: GroupCoordinate(0),
                    hash: ValueHash(3),
                },
                AggregationHash {
                    group: GroupCoordinate(0),
                    hash: ValueHash(5),
                },
            ];
            let mut output = Groups::default();
            written(OperatorCall::FinishAggregation {
                counts: GroupCounts(&[0, 0]),
                hashes: &mut hashes,
                digest: Finishing::Digested,
                target: &mut output,
            })?;
            assert_eq!(
                hashes,
                vec![
                    AggregationHash {
                        group: GroupCoordinate(0),
                        hash: ValueHash(3)
                    },
                    AggregationHash {
                        group: GroupCoordinate(0),
                        hash: ValueHash(5)
                    },
                    AggregationHash {
                        group: GroupCoordinate(1),
                        hash: ValueHash(9)
                    }
                ]
            );
            let mut reordered = vec![
                AggregationHash {
                    group: GroupCoordinate(0),
                    hash: ValueHash(5),
                },
                AggregationHash {
                    group: GroupCoordinate(1),
                    hash: ValueHash(9),
                },
                AggregationHash {
                    group: GroupCoordinate(0),
                    hash: ValueHash(3),
                },
            ];
            let mut repeated = Groups::default();
            written(OperatorCall::FinishAggregation {
                counts: GroupCounts(&[0, 0]),
                hashes: &mut reordered,
                digest: Finishing::Digested,
                target: &mut repeated,
            })?;
            assert_eq!(
                output.rows, repeated.rows,
                "finish is independent of hash arrival order and repetition"
            );
            Ok(observation(output))
        }
    }
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
