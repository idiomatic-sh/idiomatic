//! Exercise the owning Values codec directly, including compact backing and refusal rollback.
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::table_words::{service, types};
use crate::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::{KeptWords, PackedWords},
};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, Name, NamePlace, Offset, Origin, PlaceNumber, TextHash,
    Value as Cell, Words, WorkerNumber,
};
use std::sync::Arc;
use types::{DecodeMode, WordSlice};
fn place(at: u32) -> NamePlace {
    NamePlace {
        origin: Origin::Derived(WorkerNumber(0)),
        file: FileId(0),
        start: Offset(at),
        end: Offset(at + 1),
    }
}
fn table() -> Table {
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
fn packed(frames: &[Table]) -> Arc<KeptWords> {
    let mut words = Words::default();
    for frame in frames {
        service::encode_table(frame, &[place(1), place(2)], &mut words);
    }
    Arc::new(KeptWords(service::pack(WordSlice(&words.0))))
}
fn same(actual: &Table, expected: &Table) {
    assert_eq!(actual.kinds, expected.kinds);
    assert_eq!(actual.count, expected.count);
    assert_eq!(actual.cells, expected.cells);
}
fn differential(input: Arc<KeptWords>, mode: DecodeMode) {
    let seed = vec![place(99)];
    let mut old_places = seed.clone();
    let mut new_places = seed;
    let old = match mode {
        DecodeMode::Permissive => Ok(service::decode_combined(&input.0, &mut old_places)),
        DecodeMode::Complete => service::decode_complete(&input.0, &mut old_places),
    };
    let new = service::admit_compact(input, &mut new_places, mode);
    assert_eq!(old.as_ref().err(), new.as_ref().err());
    assert_eq!(old_places, new_places);
    if let (Ok(old), Ok((meta, _))) = (old, new) {
        assert_eq!(service::compact_kinds(&meta), old.kinds);
        assert_eq!(service::compact_rows(&meta), old.count.0);
        assert_eq!(service::compact_cells(&meta), old.cells.len());
        let actual = service::materialize_compact(&meta).unwrap();
        same(&actual, &old);
        let expected_rectangular = old
            .count
            .0
            .checked_mul(old.kinds.len())
            .is_some_and(|cells| cells == old.cells.len());
        assert_eq!(service::compact_rectangular(&meta), expected_rectangular);
        if expected_rectangular {
            let mut scratch = vec![Cell::Number(Integer(0)); old.kinds.len()];
            for row in 0..old.count.0 {
                let expected = &old.cells[row * old.kinds.len()..(row + 1) * old.kinds.len()];
                assert_eq!(
                    service::compact_read(&meta, row, &mut scratch),
                    Some(expected)
                );
                for (column, value) in expected.iter().enumerate() {
                    let actual = service::compact_cell(&meta, row, column).unwrap();
                    assert_eq!(actual, *value);
                    if let (Cell::Name(a), Cell::Name(b)) = (actual, value) {
                        assert_eq!(a.place, b.place);
                    }
                }
            }
            assert!(service::compact_read(&meta, old.count.0, &mut scratch).is_none());
            assert!(service::compact_cell(&meta, 0, old.kinds.len()).is_none());
        }
        let before = new_places.clone();
        same(&service::materialize_compact(&meta).unwrap(), &old);
        assert_eq!(new_places, before, "materialization never re-admits names");
    }
}
fn multiple_frames_repeated_columns_and_name_rebase_match_production_decoder() {
    let frames = [table(), table()];
    for mode in [DecodeMode::Permissive, DecodeMode::Complete] {
        differential(packed(&frames), mode);
    }
    let mut places = vec![];
    let (meta, names) =
        service::admit_compact(packed(&frames), &mut places, DecodeMode::Complete).unwrap();
    assert_eq!(service::compact_rows(&meta), 4);
    assert_eq!(service::compact_cells(&meta), 12);
    assert!(service::compact_rectangular(&meta));
    assert_eq!(names.len(), 4);
    assert_eq!(places.len(), 4);
    let actual = service::materialize_compact(&meta).unwrap();
    assert_eq!(
        actual.cells[7],
        Cell::Name(Name {
            hash: TextHash(17),
            place: PlaceNumber(2)
        })
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
    let (meta, _) = service::admit_compact(
        Arc::new(KeptWords(PackedWords(bytes))),
        &mut places,
        DecodeMode::Permissive,
    )
    .unwrap();
    assert_eq!(service::compact_rows(&meta), 2);
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
    let (mut meta, _) = service::admit_compact(
        Arc::new(KeptWords(PackedWords::default())),
        &mut places,
        DecodeMode::Complete,
    )
    .unwrap();
    service::shape_compact(&mut meta, &[CellKind::Number]).unwrap();
    let rows = service::materialize_compact(&meta).unwrap();
    assert_eq!(rows.count, RowCount(0));
    assert!(rows.cells.is_empty());
    assert_eq!(rows.kinds, vec![CellKind::Number]);
}
fn compact_lease_pins_backing_until_metadata_is_retired() {
    let input = packed(&[table()]);
    let weak = Arc::downgrade(&input);
    let (meta, _) =
        service::admit_compact(Arc::clone(&input), &mut vec![], DecodeMode::Complete).unwrap();
    assert!(Arc::ptr_eq(service::compact_backing(&meta), &input));
    let storage = service::compact_storage(&meta);
    assert_eq!(storage[0], (input.0.0.as_ptr() as usize, input.0.0.len()));
    assert!(
        service::compact_metadata_bytes(&meta)
            >= size_of::<types::CompactTable>() + storage[1].1 * size_of::<usize>()
    );
    drop(input);
    assert!(weak.upgrade().is_some());
    same(&service::materialize_compact(&meta).unwrap(), &table());
    drop(meta);
    assert!(weak.upgrade().is_none());
}

fn compact_random_access_all_value_kinds_and_zero_width_frames() {
    use ryo_relations::domain::relations::types::{Flag, NodeId, Offset, Span};
    use ryo_relations::domain::terms::types::Symbol;
    let values = Table {
        kinds: vec![
            CellKind::Node,
            CellKind::Symbol,
            CellKind::File,
            CellKind::Span,
            CellKind::Number,
            CellKind::Flag,
            CellKind::Symbol,
        ],
        count: RowCount(1),
        cells: vec![
            Cell::Node(NodeId(u64::MAX)),
            Cell::Symbol(Symbol(13)),
            Cell::File(FileId(8)),
            Cell::Span(Span {
                start: Offset(7),
                end: Offset(19),
            }),
            Cell::Number(Integer(i64::MIN)),
            Cell::Flag(Flag::Raised),
            Cell::Name(Name {
                hash: TextHash(17),
                place: PlaceNumber(0),
            }),
        ],
    };
    for mode in [DecodeMode::Permissive, DecodeMode::Complete] {
        differential(packed(&[values.clone(), values.clone()]), mode);
        differential(
            packed(&[
                Table {
                    kinds: vec![],
                    count: RowCount(3),
                    cells: vec![],
                },
                Table {
                    kinds: vec![],
                    count: RowCount(2),
                    cells: vec![],
                },
            ]),
            mode,
        );
    }
}

fn permissive_mixed_width_frames_preserve_flat_rows_and_name_places() {
    let a = Table {
        kinds: vec![CellKind::Number, CellKind::Number],
        count: RowCount(1),
        cells: vec![Cell::Number(Integer(1)), Cell::Number(Integer(2))],
    };
    let b = Table {
        kinds: vec![CellKind::Symbol],
        count: RowCount(1),
        cells: vec![Cell::Name(Name {
            hash: TextHash(11),
            place: PlaceNumber(0),
        })],
    };
    let c = Table {
        kinds: vec![CellKind::Symbol, CellKind::Number, CellKind::Number],
        count: RowCount(1),
        cells: vec![
            Cell::Name(Name {
                hash: TextHash(12),
                place: PlaceNumber(1),
            }),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(4)),
        ],
    };
    differential(packed(&[a, b, c]), DecodeMode::Permissive);
}

type Input = fn() -> Result<(), Failure>;
type Value = ();
type Failure = crate::domain::table_words::errors::WordsDecodeError;
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
        name: CaseName("compact_lease_pins_backing_until_metadata_is_retired"),
        word: Word::Succeeds,
        input: || {
            || {
                compact_lease_pins_backing_until_metadata_is_retired();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("compact_random_access_all_value_kinds_and_zero_width_frames"),
        word: Word::Succeeds,
        input: || {
            || {
                compact_random_access_all_value_kinds_and_zero_width_frames();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("permissive_mixed_width_frames_preserve_flat_rows_and_name_places"),
        word: Word::Succeeds,
        input: || {
            || {
                permissive_mixed_width_frames_preserve_flat_rows_and_name_places();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("incomplete_input_refuses_without_publishing_places"),
        word: Word::Refuses,
        input: || incomplete_input,
        expect: Expect::Refusal(|error| selects(matches!(error, Failure::Incomplete))),
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
    let result = service::admit_compact(
        Arc::new(KeptWords(PackedWords(vec![0x80]))),
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
