//! Catalog-wide physical access and owner-only admission, independent of semantic completeness.
use super::fixtures::{
    Catalog, Compact, Epoch, Normalized, Observed, PartialPopulation, Published, catalog, epoch,
    fresh_owner, normalized, partial, read,
};
use crate::application::file_dag::relations::publication::types::RelationResult;
use crate::application::file_dag::relations::types::{CheckCount, Sameness};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError,
    table_model::types::PhysicalShape,
};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as CellValue};
use std::borrow::Cow;
use std::sync::Arc;

type Input = Published;

type Value = Observed;

type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("compact_publications_share_backing_accounting_and_expire_with_generation"),
        word: Word::Succeeds,
        input: || Published::Compact,
        expect: Expect::Value(Observed::Compact(Compact {
            names: 0,
            shape: ryo_values::domain::table_words::types::TableShape::Rectangular,
            records: Sameness::Different,
            second_adds_only_its_record: true,
            first_charges_backing_once: true,
            rows: Cow::Borrowed(&[
                Cow::Borrowed(&[CellValue::Number(Integer(4))]),
                Cow::Borrowed(&[CellValue::Number(Integer(5))]),
            ]),
            optional_bytes: 0,
            backing_alive: [true, true],
            released_first: Some(OperatingError::NotRead),
            released_second: Some(OperatingError::NotRead),
            consumer_keeps_backing: true,
            consumer_charged: true,
            backing_after_consumer: [false, false],
            charge_after_consumer: (0, 0, 0),
        })),
    },
    Case {
        name: CaseName(
            "every_catalog_relation_is_admitted_resolved_and_read_with_its_actual_schema",
        ),
        word: Word::Succeeds,
        input: || Published::Catalog,
        expect: Expect::Value(Observed::Catalog(Catalog {
            complete: 69,
            complete_again: 69,
            asked_its_relation: 69,
            read_its_rows: 69,
            // 69 catalog relations, separately from five physical representations.
            groups: [57, 8, 4],
            checks: CheckCount(69),
            reuses: CheckCount(69),
        })),
    },
    Case {
        name: CaseName(
            "population_epoch_invalidates_proofs_even_when_same_buffer_and_count_are_reused",
        ),
        word: Word::Succeeds,
        input: || Published::ReusedBuffer,
        expect: Expect::Value(Observed::Epoch(Epoch {
            original: Ok(PhysicalShape::Complete),
            same_buffer: true,
            old_inspected: Some(OperatingError::NotRead),
            old_resolved: Some(OperatingError::NotRead),
            provider_reads: 0,
            fresh_record: Sameness::Different,
            fresh: Ok(PhysicalShape::Complete),
            checks: CheckCount(2),
            reads_the_new_numbers: true,
            other_owner: Some(OperatingError::NotRead),
        })),
    },
    Case {
        name: CaseName(
            "stable_partial_population_reuses_structure_without_proving_semantic_absence",
        ),
        word: Word::Succeeds,
        input: || Published::Partial,
        expect: Expect::Value(Observed::Partial(PartialPopulation {
            record: Sameness::Same,
            shapes: [Ok(PhysicalShape::Complete), Ok(PhysicalShape::Complete)],
            checks: CheckCount(1),
            eligible: false,
        })),
    },
    Case {
        name: CaseName("normalization_reuses_one_owned_fallback_without_promoting_missing_rows"),
        word: Word::Succeeds,
        input: || Published::Ragged,
        expect: Expect::Value(Observed::Normalized(Normalized {
            one_table: true,
            reads_the_first_row_alone: true,
            second_reads_the_same: true,
            checks: CheckCount(1),
        })),
    },
];

/// Publishes the input to a fresh owner and asks it as the input says.
fn subject(input: Published) -> Result<Observed, OperatingError> {
    Ok(match input {
        Published::Compact => Observed::Compact(compact()?),
        Published::Catalog => Observed::Catalog(catalog()?),
        Published::ReusedBuffer => Observed::Epoch(epoch()?),
        Published::Partial => Observed::Partial(partial()?),
        Published::Ragged => Observed::Normalized(normalized()?),
    })
}

fn compact() -> Result<Compact, OperatingError> {
    use ryo_relations::domain::relations::types::Words;
    use ryo_values::domain::table_words::{
        service,
        types::{DecodeMode, WordSlice},
    };
    use ryo_values::{
        domain::rows::types::{RowCount, Table},
        domain::stored_facts::types::KeptWords,
    };

    let table = Table {
        kinds: vec![CellKind::Number],
        cells: vec![CellValue::Number(Integer(4)), CellValue::Number(Integer(5))],
        count: RowCount(2),
    };
    let mut encoded = Words::default();
    service::encode_table(&table, &[], &mut encoded);
    let words = Arc::new(KeptWords(service::pack(WordSlice(&encoded.0))));
    let words_weak = Arc::downgrade(&words);
    let ryo_values::domain::table_words::types::Admitted {
        table: compact,
        names,
    } = ryo_values::domain::compact_table::service::admit_compact(
        ryo_values::domain::table_words::types::CompactBacking(Arc::clone(&words)),
        &mut Vec::new(),
        DecodeMode::Complete,
    )
    .expect("a complete compact table");
    let shape = ryo_values::domain::compact_table::service::compact_rectangular(&compact);
    let compact = Arc::new(compact);
    let compact_weak = Arc::downgrade(&compact);
    let owner = fresh_owner();
    let first = owner.source_compact(Arc::clone(&compact))?;
    drop(owner.admit(first.as_ref(), RepresentationInput::Compact(&compact))?);
    let first_unique = owner.derived.publication_ownership_bytes().2;
    let second = owner.source_compact(Arc::clone(&compact))?;
    drop(owner.admit(second.as_ref(), RepresentationInput::Compact(&compact))?);
    let records = first.same_record(&second);
    // A second publication of the same immutable Compact and word buffer
    // adds only its own record. Shared backing allocations are not charged
    // again to the owner ledger.
    let second_adds_only_its_record = owner.derived.publication_ownership_bytes().2
        == first_unique + size_of::<RelationResult>() + 2 * size_of::<usize>();
    let first_charges_backing_once = first_unique
        == crate::application::file_dag::accounting::ledger::capacity::service::compact_table(
            &compact,
        )
        .0 + size_of::<KeptWords>()
            + 2 * size_of::<usize>()
            + words.0.0.capacity()
            + size_of::<RelationResult>()
            + 2 * size_of::<usize>();
    let unread = |_| Err(OperatingError::NotRead);
    let rows = {
        let resolved = owner.resolve(first.as_ref(), &unread)?;
        Cow::Owned(read(&resolved).into_iter().map(Cow::Owned).collect())
    };
    let consumer = first.clone();
    let optional_bytes = owner.derived.publication_ownership_bytes().1;
    drop(compact);
    drop(words);
    let backing_alive = [
        compact_weak.upgrade().is_some(),
        words_weak.upgrade().is_some(),
    ];
    owner.release_population();
    let released_first = owner.resolve(first.as_ref(), &unread).err();
    let released_second = owner.resolve(second.as_ref(), &unread).err();
    drop(first);
    drop(second);
    let consumer_keeps_backing = compact_weak.upgrade().is_some();
    let consumer_charged = owner.derived.publication_ownership_bytes().0 > 0;
    drop(consumer);
    Ok(Compact {
        names: names.len(),
        shape,
        records,
        second_adds_only_its_record,
        first_charges_backing_once,
        rows,
        optional_bytes,
        backing_alive,
        released_first,
        released_second,
        consumer_keeps_backing,
        consumer_charged,
        backing_after_consumer: [
            compact_weak.upgrade().is_some(),
            words_weak.upgrade().is_some(),
        ],
        charge_after_consumer: owner.derived.publication_ownership_bytes(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
