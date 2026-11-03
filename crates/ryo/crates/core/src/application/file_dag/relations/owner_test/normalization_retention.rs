//! Optional publication admission cannot hide later normalization growth.
use super::*;

#[test]
fn lazy_source_refusal_preserves_required_normalization_and_charges_finalized_retention() {
    let owner = RelationOwner::new(true);
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let mut physical = columns(schema);
    physical[1].0.pop();
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let publication = owner.source(relation, &Rows::Stored(source)).unwrap();
    let weak = Arc::downgrade(publication.record());
    let limit = 6 * 1024 * 1024;
    assert!(
        owner
            .retain_publication(&publication, limit)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        owner.checks.get(),
        0,
        "retention never forces physical inspection"
    );
    assert!(
        publication.normalized.get().is_none(),
        "retention never materializes a lazy source"
    );
    assert_eq!(owner.derived.publication_ownership_bytes().1, 0);

    let normalized_address;
    {
        let first = owner.normalize(&publication, Rows::Stored(source)).unwrap();
        let Rows::Computed(table) = first else {
            panic!("readable physical fallback must normalize")
        };
        normalized_address = table as *const _;
        assert_eq!(read(&Rows::Computed(table)), [row(schema, 0)]);
        assert!(owner.derived.publication_ownership_bytes().0 > 0);
    }
    assert!(owner.retain_publication(&publication, 0).unwrap().is_none());
    let again = owner.normalize(&publication, Rows::Stored(source)).unwrap();
    let Rows::Computed(table) = again else {
        panic!("same demanded fallback")
    };
    assert_eq!(normalized_address, table as *const _);
    assert_eq!(
        owner.checks.get(),
        1,
        "required consumers reuse the completed normalization"
    );
    assert_eq!(owner.derived.publication_ownership_bytes().1, 0);
    let retained = owner
        .retain_publication(&publication, limit)
        .unwrap()
        .unwrap();
    let (_, optional, unique) = owner.derived.publication_ownership_bytes();
    assert!(optional > 0);
    assert_eq!(
        optional, unique,
        "required and optional aliases charge the same physical buffers once"
    );
    drop(publication);
    let (required, still_optional, unique) = owner.derived.publication_ownership_bytes();
    assert_eq!(required, 0);
    assert_eq!(still_optional, optional);
    assert_eq!(unique, optional);
    assert!(weak.upgrade().is_some());
    drop(retained);
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.derived.publication_ownership_bytes(), (0, 0, 0));
}
