//! Catalog-wide physical access and owner-only admission, independent of semantic completeness.
use super::RelationOwner;
use crate::domain::{
    coverage::types::{Lacking, Partial},
    operating::errors::OperatingError,
    operator::contract::views::BorrowedRows,
    table_model::{
        collections::representations::{
            publication::PublishedRelation,
            types::{PhysicalShape, RepresentationKind},
        },
        service::complete_for_reuse,
        types::{Rows, SelectedRows, StoredSelection},
    },
};
use ryo_relations::domain::relations::{
    service::{all, kinds, to_bits},
    types::{CellKind, ColumnsView, FileId, Integer, Relation, Value},
};
use ryo_values::domain::{plan_parts::types::ColumnIndex, stored_facts::types::PartialCause};
use std::{cell::Cell, sync::Arc};
mod fixtures;
mod producer;
use fixtures::{columns, group, read, row, selected};

#[test]
fn every_catalog_relation_is_admitted_resolved_and_read_with_its_actual_schema() {
    let owner = RelationOwner::new(true);
    let mut groups = [0; 3];
    for relation in all() {
        let schema = kinds(*relation);
        let physical = columns(schema);
        let source = ColumnsView {
            kinds: schema,
            columns: &physical,
        };
        let rows = Rows::Stored(source);
        let publication = owner.source(*relation, &rows).unwrap();
        assert_eq!(
            owner.inspect(publication.as_ref(), BorrowedRows::from(&rows)),
            Ok(PhysicalShape::Complete),
            "{relation:?}"
        );
        assert_eq!(
            owner.inspect(publication.as_ref(), BorrowedRows::from(&rows)),
            Ok(PhysicalShape::Complete),
            "{relation:?}"
        );
        let resolved = owner
            .resolve(publication.as_ref(), &|asked| {
                assert_eq!(asked, *relation);
                Ok(Rows::Stored(source))
            })
            .unwrap();
        assert_eq!(
            read(&resolved),
            [row(schema, 0), row(schema, 1)],
            "{relation:?}"
        );
        groups[group(*relation)] += 1;
    }
    assert_eq!(
        groups,
        [57, 7, 4],
        "68 catalog relations, separately from four physical representations"
    );
    assert_eq!(owner.checks.get(), 68);
    assert_eq!(owner.reuses.get(), 68);
}

#[test]
fn population_epoch_invalidates_proofs_even_when_same_buffer_and_count_are_reused() {
    let relation = Relation::FilesystemLineCount;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    let owner = RelationOwner::new(true);
    let original = {
        let rows = Rows::Stored(ColumnsView {
            kinds: schema,
            columns: &physical,
        });
        let publication = owner.source(relation, &rows).unwrap();
        assert_eq!(
            owner.inspect(publication.as_ref(), BorrowedRows::from(&rows)),
            Ok(PhysicalShape::Complete)
        );
        publication
    };
    let address = physical[1].0.as_ptr();
    owner.release_population();
    physical[1].0.clear();
    physical[1].0.extend([
        to_bits(Value::Number(Integer(200))).0,
        to_bits(Value::Number(Integer(201))).0,
    ]);
    assert_eq!(address, physical[1].0.as_ptr());
    assert_eq!(physical[1].0.len(), 2);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let rows = Rows::Stored(source);
    assert!(matches!(
        owner.inspect(original.as_ref(), BorrowedRows::from(&rows)),
        Err(OperatingError::NotRead)
    ));
    let reads = Cell::new(0);
    assert!(matches!(
        owner.resolve(original.as_ref(), &|_| {
            reads.set(reads.get() + 1);
            Ok(Rows::Stored(source))
        }),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(
        reads.get(),
        0,
        "old epoch is refused before provider access"
    );
    let fresh = owner.source(relation, &rows).unwrap();
    assert!(!original.same_record(&fresh));
    assert_eq!(
        owner.inspect(fresh.as_ref(), BorrowedRows::from(&rows)),
        Ok(PhysicalShape::Complete)
    );
    assert_eq!(owner.checks.get(), 2);
    let resolved = owner
        .resolve(fresh.as_ref(), &|_| Ok(Rows::Stored(source)))
        .unwrap();
    let mut expected = [row(schema, 0), row(schema, 1)];
    expected[0][1] = Value::Number(Integer(200));
    expected[1][1] = Value::Number(Integer(201));
    assert_eq!(read(&resolved), expected);
    let other = RelationOwner::new(true);
    assert!(matches!(
        other.inspect(fresh.as_ref(), BorrowedRows::from(&rows)),
        Err(OperatingError::NotRead)
    ));
}

#[test]
fn different_projections_and_positions_have_distinct_admissions_and_correct_values() {
    let schema = kinds(Relation::SourceLocation);
    let physical = columns(schema);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let numeric: Vec<_> = schema
        .iter()
        .enumerate()
        .filter(|(_, kind)| **kind == CellKind::Number)
        .map(|(at, _)| ColumnIndex(u32::try_from(at).unwrap()))
        .take(2)
        .collect();
    assert_eq!(numeric.len(), 2);
    let first = selected(
        source,
        vec![1, 0, 1],
        vec![numeric[1], numeric[0], numeric[1]],
        vec![CellKind::Number; 3],
    );
    let second = SelectedRows {
        source,
        selection: Arc::new(StoredSelection {
            relation: Relation::SourceLocation,
            rows: Arc::clone(&first.selection.rows),
            columns: vec![numeric[0], numeric[1], numeric[0]],
            kinds: vec![CellKind::Number; 3],
            identity: Arc::clone(&first.selection.identity),
        }),
    };
    assert!(Arc::ptr_eq(&first.selection.rows, &second.selection.rows));
    let shifted = selected(
        source,
        vec![0, 1, 0],
        first.selection.columns.clone(),
        first.selection.kinds.clone(),
    );
    let owner = RelationOwner::new(true);
    let mut publications = Vec::new();
    let a = [
        row(schema, 0)[numeric[0].0 as usize],
        row(schema, 1)[numeric[0].0 as usize],
    ];
    let b = [
        row(schema, 0)[numeric[1].0 as usize],
        row(schema, 1)[numeric[1].0 as usize],
    ];
    for (view, expected) in [
        (
            &first,
            vec![
                vec![b[1], a[1], b[1]],
                vec![b[0], a[0], b[0]],
                vec![b[1], a[1], b[1]],
            ],
        ),
        (
            &second,
            vec![
                vec![a[1], b[1], a[1]],
                vec![a[0], b[0], a[0]],
                vec![a[1], b[1], a[1]],
            ],
        ),
        (
            &shifted,
            vec![
                vec![b[0], a[0], b[0]],
                vec![b[1], a[1], b[1]],
                vec![b[0], a[0], b[0]],
            ],
        ),
    ] {
        let rows = Rows::Selected(view.clone());
        let publication = owner.source(Relation::SourceLocation, &rows).unwrap();
        assert_eq!(
            owner.inspect(publication.as_ref(), BorrowedRows::from(&rows)),
            Ok(PhysicalShape::Complete)
        );
        let resolved = owner
            .resolve(publication.as_ref(), &|_| Ok(Rows::Selected(view.clone())))
            .unwrap();
        assert_eq!(read(&resolved), expected);
        publications.push(publication);
    }
    assert!(!publications[0].same_record(&publications[1]));
    assert!(!publications[0].same_record(&publications[2]));
    assert!(matches!(
        owner.inspect(
            publications[0].as_ref(),
            BorrowedRows::from(&Rows::Selected(second))
        ),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(owner.checks.get(), 3);
}

#[test]
fn arbitrary_selected_metadata_never_gains_a_trusted_complete_stamp() {
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let physical = columns(schema);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let numeric = ColumnIndex(
        u32::try_from(
            schema
                .iter()
                .position(|kind| *kind == CellKind::Number)
                .unwrap(),
        )
        .unwrap(),
    );
    let owner = RelationOwner::new(true);
    for view in [
        selected(
            source,
            vec![0],
            vec![ColumnIndex(u32::MAX)],
            vec![CellKind::Number],
        ),
        selected(source, vec![0], vec![numeric], vec![CellKind::Flag]),
    ] {
        let rows = Rows::Selected(view);
        let publication = owner.source(relation, &rows).unwrap();
        assert!(matches!(
            owner.inspect(publication.as_ref(), BorrowedRows::from(&rows)),
            Err(OperatingError::ShapeMismatch)
        ));
    }
    let rows = Rows::Selected(selected(
        source,
        vec![9],
        vec![numeric],
        vec![CellKind::Number],
    ));
    let publication = owner.source(relation, &rows).unwrap();
    assert_eq!(
        owner.inspect(publication.as_ref(), BorrowedRows::from(&rows)),
        Ok(PhysicalShape::ReadablePhysicalFallback)
    );
}

#[derive(Debug)]
struct Forged;
impl PublishedRelation for Forged {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn kind(&self) -> RepresentationKind {
        RepresentationKind::Stored
    }
}
#[test]
fn a_foreign_publication_with_correct_kind_has_no_admission_authority() {
    let owner = RelationOwner::new(true);
    let physical = columns(&[CellKind::Number]);
    let rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number],
        columns: &physical,
    });
    assert!(matches!(
        owner.inspect(&Forged, BorrowedRows::from(&rows)),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        owner.admit(&Forged, BorrowedRows::from(&rows)),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        owner.resolve(&Forged, &|_| panic!(
            "forged publication must not acquire a provider"
        )),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(owner.checks.get(), 0);
}

#[test]
fn stable_partial_population_reuses_structure_without_proving_semantic_absence() {
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
        at: Some(FileId(9)),
    }];
    let owner = RelationOwner::new(true);
    let first = owner.source(relation, &rows).unwrap();
    let second = owner.source(relation, &rows).unwrap();
    assert!(first.same_record(&second));
    for publication in [&first, &second] {
        assert_eq!(
            owner.inspect(publication.as_ref(), BorrowedRows::from(&rows)),
            Ok(PhysicalShape::Complete)
        );
    }
    assert_eq!(owner.checks.get(), 1);
    assert!(complete_for_reuse(relation, &rows, &[], true));
    assert!(!complete_for_reuse(relation, &rows, &partials, true));
    owner.derived.prepare_file(
        true,
        &[relation],
        &partials,
        &[relation],
        &[],
        &crate::domain::operator::types::PreparedPool::default(),
        |_| true,
    );
    assert!(
        !owner.derived.relation_eligible(relation),
        "physical completeness cannot erase a partial relation"
    );
}

#[test]
fn normalization_reuses_one_owned_fallback_without_promoting_missing_rows() {
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    physical[1].0.pop();
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let owner = RelationOwner::new(true);
    let publication = owner
        .source(Relation::Function, &Rows::Stored(source))
        .unwrap();
    let first = owner.normalize(&publication, Rows::Stored(source)).unwrap();
    let second = owner.normalize(&publication, Rows::Stored(source)).unwrap();
    let (Rows::Computed(first_table), Rows::Computed(second_table)) = (&first, &second) else {
        panic!("ragged input needs an owned normalized population")
    };
    assert!(std::ptr::eq(*first_table, *second_table));
    assert_eq!(read(&first), [row(schema, 0)]);
    assert_eq!(read(&second), read(&first));
    assert_eq!(owner.checks.get(), 1);
}

mod reuse;

mod normalization_retention;
