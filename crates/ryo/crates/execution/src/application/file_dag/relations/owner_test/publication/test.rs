//! Required publication aliases share normalization and release its backing at the last consumer.
use super::super::*;

#[test]
fn lazy_source_normalization_is_shared_and_lives_until_the_last_consumer() {
    let owner = RelationOwner::new(
        true,
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    physical[1].0.pop();
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let publication = owner.source(relation, &Rows::Stored(source)).unwrap();
    let consumer = publication.clone();
    let weak = Arc::downgrade(publication.record());
    assert_eq!(
        owner.checks.get(),
        0,
        "publication does not inspect undemanded source rows"
    );
    assert!(publication.normalized.get().is_none());
    assert_eq!(owner.derived.publication_ownership_bytes().1, 0);
    let normalized_address;
    {
        let Rows::Computed(table) = owner.normalize(&publication, Rows::Stored(source)).unwrap()
        else {
            panic!("readable physical fallback must normalize")
        };
        normalized_address = table as *const _;
        assert_eq!(read(&Rows::Computed(table)), [row(schema, 0)]);
    }
    let charged = owner.derived.publication_ownership_bytes();
    assert!(charged.0 > 0);
    assert_eq!(charged.1, 0);
    assert_eq!(charged.0, charged.2);
    drop(publication);
    assert!(weak.upgrade().is_some());
    let Rows::Computed(table) = owner.normalize(&consumer, Rows::Stored(source)).unwrap() else {
        panic!("same demanded fallback")
    };
    assert_eq!(normalized_address, table as *const _);
    assert_eq!(owner.checks.get(), 1);
    assert_eq!(
        owner.derived.publication_ownership_bytes(),
        charged,
        "alias does not charge normalization again"
    );
    drop(consumer);
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.derived.publication_ownership_bytes(), (0, 0, 0));
}

#[test]
fn computed_publication_clones_keep_required_backing_until_last_handle_in_live_generation() {
    use ryo_values::domain::rows::types::{RowCount, Table};
    let owner = RelationOwner::new(
        true,
        crate::application::file_dag::testing::program_derived_port(),
    );
    let generation = owner.generation_guard();
    let publication = owner
        .computed(Table {
            kinds: vec![CellKind::Number],
            cells: vec![Value::Number(Integer(41)), Value::Number(Integer(43))],
            count: RowCount(2),
        })
        .unwrap();
    let record = Arc::downgrade(publication.record());
    let alias = publication.clone();
    assert!(alias.same_record(&publication));
    let charged = owner.derived.publication_ownership_bytes();
    assert!(
        charged.0 > 0,
        "actual computed publication has required backing"
    );
    assert_eq!(charged.1, 0, "publication is not optional retention");
    assert_eq!(charged.0, charged.2, "one physical required backing charge");
    drop(publication);
    assert!(record.upgrade().is_some());
    assert_eq!(owner.derived.publication_ownership_bytes(), charged);
    {
        let rows = owner
            .resolve(alias.as_ref(), &|_| Err(OperatingError::NotRead))
            .unwrap();
        assert_eq!(
            read(&rows),
            [
                vec![Value::Number(Integer(41))],
                vec![Value::Number(Integer(43))]
            ]
        );
        assert_eq!(owner.derived.publication_ownership_bytes(), charged);
    }
    assert!(owner.matches_generation(&generation));
    assert!(owner.generation_liveness().is_live());
    drop(alias);
    assert!(
        record.upgrade().is_none(),
        "last leased handle releases the actual record"
    );
    assert_eq!(owner.derived.publication_ownership_bytes(), (0, 0, 0));
    assert!(
        owner.matches_generation(&generation),
        "release did not rely on retiring the generation"
    );
    assert!(owner.generation_liveness().is_live());
}
