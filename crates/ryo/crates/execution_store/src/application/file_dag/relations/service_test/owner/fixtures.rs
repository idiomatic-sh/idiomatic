//! Schema-derived typed populations and ordinary access, shared by admission fixtures.
use ryo_execution_domain::domain::table_model::{
    service::{count_of, read_row},
    types::Rows,
};
use ryo_relations::domain::relations::{
    service::{node_id, to_bits},
    types::{CellKind, FactColumn, FileId, Flag, Integer, Offset, Position, Relation, Span, Value},
};
use ryo_relations::domain::terms::types::Symbol;

macro_rules! catalog_groups {
    (syntax { $( $syntax:ident $sm:ident $sr:ident $sc:ident ; )* }
     manifest { $( $manifest:ident $mm:ident $mr:ident $mc:ident ; )* }
     engine { $( $engine:ident $em:ident $er:ident $ec:ident ; )* }) => {
        pub(super) fn group(relation: Relation) -> usize {
            match relation { $(Relation::$syntax => 0,)* $(Relation::$manifest => 1,)* $(Relation::$engine => 2,)* }
        }
    };
}
ryo_relations::each_relation!(catalog_groups);

pub(super) fn row(schema: &[CellKind], at: u32) -> Vec<Value> {
    schema
        .iter()
        .enumerate()
        .map(|(column, kind)| {
            let column = u32::try_from(column).unwrap();
            match kind {
                CellKind::Node => Value::Node(node_id(FileId(9), Position(at + column + 1))),
                CellKind::Symbol => Value::Symbol(Symbol(at + column + 3)),
                CellKind::File => Value::File(FileId(9)),
                CellKind::Span => Value::Span(Span {
                    start: Offset((at + column) * 2),
                    end: Offset((at + column) * 2 + 2),
                }),
                CellKind::Number => Value::Number(Integer(i64::from(100 + at * 10 + column))),
                CellKind::Flag => Value::Flag(if at == 0 { Flag::Raised } else { Flag::Lowered }),
            }
        })
        .collect()
}
pub(super) fn columns(schema: &[CellKind]) -> Vec<FactColumn> {
    let rows = [row(schema, 0), row(schema, 1)];
    (0..schema.len())
        .map(|at| FactColumn(rows.iter().map(|row| to_bits(row[at]).0).collect()))
        .collect()
}
pub(super) fn read(rows: &Rows<'_>) -> Vec<Vec<Value>> {
    let mut decode = Vec::new();
    (0..count_of(rows).0)
        .map(|at| {
            read_row(
                rows,
                ryo_values::domain::rows::types::RowIndex(at),
                &mut decode,
            )
            .unwrap()
            .to_vec()
        })
        .collect()
}
use super::super::super::service::RelationOwner;
use crate::application::file_dag::relations::types::{CheckCount, Sameness};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    coverage::types::{Lacking, Partial},
    operating::errors::OperatingError,
    table_model::types::PhysicalShape,
};
use ryo_relations::domain::relations::{
    service::{all, kinds},
    types::{ColumnsView, Value as CellValue},
};
use ryo_values::domain::stored_facts::types::PartialCause;
use std::borrow::Cow;
use std::{cell::Cell, sync::Arc};

/// The populations the owner publishes and is asked to admit, resolve and read.
#[derive(Clone, Copy, Debug)]
pub(super) enum Published {
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
pub(super) struct Compact {
    pub(super) names: usize,
    pub(super) shape: ryo_values::domain::table_words::types::TableShape,
    pub(super) records: Sameness,
    pub(super) second_adds_only_its_record: bool,
    pub(super) first_charges_backing_once: bool,
    pub(super) rows: Cow<'static, [Cow<'static, [CellValue]>]>,
    pub(super) optional_bytes: usize,
    pub(super) backing_alive: [bool; 2],
    pub(super) released_first: Option<OperatingError>,
    pub(super) released_second: Option<OperatingError>,
    pub(super) consumer_keeps_backing: bool,
    pub(super) consumer_charged: bool,
    pub(super) backing_after_consumer: [bool; 2],
    pub(super) charge_after_consumer: (usize, usize, usize),
}

/// What publishing every catalog relation observed.
#[derive(Debug, PartialEq)]
pub(super) struct Catalog {
    pub(super) complete: usize,
    pub(super) complete_again: usize,
    pub(super) asked_its_relation: usize,
    pub(super) read_its_rows: usize,
    pub(super) groups: [usize; 3],
    pub(super) checks: CheckCount,
    pub(super) reuses: CheckCount,
}

/// What a reused buffer observed across populations.
#[derive(Debug, PartialEq)]
pub(super) struct Epoch {
    pub(super) original: Result<PhysicalShape, OperatingError>,
    pub(super) same_buffer: bool,
    pub(super) old_inspected: Option<OperatingError>,
    pub(super) old_resolved: Option<OperatingError>,
    pub(super) provider_reads: usize,
    pub(super) fresh_record: Sameness,
    pub(super) fresh: Result<PhysicalShape, OperatingError>,
    pub(super) checks: CheckCount,
    pub(super) reads_the_new_numbers: bool,
    pub(super) other_owner: Option<OperatingError>,
}

/// What a partial population observed.
#[derive(Debug, PartialEq)]
pub(super) struct PartialPopulation {
    pub(super) record: Sameness,
    pub(super) shapes: [Result<PhysicalShape, OperatingError>; 2],
    pub(super) checks: CheckCount,
    pub(super) eligible: bool,
}

/// What normalizing a ragged population observed.
#[derive(Debug, PartialEq)]
pub(super) struct Normalized {
    pub(super) one_table: bool,
    pub(super) reads_the_first_row_alone: bool,
    pub(super) second_reads_the_same: bool,
    pub(super) checks: CheckCount,
}

/// What the owner observed.
#[derive(Debug, PartialEq)]
pub(super) enum Observed {
    Compact(Compact),
    Catalog(Catalog),
    Epoch(Epoch),
    Partial(PartialPopulation),
    Normalized(Normalized),
}

pub(super) fn fresh_owner() -> RelationOwner {
    RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        Arc::new(crate::application::file_dag::fixtures::derived()),
    )
}

pub(super) fn catalog() -> Result<Catalog, OperatingError> {
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

pub(super) fn epoch() -> Result<Epoch, OperatingError> {
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

pub(super) fn partial() -> Result<PartialPopulation, OperatingError> {
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
        crate::application::file_dag::derived::prepared::service::RequestDerived::new(Box::new([
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

pub(super) fn normalized() -> Result<Normalized, OperatingError> {
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
