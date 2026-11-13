//! Exercise the compact backing directly: leases, random access, mixed widths and cross-file symbol identities.
use super::{differential, packed, place, same, table};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::compact_table::service as compact;
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::rows::types::RowIndex;
use crate::domain::rows::types::{RowCount, Table};
use crate::domain::stored_facts::types::KeptWords;
use crate::domain::table_words::{service, types};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, Name, Offset, PlaceNumber, TextHash, Value as Cell, Words,
};
use std::sync::Arc;
use types::{Admitted, DecodeMode, WordSlice};
fn compact_lease_pins_backing_until_metadata_is_retired() {
    let input = packed(&[table()]);
    let weak = Arc::downgrade(&input);
    let Admitted { table: meta, .. } = compact::admit_compact(
        crate::domain::table_words::types::CompactBacking(Arc::clone(&input)),
        &mut vec![],
        DecodeMode::Complete,
    )
    .unwrap();
    assert!(Arc::ptr_eq(&compact::compact_backing(&meta).0, &input));
    let storage = compact::compact_storage(&meta);
    assert_eq!(
        storage.words,
        types::StoragePart {
            address: types::StorageAddress(input.0.0.as_ptr().addr()),
            length: types::StorageLength(input.0.0.len()),
        }
    );
    drop(input);
    assert!(weak.upgrade().is_some());
    same(&meta, &table());
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
fn cross_file_symbol_indices_survive_frame_local_rebasing() {
    use ryo_language::domain::symbols::types::ByteSlice;
    use ryo_language::domain::symbols::{
        service::{hash_of, intern_derived, name_made_across},
        types::Symbols,
    };
    use ryo_relations::domain::terms::types::NameText;
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("global"));
    let global = name_made_across(&symbols, symbol).expect("real cross-file symbol");
    let local = Name {
        hash: hash_of(ByteSlice(b"local")),
        place: PlaceNumber(0),
    };
    let frame = Table {
        kinds: vec![CellKind::Symbol, CellKind::Symbol],
        count: RowCount(1),
        cells: vec![Cell::Name(global), Cell::Name(local)],
    };
    let mut words = Words::default();
    service::encode_table(&frame, &[place(1)], &mut words);
    service::encode_table(&frame, &[place(2)], &mut words);
    let input = Arc::new(KeptWords(service::pack(WordSlice(&words.0))));
    let expected = Table {
        kinds: frame.kinds,
        count: RowCount(2),
        cells: vec![
            Cell::Name(global),
            Cell::Name(Name {
                hash: local.hash,
                place: PlaceNumber(2),
            }),
            Cell::Name(global),
            Cell::Name(Name {
                hash: local.hash,
                place: PlaceNumber(3),
            }),
        ],
    };
    for mode in [DecodeMode::Complete, DecodeMode::Permissive] {
        let seed = vec![place(8), place(9)];
        let mut decoded_places = seed.clone();
        let decoded = match mode {
            DecodeMode::Complete => {
                service::decode_complete(&input.0, &mut decoded_places).unwrap()
            }
            DecodeMode::Permissive => service::decode_combined(&input.0, &mut decoded_places),
        };
        let mut compact_places = seed;
        let Admitted {
            table: compact,
            names,
        } = compact::admit_compact(
            crate::domain::table_words::types::CompactBacking(Arc::clone(&input)),
            &mut compact_places,
            mode,
        )
        .unwrap();
        let compact_cells = (0..2)
            .flat_map(|row| (0..2).map(move |column| (row, column)))
            .map(|(row, column)| {
                compact::compact_cell(&compact, RowIndex(row), ColumnIndex(column)).unwrap()
            })
            .collect::<Vec<_>>();
        let mut compact_rows = Vec::new();
        let mut row_scratch = [Cell::Number(Integer(0)); 2];
        for row in 0..2 {
            compact_rows.extend_from_slice(
                compact::compact_read(&compact, RowIndex(row), &mut row_scratch).unwrap(),
            );
        }
        let identities = |cells: &[Cell]| {
            cells
                .iter()
                .map(|cell| {
                    let Cell::Name(name) = *cell else {
                        panic!("name-only fixture")
                    };
                    (name.hash, name.place)
                })
                .collect::<Vec<_>>()
        };
        // Name equality intentionally compares hashes, so place identity must
        // be compared explicitly to detect a global index being renumbered.
        assert_eq!(
            (
                identities(&decoded.cells),
                identities(&compact_cells),
                identities(&compact_rows)
            ),
            (
                identities(&expected.cells),
                identities(&expected.cells),
                identities(&expected.cells)
            )
        );
        // Both actual codec routes execute before comparison; agreement between
        // two routes is insufficient if they both renumber the global symbol.
        assert_eq!(
            (decoded.cells, compact_cells),
            (expected.cells.clone(), expected.cells.clone())
        );
        same(&compact, &expected);
        assert_eq!(decoded_places, compact_places);
        assert_eq!(compact_places.len(), 4);
        assert_eq!(
            names
                .iter()
                .map(|name| (name.hash, name.place))
                .collect::<Vec<_>>(),
            identities(&expected.cells)
        );
    }
}
type Input = fn() -> Result<(), Failure>;
type Value = ();
type Failure = crate::domain::table_words::errors::TableWordsError;
const CASES: &[Case<Input, Value, Failure>] = &[
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
        name: CaseName("cross_file_symbol_indices_survive_frame_local_rebasing"),
        word: Word::Succeeds,
        input: || {
            || {
                cross_file_symbol_indices_survive_frame_local_rebasing();
                Ok(())
            }
        },
        expect: Expect::Value(()),
    },
];
fn subject(check: Input) -> Result<Value, Failure> {
    check()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
