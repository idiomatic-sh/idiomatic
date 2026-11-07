//! Reduction invariants use explicit independent group descriptors and competing column faults.
use super::service as reduction;
use crate::domain::contract::types::{PortableOperatorFault};
use crate::domain::contract::errors::ContractError;
use crate::domain::reduction::types::{
    AggregationCheckOrder, AggregationHash, AggregationPlan, DigestDatum, DigestValues, Finishing,
    GroupColumns, GroupCoordinate, GroupCounts, GroupDigest, GroupOutput, HashScratch,
    StreamingReduction, ValueHash,
};
use crate::domain::set_rows::types::Occurrence;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, WholeNumber},
    rows::types::{RowCount, RowIndex, Table},
};
#[derive(Clone, Copy)]
enum Scenario {
    Count,
    Distinct,
    StreamDistinct,
    StreamEarlyRow,
    StreamCountOrder,
    CollectedCountOrder,
    StreamDigestOrder,
    DigestIdentity,
}
type Input = Scenario;
type Value = std::borrow::Cow<'static, [i64]>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("groups_keep_first_occurrence_order"),
        word: Word::Succeeds,
        input: || Scenario::Count,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0, 3, 0, 1, 1, 0])),
    },
    Case {
        name: CaseName("distinct_values_count_once_per_group"),
        word: Word::Succeeds,
        input: || Scenario::Distinct,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0, 2, 0, 1, 1, 0])),
    },
    Case {
        name: CaseName("streaming_distinct_retains_original_first_rows"),
        word: Word::Succeeds,
        input: || Scenario::StreamDistinct,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[2, 7, 1, 7, 0, 2, 0, 1, 1, 0])),
    },
    Case {
        name: CaseName("bad_early_row_refuses_before_later_upstream_work"),
        word: Word::Refuses,
        input: || Scenario::StreamEarlyRow,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: ryo_values::domain::plan_parts::types::ColumnIndex(1) }),
    },
    Case {
        name: CaseName("streamed_count_checks_keys_before_distinct"),
        word: Word::Refuses,
        input: || Scenario::StreamCountOrder,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: ryo_values::domain::plan_parts::types::ColumnIndex(99) }),
    },
    Case {
        name: CaseName("collected_count_checks_distinct_before_keys"),
        word: Word::Refuses,
        input: || Scenario::CollectedCountOrder,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: ryo_values::domain::plan_parts::types::ColumnIndex(98) }),
    },
    Case {
        name: CaseName("streamed_digest_checks_operand_before_keys"),
        word: Word::Refuses,
        input: || Scenario::StreamDigestOrder,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: ryo_values::domain::plan_parts::types::ColumnIndex(98) }),
    },
    Case {
        name: CaseName("digest_depends_on_resolved_text_set_and_not_order_repeats_or_handles"),
        word: Word::Succeeds,
        input: || Scenario::DigestIdentity,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1, 2, 1])),
    },
];
#[derive(Default)]
struct Output(Vec<(RowIndex, RowCount, i64)>);
impl GroupOutput for Output {
    fn clear(&mut self) {
        self.0.clear();
    }
    fn push(&mut self, first: RowIndex, counted: RowCount, digest: GroupDigest) {
        self.0.push((first, counted, digest.0));
    }
    fn sort_first(&mut self) {
        self.0.sort_unstable_by_key(|group| group.0);
    }
}
fn values(groups: &Output) -> Vec<i64> {
    groups
        .0
        .iter()
        .flat_map(|(first, count, digest)| {
            [
                i64::try_from(first.0).unwrap(),
                i64::try_from(count.0).unwrap(),
                *digest,
            ]
        })
        .collect()
}
fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
fn table(rows: &[[i64; 2]]) -> Table {
    Table {
        kinds: vec![CellKind::Number; 2],
        cells: rows
            .iter()
            .flat_map(|row| row.iter().copied().map(number))
            .collect(),
        count: RowCount(rows.len()),
    }
}
#[derive(Default)]
struct State {
    first: Vec<Vec<Cell>>,
    keys: Vec<Vec<Cell>>,
    counts: Vec<RowCount>,
    pairs: std::collections::HashSet<(usize, Cell)>,
    hashes: Vec<AggregationHash>,
    group_calls: usize,
}
impl StreamingReduction for State {
    fn group(&mut self, key: &[Cell], row: &[Cell]) -> Result<RowIndex, ContractError> {
        self.group_calls += 1;
        let at = if let Some(at) = self.keys.iter().position(|each| each == key) {
            at
        } else {
            let at = self.keys.len();
            self.keys.push(key.to_vec());
            self.first.push(row.to_vec());
            self.counts.push(RowCount(0));
            at
        };
        Ok(RowIndex(at))
    }
    fn insert_value(&mut self, group: RowIndex, value: Cell) -> Occurrence {
        Occurrence::from(self.pairs.insert((group.0, value)))
    }
    fn count(&mut self, group: RowIndex) -> Option<&mut RowCount> {
        self.counts.get_mut(group.0)
    }
    fn datum(&self, value: Cell) -> DigestDatum<'_> {
        let Cell::Number(Integer(number)) = value else {
            panic!("numeric fixture")
        };
        DigestDatum::Number(WholeNumber(number))
    }
    fn append_hash(&mut self, hash: AggregationHash) {
        self.hashes.push(hash);
    }
}
struct Resolved {
    renamed: bool,
}
impl DigestValues for Resolved {
    fn datum(&self, value: Cell) -> DigestDatum<'_> {
        let Cell::Number(Integer(id)) = value else {
            panic!("numeric handle fixture")
        };
        let text = match (self.renamed, id) {
            (false, 7) | (true, 70) => "same",
            (false, 8) | (true, 80) => "other",
            _ => "changed",
        };
        DigestDatum::Text(text)
    }
}
fn digest(table: &Table, resolved: &Resolved) -> Output {
    let mut output = Output::default();
    let mut hashes = Vec::new();
    let order: Vec<_> = (0..table.count.0).map(RowIndex).collect();
    reduction::digest_groups(
        table,
        &[ColumnIndex(0)],
        ColumnIndex(1),
        &order,
        HashScratch(&mut hashes),
        resolved,
        &mut output,
    )
    .unwrap();
    output
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut output = Output::default();
    match scenario {
        Scenario::Count | Scenario::Distinct => {
            let table = table(&[[2, 7], [1, 7], [2, 7], [2, 8]]);
            reduction::count_groups(
                &table,
                GroupColumns::Columns(&[ColumnIndex(0)]),
                if matches!(scenario, Scenario::Distinct) {
                    Some(ColumnIndex(1))
                } else {
                    None
                },
                &[RowIndex(1), RowIndex(0), RowIndex(2), RowIndex(3)],
                &mut Vec::new(),
                &mut output,
            )?;
        }
        Scenario::StreamDistinct => {
            let mut state = State::default();
            let program = AggregationPlan::Count {
                keys: &[ColumnIndex(0)],
                distinct: Some(ColumnIndex(1)),
            };
            for row in [[2, 7], [1, 7], [2, 7], [2, 8]] {
                reduction::push(program, &row.map(number), &mut Vec::new(), &mut state)?;
            }
            let counts: Vec<usize> = state.counts.iter().map(|count| count.0).collect();
            reduction::finish(
                GroupCounts(&counts),
                &mut state.hashes,
                Finishing::Counted,
                &mut output,
            );
            let mut result = state
                .first
                .into_iter()
                .flatten()
                .map(|cell| {
                    let Cell::Number(number) = cell else {
                        panic!("numeric fixture")
                    };
                    number.0
                })
                .collect::<Vec<_>>();
            result.extend(values(&output));
            return Ok(std::borrow::Cow::Owned(result));
        }
        Scenario::StreamEarlyRow => {
            let mut state = State::default();
            let result = reduction::push(
                AggregationPlan::Digest {
                    keys: &[ColumnIndex(0)],
                    over: ColumnIndex(1),
                },
                &[number(1)],
                &mut Vec::new(),
                &mut state,
            );
            assert_eq!(state.group_calls, 1);
            assert!(state.hashes.is_empty());
            result?;
        }
        Scenario::StreamCountOrder
        | Scenario::CollectedCountOrder
        | Scenario::StreamDigestOrder => {
            let program = if matches!(scenario, Scenario::StreamDigestOrder) {
                AggregationPlan::Digest {
                    keys: &[ColumnIndex(99)],
                    over: ColumnIndex(98),
                }
            } else {
                AggregationPlan::Count {
                    keys: &[ColumnIndex(99)],
                    distinct: Some(ColumnIndex(98)),
                }
            };
            reduction::check_program(
                program,
                &[CellKind::Number],
                if matches!(scenario, Scenario::CollectedCountOrder) {
                    AggregationCheckOrder::Collected
                } else {
                    AggregationCheckOrder::Streaming
                },
                &mut Vec::new(),
            )?;
        }
        Scenario::DigestIdentity => {
            let original = digest(
                &table(&[[1, 7], [1, 8], [1, 7]]),
                &Resolved { renamed: false },
            );
            let renamed = digest(&table(&[[1, 80], [1, 70]]), &Resolved { renamed: true });
            let changed = digest(&table(&[[1, 7], [1, 9]]), &Resolved { renamed: false });
            return Ok(std::borrow::Cow::Owned(vec![
                i64::from(original.0[0].2 == renamed.0[0].2),
                i64::try_from(original.0[0].1.0).unwrap(),
                i64::from(original.0[0].2 != changed.0[0].2),
            ]));
        }
    }
    Ok(std::borrow::Cow::Owned(values(&output)))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
