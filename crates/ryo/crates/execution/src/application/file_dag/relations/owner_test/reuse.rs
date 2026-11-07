//! Reacquisition does not turn every consumer into a structural scan.
use super::*;
#[test]
fn many_selected_consumers_reacquire_one_generation_proof() {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        (std::sync::Arc::new(
            crate::application::file_dag::testing::derived(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
    let relation = Relation::FilesystemLineCount;
    let schema = kinds(relation);
    let physical = columns(schema);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let positions = Arc::new(vec![1, 0, 1]);
    let selection = Arc::new(StoredSelection {
        relation,
        rows: positions,
        columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        kinds: vec![schema[1], schema[0], schema[1]],
        identity: Arc::new(Default::default()),
    });
    let rows = Rows::Selected(SelectedRows { source, selection });
    let first = owner.source(relation, &rows).unwrap();
    for _ in 0..128 {
        let reacquired = owner.source(relation, &rows).unwrap();
        assert_eq!(first.same_record(&reacquired), crate::application::file_dag::relations::types::Sameness::Same);
        let resolved = owner
            .resolve(reacquired.as_ref(), &|_| Ok(rows.clone()))
            .unwrap();
        assert_eq!(
            owner.inspect(reacquired.as_ref(), BorrowedRows::from(&resolved)),
            Ok(PhysicalShape::Complete)
        );
        assert_eq!(
            read(&resolved),
            vec![
                vec![row(schema, 1)[1], row(schema, 1)[0], row(schema, 1)[1]],
                vec![row(schema, 0)[1], row(schema, 0)[0], row(schema, 0)[1]],
                vec![row(schema, 1)[1], row(schema, 1)[0], row(schema, 1)[1]]
            ]
        );
    }
    assert_eq!(owner.checks.get(), crate::application::file_dag::relations::types::CheckCount(1));
    assert_eq!(owner.reuses.get(), crate::application::file_dag::relations::types::CheckCount(127));
    owner.release_population();
    assert!(matches!(
        owner.resolve(first.as_ref(), &|_| Ok(rows.clone())),
        Err(OperatingError::NotRead)
    ));
    let next = owner.source(relation, &rows).unwrap();
    assert_eq!(first.same_record(&next), crate::application::file_dag::relations::types::Sameness::Different);
    owner
        .inspect(next.as_ref(), BorrowedRows::from(&rows))
        .unwrap();
    assert_eq!(owner.checks.get(), crate::application::file_dag::relations::types::CheckCount(2));
}
