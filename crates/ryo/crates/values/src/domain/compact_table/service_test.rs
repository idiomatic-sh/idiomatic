//! Exercise the owning Values codec directly: the decoder differential, truncation and refusal rollback.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::compact_table::service as compact;
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::rows::types::RowIndex;
use crate::domain::rows::types::{RowCount, Table};
use crate::domain::stored_facts::types::{KeptWords, PackedWords};
use crate::domain::table_words::{service, types};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, Name, NamePlace, Offset, Origin, PlaceNumber, TextHash,
    Value as Cell, Words, WorkerNumber,
};
use std::sync::Arc;
use types::{Admitted, CellCount, DecodeMode, WordSlice};
pub(super) fn place(at: u32) -> NamePlace {
    NamePlace {
        origin: Origin::Derived(WorkerNumber(0)),
        file: FileId(0),
        start: Offset(at),
        end: Offset(at + 1),
    }
}
pub(super) fn table() -> Table {
    Table {
        kinds: vec![CellKind::Number, CellKind::Symbol, CellKind::Number],
        count: RowCount(2),
        cells: vec![
            Cell::Number(Integer(9)),
            Cell::Name(Name {
                hash: TextHash(17),
                place: PlaceNumber(0),
            }),
            Cell::Number(Integer(9)),
            Cell::Number(Integer(-7)),
            Cell::Name(Name {
                hash: TextHash(19),
                place: PlaceNumber(1),
            }),
            Cell::Number(Integer(-7)),
        ],
    }
}
pub(super) fn packed(frames: &[Table]) -> Arc<KeptWords> {
    let mut words = Words::default();
    for frame in frames {
        service::encode_table(frame, &[place(1), place(2)], &mut words);
    }
    Arc::new(KeptWords(service::pack(WordSlice(&words.0))))
}
pub(super) fn same(actual: &types::CompactTable, expected: &Table) {
    assert_eq!(compact::compact_kinds(actual), expected.kinds);
    assert_eq!(compact::compact_rows(actual), expected.count);
    assert_eq!(actual.cells, CellCount(expected.cells.len()));
    let width = expected.kinds.len();
    let mut scratch = vec![Cell::Number(Integer(0)); width];
    for row in 0..expected.count.0 {
        let start = row * width;
        assert_eq!(
            compact::compact_read(actual, RowIndex(row), &mut scratch),
            expected.cells.get(start..start + width)
        );
        for column in 0..width {
            assert_eq!(
                compact::compact_cell(actual, RowIndex(row), ColumnIndex(column as u32)),
                expected.cells.get(start + column).copied()
            );
        }
    }
    assert!(compact::compact_read(actual, RowIndex(expected.count.0), &mut scratch).is_none());
}
pub(super) fn differential(input: Arc<KeptWords>, mode: DecodeMode) {
    let seed = vec![place(99)];
    let mut old_places = seed.clone();
    let mut new_places = seed;
    let old = match mode {
        DecodeMode::Permissive => Ok(service::decode_combined(&input.0, &mut old_places)),
        DecodeMode::Complete => service::decode_complete(&input.0, &mut old_places),
    };
    let new = compact::admit_compact(
        crate::domain::table_words::types::CompactBacking(input),
        &mut new_places,
        mode,
    );
    assert_eq!(old.as_ref().err(), new.as_ref().err());
    assert_eq!(old_places, new_places);
    if let (Ok(old), Ok(Admitted { table: meta, .. })) = (old, new) {
        assert_eq!(compact::compact_kinds(&meta), old.kinds);
        assert_eq!(compact::compact_rows(&meta), old.count);
        assert_eq!(meta.cells, CellCount(old.cells.len()));
        same(&meta, &old);
        let expected_rectangular = old
            .count
            .0
            .checked_mul(old.kinds.len())
            .is_some_and(|cells| cells == old.cells.len());
        assert_eq!(
            compact::compact_rectangular(&meta)
                == crate::domain::table_words::types::TableShape::Rectangular,
            expected_rectangular
        );
        if expected_rectangular {
            let mut scratch = vec![Cell::Number(Integer(0)); old.kinds.len()];
            for row in 0..old.count.0 {
                let expected = &old.cells[row * old.kinds.len()..(row + 1) * old.kinds.len()];
                assert_eq!(
                    compact::compact_read(&meta, RowIndex(row), &mut scratch),
                    Some(expected)
                );
                for (column, value) in expected.iter().enumerate() {
                    let actual =
                        compact::compact_cell(&meta, RowIndex(row), ColumnIndex(column as u32))
                            .unwrap();
                    assert_eq!(actual, *value);
                    if let (Cell::Name(a), Cell::Name(b)) = (actual, value) {
                        assert_eq!(a.place, b.place);
                    }
                }
            }
            assert!(compact::compact_read(&meta, RowIndex(old.count.0), &mut scratch).is_none());
            assert!(
                compact::compact_cell(&meta, RowIndex(0), ColumnIndex(old.kinds.len() as u32))
                    .is_none()
            );
        }
        let before = new_places.clone();
        same(&meta, &old);
        assert_eq!(new_places, before, "compact reads never re-admit names");
    }
}
fn multiple_frames_repeated_columns_and_name_rebase_match_production_decoder() {
    let frames = [table(), table()];
    for mode in [DecodeMode::Permissive, DecodeMode::Complete] {
        differential(packed(&frames), mode);
    }
    let mut places = vec![];
    let Admitted { table: meta, names } = compact::admit_compact(
        crate::domain::table_words::types::CompactBacking(packed(&frames)),
        &mut places,
        DecodeMode::Complete,
    )
    .unwrap();
    assert_eq!(compact::compact_rows(&meta), RowCount(4));
    assert_eq!(meta.cells, CellCount(12));
    assert_eq!(
        compact::compact_rectangular(&meta),
        crate::domain::table_words::types::TableShape::Rectangular
    );
    assert_eq!(names.len(), 4);
    assert_eq!(places.len(), 4);
    assert_eq!(
        compact::compact_cell(&meta, RowIndex(2), ColumnIndex(1)),
        Some(Cell::Name(Name {
            hash: TextHash(17),
            place: PlaceNumber(2)
        }))
    );
}
fn truncated_tail_rolls_back_that_frame_or_complete_input() {
    let first = packed(&[table()]);
    let second = packed(&[table()]);
    let mut bytes = first.0.0.clone();
    bytes.extend_from_slice(&second.0.0[..second.0.0.len() - 1]);
    for mode in [DecodeMode::Permissive, DecodeMode::Complete] {
        differential(Arc::new(KeptWords(PackedWords(bytes.clone()))), mode);
    }
    let mut places = vec![];
    let Admitted { table: meta, .. } = compact::admit_compact(
        crate::domain::table_words::types::CompactBacking(Arc::new(KeptWords(PackedWords(bytes)))),
        &mut places,
        DecodeMode::Permissive,
    )
    .unwrap();
    assert_eq!(compact::compact_rows(&meta), RowCount(2));
    assert_eq!(places.len(), 2);
}
fn noncanonical_cut_and_incompatible_schema_preserve_modes() {
    for bytes in [vec![0x80], vec![0x80, 0x00], vec![0xff; 12]] {
        for mode in [DecodeMode::Permissive, DecodeMode::Complete] {
            differential(Arc::new(KeptWords(PackedWords(bytes.clone()))), mode);
        }
    }
    let other = Table {
        kinds: vec![CellKind::File],
        count: RowCount(1),
        cells: vec![Cell::File(FileId(3))],
    };
    for mode in [DecodeMode::Permissive, DecodeMode::Complete] {
        differential(packed(&[table(), other.clone()]), mode);
    }
}
fn declared_cell_overflow_does_not_admit_current_frame_names() {
    let first = packed(&[table()]);
    let mut words = Words(vec![2, u64::MAX]);
    ryo_language::domain::words::service::encode_kind(CellKind::Number, &mut words);
    ryo_language::domain::words::service::encode_kind(CellKind::Number, &mut words);
    ryo_language::domain::words::service::encode_places(&[place(8)], &mut words);
    let mut bytes = first.0.0.clone();
    let suffix = service::pack(WordSlice(&words.0));
    bytes.extend_from_slice(&suffix.0);
    for mode in [DecodeMode::Permissive, DecodeMode::Complete] {
        differential(Arc::new(KeptWords(PackedWords(bytes.clone()))), mode);
    }
}
fn empty_bucket_schema_is_metadata_not_unloaded_rows() {
    let mut places = vec![];
    let Admitted { table: meta, .. } = compact::admit_compact(
        crate::domain::table_words::types::CompactBacking(Arc::new(KeptWords(
            PackedWords::default(),
        ))),
        &mut places,
        DecodeMode::Complete,
    )
    .unwrap();
    assert_eq!(compact::compact_rows(&meta), RowCount(0));
    assert_eq!(meta.cells, CellCount(0));
    assert!(compact::compact_kinds(&meta).is_empty());
    assert!(compact::compact_read(&meta, RowIndex(0), &mut [Cell::Number(Integer(0))]).is_none());
}
type Input = fn() -> Result<(), Failure>;
type Value = ();
type Failure = crate::domain::table_words::errors::TableWordsError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("multiple_frames_repeated_columns_and_name_rebase_match_production_decoder"),
        word: Word::Succeeds,
        input: || {
            || {
                multiple_frames_repeated_columns_and_name_rebase_match_production_decoder();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("truncated_tail_rolls_back_that_frame_or_complete_input"),
        word: Word::Succeeds,
        input: || {
            || {
                truncated_tail_rolls_back_that_frame_or_complete_input();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("noncanonical_cut_and_incompatible_schema_preserve_modes"),
        word: Word::Succeeds,
        input: || {
            || {
                noncanonical_cut_and_incompatible_schema_preserve_modes();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("declared_cell_overflow_does_not_admit_current_frame_names"),
        word: Word::Succeeds,
        input: || {
            || {
                declared_cell_overflow_does_not_admit_current_frame_names();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty_bucket_schema_is_metadata_not_unloaded_rows"),
        word: Word::Succeeds,
        input: || {
            || {
                empty_bucket_schema_is_metadata_not_unloaded_rows();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("incomplete_input_refuses_without_publishing_places"),
        word: Word::Refuses,
        input: || incomplete_input,
        expect: Expect::Refusal(|error| crate::selects!(matches!(error, Failure::Incomplete))),
    },
];
fn subject(check: Input) -> Result<Value, Failure> {
    check()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn incomplete_input() -> Result<(), Failure> {
    let before = vec![place(99)];
    let mut places = before.clone();
    let result = compact::admit_compact(
        crate::domain::table_words::types::CompactBacking(Arc::new(KeptWords(PackedWords(vec![
            0x80,
        ])))),
        &mut places,
        DecodeMode::Complete,
    )
    .map(|_| ());
    assert_eq!(places, before);
    result
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod access_test;
