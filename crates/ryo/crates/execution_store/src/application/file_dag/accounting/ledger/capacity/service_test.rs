//! Capacity accounting must include reserved room, not merely logical lengths.
use crate::cases::{Case, CaseName, Expect, Word, run};

use super::service::{arc_bytes, compact_table, vector};
use ryo_values::domain::compact_table::service::admit_compact;
use ryo_values::domain::stored_facts::types::{KeptWords, PackedWords};
use ryo_values::domain::table_words::types::{
    CompactBacking, CompactTable, DecodeMode, FrameOffset,
};
use std::sync::Arc;

/// What is accounted: a vector with reserved room, the `Arc` around one, or an admitted compact table.
#[derive(Clone, Copy, Debug)]
enum Held {
    /// A `Vec<u32>` reserved for `capacity` values holding `len` of them.
    Vector { capacity: usize, len: usize },
    /// The `Arc` header around a `Vec<u32>`.
    ArcOfVector,
    /// The empty compact table admitted from default packed words.
    CompactTable,
}

/// The bytes charged, and the floor the charge must reach: the reserved room plus every header.
#[derive(Debug, PartialEq, Eq)]
struct Charge {
    bytes: usize,
    floor: usize,
}

type Input = Held;
type Value = Charge;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("vector_accounting_charges_unused_capacity_and_arc_header"),
        word: Word::Succeeds,
        input: || Held::Vector {
            capacity: 19,
            len: 1,
        },
        expect: Expect::Value(Charge {
            bytes: 76,
            floor: 76,
        }),
    },
    Case {
        name: CaseName("an_arc_charges_its_two_counters_beside_the_value"),
        word: Word::Succeeds,
        input: || Held::ArcOfVector,
        expect: Expect::Value(Charge {
            bytes: 47,
            floor: 40,
        }),
    },
    Case {
        name: CaseName("compact_table_accounting_charges_header_and_reserved_vectors"),
        word: Word::Succeeds,
        input: || Held::CompactTable,
        expect: Expect::Value(Charge {
            bytes: 112,
            floor: 96,
        }),
    },
];

/// Charges the held value and states the floor its reserved room and headers set.
fn subject(held: Input) -> Result<Value, Failure> {
    Ok(match held {
        Held::Vector { capacity, len } => {
            let mut values = Vec::<u32>::with_capacity(capacity);
            values.extend((0..len).map(|_| 7));
            Charge {
                bytes: vector(&values).0,
                floor: values.capacity() * size_of::<u32>(),
            }
        }
        Held::ArcOfVector => Charge {
            bytes: arc_bytes::<Vec<u32>>().0,
            floor: size_of::<Vec<u32>>() + 2 * size_of::<usize>(),
        },
        Held::CompactTable => {
            let table = admit_compact(
                CompactBacking(Arc::new(KeptWords(PackedWords::default()))),
                &mut Vec::new(),
                DecodeMode::Complete,
            )
            .map_err(|error| format!("{error:?}"))?
            .table;
            Charge {
                bytes: compact_table(&table).0,
                floor: size_of::<CompactTable>()
                    + table.row_offsets.capacity() * size_of::<FrameOffset>(),
            }
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
