//! Capacity accounting must include reserved room, not merely logical lengths.
use crate::cases::{Case, CaseName, Expect, Word, run};

use super::service::{arc_bytes, vector};

fn vector_accounting_charges_unused_capacity_and_arc_header() {
    let mut values = Vec::<u32>::with_capacity(19);
    values.push(7);
    assert_eq!(vector(&values).0, values.capacity() * size_of::<u32>());
    assert!(arc_bytes::<Vec<u32>>().0 >= size_of::<Vec<u32>>() + 2 * size_of::<usize>());
}

fn compact_table_accounting_charges_header_and_reserved_vectors() {
    use ryo_values::domain::stored_facts::types::{KeptWords, PackedWords};
    use ryo_values::domain::table_words::types;
    let admitted = ryo_values::domain::compact_table::service::admit_compact(
        types::CompactBacking(std::sync::Arc::new(KeptWords(PackedWords::default()))),
        &mut Vec::new(),
        types::DecodeMode::Complete,
    )
    .unwrap();
    let table = admitted.table;
    assert!(
        super::service::compact_table(&table).0
            >= size_of::<types::CompactTable>()
                + table.row_offsets.capacity() * size_of::<types::FrameOffset>()
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    VectorAccountingChargesUnusedCapacityAndArcHeader,
    CompactTableAccountingChargesHeaderAndReservedVectors,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("vector_accounting_charges_unused_capacity_and_arc_header"),
        word: Word::Succeeds,
        input: || Scenario::VectorAccountingChargesUnusedCapacityAndArcHeader,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("compact_table_accounting_charges_header_and_reserved_vectors"),
        word: Word::Succeeds,
        input: || Scenario::CompactTableAccountingChargesHeaderAndReservedVectors,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::VectorAccountingChargesUnusedCapacityAndArcHeader => {
            vector_accounting_charges_unused_capacity_and_arc_header();
            Ok(())
        }
        Scenario::CompactTableAccountingChargesHeaderAndReservedVectors => {
            compact_table_accounting_charges_header_and_reserved_vectors();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
