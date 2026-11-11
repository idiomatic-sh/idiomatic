//! Catalog-wide physical access and owner-only admission, independent of semantic completeness.
use super::super::super::service::RelationOwner;
use crate::application::file_dag::relations::publication::types::RelationResult;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    coverage::types::{Lacking, Partial},
    operating::errors::OperatingError,
    table_model::{types::PhysicalShape, types::Rows},
};
use ryo_relations::domain::relations::{
    service::{all, kinds, to_bits},
    types::{CellKind, ColumnsView, FileId, Integer, Relation, Value as CellValue},
};
use ryo_values::domain::stored_facts::types::PartialCause;
use std::{cell::Cell, sync::Arc};

use super::fixtures::{columns, group, read, row};
use crate::application::file_dag::relations::types::{CheckCount, Sameness};
use std::borrow::Cow;

/// The populations the owner publishes and is asked to admit, resolve and read.
#[derive(Clone, Copy, Debug)]
enum Published {
    /// One compact table of the numbers 4 and 5, published twice, then its population released.
    Compact,
    /// Two stored rows of every catalog relation.
    Catalog,
    /// `FilesystemLineCount` rows whose buffer is reused for the next population.
    ReusedBuffer,
    /// `Function` rows the file's partials mark partial, published twice.
    Partial,
    /// `SourceLocation` rows whose second column is one row short, normalized twice.
    Ragged,
}

/// What two publications of one compact table observed.
#[derive(Debug, PartialEq)]
struct Compact {
    names: usize,
    shape: ryo_values::domain::table_words::types::TableShape,
    records: Sameness,
    second_adds_only_its_record: bool,
    first_charges_backing_once: bool,
    rows: Cow<'static, [Cow<'static, [CellValue]>]>,
    optional_bytes: usize,
    backing_alive: [bool; 2],
    released_first: Option<OperatingError>,
    released_second: Option<OperatingError>,
    consumer_keeps_backing: bool,
    consumer_charged: bool,
    backing_after_consumer: [bool; 2],
    charge_after_consumer: (usize, usize, usize),
}

/// What publishing every catalog relation observed.
#[derive(Debug, PartialEq)]
struct Catalog {
    complete: usize,
    complete_again: usize,
    asked_its_relation: usize,
    read_its_rows: usize,
    groups: [usize; 3],
    checks: CheckCount,
    reuses: CheckCount,
}

/// What a reused buffer observed across populations.
#[derive(Debug, PartialEq)]
struct Epoch {
    original: Result<PhysicalShape, OperatingError>,
    same_buffer: bool,
    old_inspected: Option<OperatingError>,
    old_resolved: Option<OperatingError>,
    provider_reads: usize,
    fresh_record: Sameness,
    fresh: Result<PhysicalShape, OperatingError>,
    checks: CheckCount,
    reads_the_new_numbers: bool,
    other_owner: Option<OperatingError>,
}

/// What a partial population observed.
#[derive(Debug, PartialEq)]
struct PartialPopulation {
    record: Sameness,
    shapes: [Result<PhysicalShape, OperatingError>; 2],
    checks: CheckCount,
    eligible: bool,
}

/// What normalizing a ragged population observed.
#[derive(Debug, PartialEq)]
struct Normalized {
    one_table: bool,
    reads_the_first_row_alone: bool,
    second_reads_the_same: bool,
    checks: CheckCount,
}

/// What the owner observed.
#[derive(Debug, PartialEq)]
enum Observed {
    Compact(Compact),
    Catalog(Catalog),
    Epoch(Epoch),
    Partial(PartialPopulation),
    Normalized(Normalized),
}

type Input = Published;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "compact_publications_share_backing_accounting_and_expire_with_generation",
        ),
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

fn fresh_owner() -> RelationOwner {
    RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        Arc::new(crate::application::file_dag::fixtures::derived()),
    )
}

/// Publishes the input to a fresh owner and asks it as the input says.
fn subject(input: Input) -> Result<Value, Failure> {
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
        == crate::application::file_dag::derived::ledger::capacity::service::compact_table(&compact).0
            + size_of::<KeptWords>()
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

fn catalog() -> Result<Catalog, OperatingError> {
    let owner = fresh_owner();
    let mut observed = Catalog {
        complete: 0,
        complete_again: 0,
        asked_its_relation: 0,
        read_its_rows: 0,
        groups: [0; 3],
        checks: CheckCount(0),
        reuses: CheckCount(0),
    };
    for relation in all() {
        let schema = kinds(*relation);
        let physical = columns(schema);
        let source = ColumnsView {
            kinds: schema,
            columns: &physical,
        };
        let rows = Rows::Stored(source);
        let publication = owner.source(*relation, &rows)?;
        let complete = || {
            usize::from(
                owner.inspect(publication.as_ref(), RepresentationInput::from(&rows))
                    == Ok(PhysicalShape::Complete),
            )
        };
        observed.complete += complete();
        observed.complete_again += complete();
        let asked_its_relation = Cell::new(true);
        let resolved = owner.resolve(publication.as_ref(), &|asked| {
            asked_its_relation.set(asked_its_relation.get() && asked == *relation);
            Ok(Rows::Stored(source))
        })?;
        observed.asked_its_relation += usize::from(asked_its_relation.get());
        observed.read_its_rows += usize::from(read(&resolved) == [row(schema, 0), row(schema, 1)]);
        observed.groups[group(*relation)] += 1;
    }
    observed.checks = owner.checks.get();
    observed.reuses = owner.reuses.get();
    Ok(observed)
}

fn epoch() -> Result<Epoch, OperatingError> {
    let relation = Relation::FilesystemLineCount;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    let owner = fresh_owner();
    let (original, original_shape) = {
        let rows = Rows::Stored(ColumnsView {
            kinds: schema,
            columns: &physical,
        });
        let publication = owner.source(relation, &rows)?;
        let shape = owner.inspect(publication.as_ref(), RepresentationInput::from(&rows));
        (publication, shape)
    };
    let address = physical[1].0.as_ptr();
    owner.release_population();
    physical[1].0.clear();
    physical[1].0.extend([
        to_bits(CellValue::Number(Integer(200))).0,
        to_bits(CellValue::Number(Integer(201))).0,
    ]);
    let same_buffer = address == physical[1].0.as_ptr() && physical[1].0.len() == 2;
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let rows = Rows::Stored(source);
    let old_inspected = owner
        .inspect(original.as_ref(), RepresentationInput::from(&rows))
        .err();
    let reads = Cell::new(0);
    let old_resolved = owner
        .resolve(original.as_ref(), &|_| {
            reads.set(reads.get() + 1);
            Ok(Rows::Stored(source))
        })
        .err();
    let fresh = owner.source(relation, &rows)?;
    let fresh_record = original.same_record(&fresh);
    let fresh_shape = owner.inspect(fresh.as_ref(), RepresentationInput::from(&rows));
    let checks = owner.checks.get();
    let resolved = owner.resolve(fresh.as_ref(), &|_| Ok(Rows::Stored(source)))?;
    let mut expected = [row(schema, 0), row(schema, 1)];
    expected[0][1] = CellValue::Number(Integer(200));
    expected[1][1] = CellValue::Number(Integer(201));
    let reads_the_new_numbers = read(&resolved) == expected;
    let other_owner = fresh_owner()
        .inspect(fresh.as_ref(), RepresentationInput::from(&rows))
        .err();
    Ok(Epoch {
        original: original_shape,
        same_buffer,
        old_inspected,
        old_resolved,
        provider_reads: reads.get(),
        fresh_record,
        fresh: fresh_shape,
        checks,
        reads_the_new_numbers,
        other_owner,
    })
}

fn partial() -> Result<PartialPopulation, OperatingError> {
    let relation = Relation::Function;
    let physical = columns(kinds(relation));
    let rows = Rows::Stored(ColumnsView {
        kinds: kinds(relation),
        columns: &physical,
    });
    let partials = [Partial {
        relation,
        cause: PartialCause("fixture partial".into()),
        lacking: Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(9)),
    }];
    let owner = fresh_owner();
    let first = owner.source(relation, &rows)?;
    let second = owner.source(relation, &rows)?;
    let record = first.same_record(&second);
    let shapes = [&first, &second]
        .map(|publication| owner.inspect(publication.as_ref(), RepresentationInput::from(&rows)));
    let checks = owner.checks.get();
    let prepared =
        crate::application::file_dag::derived::prepared::RequestDerived::new(Box::new([
            crate::application::file_dag::derived::owner::fixtures::reads(relation),
        ]));
    owner.derived.prepare_program_file(
        &prepared,
        &[ryo_execution_domain::domain::coverage::types::RuleIndex(0)],
        &partials,
    )?;
    Ok(PartialPopulation {
        record,
        shapes,
        checks,
        // Physical completeness cannot erase a partial relation.
        eligible: owner.derived.relation_eligible(relation),
    })
}

fn normalized() -> Result<Normalized, OperatingError> {
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    physical[1].0.truncate(1);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let owner = fresh_owner();
    let publication = owner.source(Relation::Function, &Rows::Stored(source))?;
    let first = owner.normalize(&publication, Rows::Stored(source))?;
    let second = owner.normalize(&publication, Rows::Stored(source))?;
    let (Rows::Computed(first_table), Rows::Computed(second_table)) = (&first, &second) else {
        panic!("ragged input needs an owned normalized population")
    };
    Ok(Normalized {
        one_table: std::ptr::eq(*first_table, *second_table),
        reads_the_first_row_alone: read(&first) == [row(schema, 0)],
        second_reads_the_same: read(&second) == read(&first),
        checks: owner.checks.get(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
