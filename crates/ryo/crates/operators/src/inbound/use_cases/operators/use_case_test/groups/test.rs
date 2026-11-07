//! Public grouping requests borrow fixture-owned storage; fixtures implement storage, never reductions.
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
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    CountGroups,
    DistinctGroups,
    CountOperandFirst,
    DigestGroups,
    HashValue,
}
#[derive(Debug, PartialEq)]
enum Observation {
    Groups(Cow<'static, [(usize, usize)]>),
    Hashes {
        same_text: bool,
        different_numbers: bool,
    },
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("count_reduction_dispatch_restores_first_occurrence_order"),
        word: Word::Succeeds,
        input: || Scenario::CountGroups,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 3), (1, 1)]))),
    },
    Case {
        name: CaseName("distinct_count_reduction_does_not_count_repeats"),
        word: Word::Succeeds,
        input: || Scenario::DistinctGroups,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2), (1, 1)]))),
    },
    Case {
        name: CaseName("collected_count_operand_fault_precedes_key_fault"),
        word: Word::Refuses,
        input: || Scenario::CountOperandFirst,
        expect: Expect::Error(Failure::NoSuchColumn { index: 98 }),
    },
    Case {
        name: CaseName("digest_dispatch_uses_resolved_text_not_handles_repeats_or_order"),
        word: Word::Succeeds,
        input: || Scenario::DigestGroups,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2)]))),
    },
    Case {
        name: CaseName("hash_dispatch_is_stable_for_equal_borrowed_text"),
        word: Word::Succeeds,
        input: || Scenario::HashValue,
        expect: Expect::Value(Observation::Hashes {
            same_text: true,
            different_numbers: true,
        }),
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
struct Resolved {
    renamed: bool,
}
impl DigestValues for Resolved {
    fn datum(&self, value: Cell) -> DigestDatum<'_> {
        let Cell::Number(Integer(id)) = value else {
            panic!("fixture handles are numeric")
        };
        DigestDatum::Text(match (self.renamed, id) {
            (false, 7) | (true, 70) => "same",
            (false, 8) | (true, 80) => "other",
            _ => "changed",
        })
    }
}
fn digest(rows: &[[i64; 2]], renamed: bool) -> Result<Groups, Failure> {
    let table = table(rows);
    let order: Vec<_> = (0..rows.len()).map(RowIndex).collect();
    let mut output = Groups::default();
    written(OperatorCall::ReduceDigest {
        table: &table,
        keys: &[ColumnIndex(0)],
        over: ColumnIndex(1),
        order: &order,
        hashes: &mut Vec::new(),
        hasher: &Resolved { renamed },
        target: &mut output,
    })?;
    Ok(output)
}
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::CountGroups | Scenario::DistinctGroups | Scenario::CountOperandFirst => {
            let table = table(&[[2, 7], [1, 7], [2, 7], [2, 8]]);
            let mut output = Groups::default();
            let (keys, distinct): (&[ColumnIndex], _) =
                if matches!(input, Scenario::CountOperandFirst) {
                    (&[ColumnIndex(99)], Some(ColumnIndex(98)))
                } else {
                    (
                        &[ColumnIndex(0)],
                        if matches!(input, Scenario::DistinctGroups) {
                            Some(ColumnIndex(1))
                        } else {
                            None
                        },
                    )
                };
            let mut values = Vec::with_capacity(8);
            let original = values.as_ptr();
            let result = written(OperatorCall::ReduceCount {
                table: &table,
                keys: GroupColumns::Columns(keys),
                distinct,
                order: &[RowIndex(1), RowIndex(0), RowIndex(2), RowIndex(3)],
                values: &mut values,
                target: &mut output,
            });
            if matches!(input, Scenario::CountOperandFirst) {
                assert_eq!(
                    output.clears, 0,
                    "metadata failure precedes output mutation"
                );
            }
            result?;
            assert_eq!(values.as_ptr(), original);
            assert!(output.rows.iter().all(|group| group.2 == 0));
            Ok(observation(output))
        }
        Scenario::DigestGroups => {
            let original = digest(&[[1, 7], [1, 8], [1, 7]], false)?;
            let renamed = digest(&[[1, 80], [1, 70]], true)?;
            let changed = digest(&[[1, 7], [1, 9]], false)?;
            assert_eq!(
                original.rows, renamed.rows,
                "same resolved text set survives changed handles, row order and duplicate count"
            );
            assert_ne!(
                original.rows[0].2, changed.rows[0].2,
                "changing resolved text invalidates this digest"
            );
            Ok(observation(original))
        }
        Scenario::HashValue => {
            let same = String::from("same");
            let literal = fixtures::operators().operators(OperatorCall::HashValue {
                datum: DigestDatum::Text("same"),
            })?;
            let borrowed = fixtures::operators().operators(OperatorCall::HashValue {
                datum: DigestDatum::Text(&same),
            })?;
            let first = fixtures::operators().operators(OperatorCall::HashValue {
                datum: DigestDatum::Number(WholeNumber(7)),
            })?;
            let second = fixtures::operators().operators(OperatorCall::HashValue {
                datum: DigestDatum::Number(WholeNumber(8)),
            })?;
            assert!(matches!(literal, OperatorAnswer::ValueHash(_)));
            assert!(matches!(first, OperatorAnswer::ValueHash(_)));
            assert!(matches!(second, OperatorAnswer::ValueHash(_)));
            Ok(Observation::Hashes {
                same_text: literal == borrowed,
                different_numbers: first != second,
            })
        }
    }
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
