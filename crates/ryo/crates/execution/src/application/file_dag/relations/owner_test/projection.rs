//! Projections, positions and forged metadata never gain authority they were not issued.
use super::*;
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
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(
            crate::application::file_dag::testing::derived(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
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
    assert_eq!(publications[0].same_record(&publications[1]), crate::application::file_dag::relations::types::Sameness::Different);
    assert_eq!(publications[0].same_record(&publications[2]), crate::application::file_dag::relations::types::Sameness::Different);
    assert!(matches!(
        owner.inspect(
            publications[0].as_ref(),
            BorrowedRows::from(&Rows::Selected(second))
        ),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(owner.checks.get(), crate::application::file_dag::relations::types::CheckCount(3));
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
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(
            crate::application::file_dag::testing::derived(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
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
}
#[test]
fn a_foreign_publication_with_correct_kind_has_no_admission_authority() {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(
            crate::application::file_dag::testing::derived(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
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
    assert_eq!(owner.checks.get(), crate::application::file_dag::relations::types::CheckCount(0));
}
