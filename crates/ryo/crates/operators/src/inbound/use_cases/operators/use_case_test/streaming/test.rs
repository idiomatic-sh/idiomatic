//! Public streaming and metadata calls borrow engine-owned reduction state and keep fault order.
use crate::inbound::use_cases::operators::{fixtures, interface::OperatorsUseCase};
use crate::domain::capture::types::*;
use crate::domain::contract::types::*;
use crate::domain::equality_join::types::*;
use crate::domain::ordered_existence::types::*;
use crate::domain::ordered_match::types::*;
use crate::domain::reduction::types::*;
use crate::domain::set_rows::types::*;
use crate::domain::source_matches::types::*;
use crate::domain::words::types::*;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::WholeNumber;
use ryo_values::domain::rows::types::{RowCount, RowIndex};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    CheckCount,
    CheckDigest,
    CheckStreamCountOrder,
    CheckCollectedCountOrder,
    CheckStreamDigestOrder,
    StreamCount,
    StreamDigest,
    StreamEarlyFault,
    StreamStorageFailure,
    FinishDigest,
}
#[derive(Debug, PartialEq)]
enum Observation {
    Groups(Cow<'static, [(usize, usize)]>),
    Kinds(Cow<'static, [CellKind]>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("count_metadata_keeps_repeated_key_kinds"),
        word: Word::Succeeds,
        input: || Scenario::CheckCount,
        expect: Expect::Value(Observation::Kinds(Cow::Borrowed(&[
            CellKind::Number,
            CellKind::Number,
        ]))),
    },
    Case {
        name: CaseName("digest_metadata_keeps_key_kinds"),
        word: Word::Succeeds,
        input: || Scenario::CheckDigest,
        expect: Expect::Value(Observation::Kinds(Cow::Borrowed(&[CellKind::Number]))),
    },
    Case {
        name: CaseName("streamed_count_metadata_checks_keys_first"),
        word: Word::Refuses,
        input: || Scenario::CheckStreamCountOrder,
        expect: Expect::Error(Failure::NoSuchColumn { index: 99 }),
    },
    Case {
        name: CaseName("collected_count_metadata_checks_operand_first"),
        word: Word::Refuses,
        input: || Scenario::CheckCollectedCountOrder,
        expect: Expect::Error(Failure::NoSuchColumn { index: 98 }),
    },
    Case {
        name: CaseName("streamed_digest_metadata_checks_operand_first"),
        word: Word::Refuses,
        input: || Scenario::CheckStreamDigestOrder,
        expect: Expect::Error(Failure::NoSuchColumn { index: 98 }),
    },
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
        expect: Expect::Error(Failure::NoSuchColumn { index: 1 }),
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
    fn push(&mut self, first: RowIndex, counted: RowCount, digest: i64) {
        self.rows.push((first, counted, digest));
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
    counts: Vec<usize>,
    values: std::collections::HashSet<(usize, Cell)>,
    hashes: Vec<AggregationHash>,
    group_calls: usize,
    refuses_group: bool,
}
impl StreamingReduction for State {
    fn group(&mut self, key: &[Cell], row: &[Cell]) -> Result<RowIndex, SemanticInterrupted> {
        self.group_calls += 1;
        if self.refuses_group {
            return Err(SemanticInterrupted);
        }
        let group = if let Some(group) = self.keys.iter().position(|stored| stored == key) {
            group
        } else {
            let group = self.keys.len();
            self.keys.push(key.to_vec());
            self.first_rows.push(row.to_vec());
            self.counts.push(0);
            group
        };
        Ok(RowIndex(group))
    }
    fn insert_value(&mut self, group: RowIndex, value: Cell) -> bool {
        self.values.insert((group.0, value))
    }
    fn count(&mut self, group: RowIndex) -> Option<&mut usize> {
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
    written(OperatorCall::FinishAggregation {
        counts: &state.counts,
        hashes: &mut state.hashes,
        digest,
        target: &mut output,
    })?;
    Ok(observation(output))
}
fn check(input: Input) -> Result<Value, Failure> {
    let mut kinds = Vec::with_capacity(4);
    let original = kinds.as_ptr();
    let (program, order) = match input {
        Scenario::CheckCount => (
            AggregationPlan::Count {
                keys: &[ColumnIndex(1), ColumnIndex(1)],
                distinct: Some(ColumnIndex(0)),
            },
            AggregationCheckOrder::Streaming,
        ),
        Scenario::CheckDigest => (
            AggregationPlan::Digest {
                keys: &[ColumnIndex(1)],
                over: ColumnIndex(0),
            },
            AggregationCheckOrder::Streaming,
        ),
        Scenario::CheckStreamDigestOrder => (
            AggregationPlan::Digest {
                keys: &[ColumnIndex(99)],
                over: ColumnIndex(98),
            },
            AggregationCheckOrder::Streaming,
        ),
        Scenario::CheckCollectedCountOrder => (
            AggregationPlan::Count {
                keys: &[ColumnIndex(99)],
                distinct: Some(ColumnIndex(98)),
            },
            AggregationCheckOrder::Collected,
        ),
        _ => (
            AggregationPlan::Count {
                keys: &[ColumnIndex(99)],
                distinct: Some(ColumnIndex(98)),
            },
            AggregationCheckOrder::Streaming,
        ),
    };
    written(OperatorCall::CheckAggregation {
        program,
        kinds: &[CellKind::Number; 2],
        order,
        target: &mut kinds,
    })?;
    assert_eq!(kinds.as_ptr(), original);
    Ok(Observation::Kinds(Cow::Owned(kinds)))
}
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::CheckCount
        | Scenario::CheckDigest
        | Scenario::CheckStreamCountOrder
        | Scenario::CheckCollectedCountOrder
        | Scenario::CheckStreamDigestOrder => check(input),
        Scenario::StreamCount
        | Scenario::StreamDigest
        | Scenario::StreamEarlyFault
        | Scenario::StreamStorageFailure => stream(input),
        Scenario::FinishDigest => {
            let mut hashes = vec![
                AggregationHash { group: 1, hash: 9 },
                AggregationHash { group: 0, hash: 5 },
                AggregationHash { group: 0, hash: 3 },
                AggregationHash { group: 0, hash: 5 },
            ];
            let mut output = Groups::default();
            written(OperatorCall::FinishAggregation {
                counts: &[0, 0],
                hashes: &mut hashes,
                digest: true,
                target: &mut output,
            })?;
            assert_eq!(
                hashes,
                vec![
                    AggregationHash { group: 0, hash: 3 },
                    AggregationHash { group: 0, hash: 5 },
                    AggregationHash { group: 1, hash: 9 }
                ]
            );
            let mut reordered = vec![
                AggregationHash { group: 0, hash: 5 },
                AggregationHash { group: 1, hash: 9 },
                AggregationHash { group: 0, hash: 3 },
            ];
            let mut repeated = Groups::default();
            written(OperatorCall::FinishAggregation {
                counts: &[0, 0],
                hashes: &mut reordered,
                digest: true,
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
